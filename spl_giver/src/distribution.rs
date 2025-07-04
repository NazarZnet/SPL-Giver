use crate::state::{AppState, PendingOp};
use actix_web::web;

use chrono::Utc;
use common::SplToken;
use common::{Buyer, Schedule, Transaction};
use solana_sdk::pubkey::Pubkey;
use tokio::time::{Duration, sleep};

pub async fn check_group_token_funding(data: &AppState) -> anyhow::Result<()> {
    let groups = data.db.get_all_groups().await?;
    for group in &groups {
        let buyers = data.db.get_buyers_by_group(group.id).await?;
        let total_pending = buyers.iter().map(|b| b.pending_spl_lamports).sum();
        log::info!(
            "Group total lamports: {} total pending lamports: {}",
            group.spl_total_lamports,
            total_pending
        );
        if group.spl_total_lamports < total_pending {
            return Err(anyhow::anyhow!(
                "Group {} does not have enough SPL tokens: group.spl_total_lamports = {}, total pending_spl_lamports for buyers = {}",
                group.id,
                group.spl_total_lamports,
                total_pending
            ));
        }
    }
    Ok(())
}

pub async fn initialize_schedules(app_state: &AppState) -> anyhow::Result<()> {
    let groups = app_state.db.get_all_groups().await?;

    for group in groups.into_iter() {
        log::info!("Distributing tokens for group: {}", group.id);
        let buyers = app_state.db.get_buyers_by_group(group.id).await?;
        for buyer in &buyers {
            let buyer_spl = buyer.paid_lamports / group.spl_price_lamports;
            let already_received_lamports = buyer.received_spl_lamports;

            let mut remaining_percent = 1.0 - buyer.received_percent;
            let mut current_percent = buyer.received_percent;
            let mut remaining_spl_lamports = buyer_spl - already_received_lamports;

            if remaining_spl_lamports == 0 || remaining_percent <= 0.0 {
                log::info!(
                    "Buyer {} already received all tokens: received_spl_lamports {}, paid_lamports {}, spl_price_lamports {}",
                    buyer.wallet,
                    buyer.received_spl_lamports,
                    buyer.paid_lamports,
                    group.spl_price_lamports
                );
                continue;
            }

            // Get existing schedule percents for this buyer
            let existing_schedules = app_state
                .db
                .get_schedules_by_buyer_and_group(&buyer.wallet.to_string(), group.id)
                .await?;
            // 1_000_000.0 to avoid floating point issues
            let existing_percents: std::collections::HashSet<u64> = existing_schedules
                .iter()
                .map(|s| (s.percent * 1_000_000.0).round() as u64)
                .collect();

            let mut unlock_time = Utc::now().naive_utc();
            let mut unlocks = vec![];

            // If buyer hasn't received anything, schedule initial unlock first
            if already_received_lamports == 0 {
                let percent = group.initial_unlock_percent.min(remaining_percent);
                let initial_amount = (buyer_spl as f64 * percent).round() as u64;
                current_percent += percent;
                let percent_key = (current_percent * 1_000_000.0).round() as u64;
                if !existing_percents.contains(&percent_key) {
                    unlocks.push((unlock_time, initial_amount, current_percent));
                }
                remaining_spl_lamports -= initial_amount;
                remaining_percent -= percent;
            }

            // Schedule future unlocks for the rest
            while remaining_spl_lamports > 0 && remaining_percent > 0.0 {
                unlock_time += chrono::Duration::seconds(group.unlock_interval_seconds);
                let percent = group.unlock_percent_per_interval.min(remaining_percent);

                //If this is the last unlock, adjust the amount to not exceed remaining SPL
                let is_last = remaining_percent <= group.unlock_percent_per_interval
                    || remaining_spl_lamports <= ((buyer_spl as f64 * percent).round() as u64);

                let interval_amount = if is_last {
                    remaining_spl_lamports
                } else {
                    (buyer_spl as f64 * percent).round() as u64
                };

                current_percent += percent;
                let percent_key = (current_percent * 1_000_000.0).round() as u64;

                if !existing_percents.contains(&percent_key) {
                    unlocks.push((unlock_time, interval_amount, current_percent));
                }
                remaining_spl_lamports = remaining_spl_lamports.saturating_sub(interval_amount);
                remaining_percent -= percent;
            }

            for (scheduled_at, amount_lamports, percent) in unlocks {
                let schedule = Schedule::new(
                    group.id,
                    buyer.wallet.to_string(),
                    scheduled_at,
                    amount_lamports,
                    percent,
                );

                // Save schedule entry to DB
                if let Err(e) = app_state.db.save_schedule(&schedule).await {
                    log::error!("Failed to save schedule for {}: {}", buyer.wallet, e);
                }
            }
        }
    }
    log::info!("Schedules created successfully");

    Ok(())
}

pub async fn start_schedule_runner(app_state: web::Data<AppState>) -> anyhow::Result<()> {
    loop {
        let now = Utc::now().naive_utc();
        let schedules = app_state.db.get_schedules_due(now).await?;
        for schedule in schedules {
            log::info!(
                "Schedule ready: id={:?} buyer={} group={} amount_lamports={} scheduled_at={}",
                schedule.id,
                schedule.buyer_wallet,
                schedule.group_id,
                schedule.amount_lamports,
                schedule.scheduled_at
            );

            if let Err(e) =
                process_schedule(&app_state, &schedule, app_state.spl_token.decimals).await
            {
                log::error!("Failed to process schedule id={}: {:#}", schedule.id, e);
            }
        }
        sleep(Duration::from_secs(60)).await;
    }
}
pub async fn transfer_tokens_for_schedule(
    data: &AppState,
    schedule: &Schedule,
    buyer: &Buyer,
    token_decimals: u8,
) -> anyhow::Result<()> {
    // Get or create ATA
    let ata = SplToken::get_or_create_associated_token_account(
        &data.spl_token.client,
        &buyer.wallet,
        &data.spl_token.main_wallet,
        &data.spl_token.mint,
    )
    .await
    .map_err(|e| anyhow::anyhow!("ATA error: {}", e))?;

    // Transfer with retries

    if let Err(e) = try_transfer_with_retries(
        &data.spl_token,
        &ata,
        schedule.amount_lamports,
        token_decimals,
        &buyer.wallet.to_string(),
    )
    .await
    {
        return Err(anyhow::anyhow!("Transfer error: {}", e));
    }

    log::info!(
        "Transferred {} token lamports to {} for schedule id={:?}",
        schedule.amount_lamports,
        buyer.wallet,
        schedule.id,
    );

    Ok(())
}
pub async fn try_transfer_with_retries(
    spl_token_context: &SplToken,
    ata: &Pubkey,
    to_unlock: u64,
    token_decimals: u8,
    buyer_wallet: &str,
) -> Result<(), String> {
    let mut attempt = 0;
    let mut last_err = None;
    while attempt < 4 {
        match spl_token_context
            .transfer_tokens(ata, to_unlock, token_decimals)
            .await
        {
            Ok(_) => {
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e.to_string());
                log::warn!(
                    "Send error for {} (attempt {}/{}): {}",
                    buyer_wallet,
                    attempt + 1,
                    4,
                    last_err.as_ref().unwrap()
                );
                attempt += 1;
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "Unknown transfer error".to_string()))
}
pub async fn process_schedule(
    app_state: &AppState,
    schedule: &Schedule,
    token_decimals: u8,
) -> anyhow::Result<Schedule> {
    //Flush any pending DB operations from previous runs
    let retry_queue = &app_state.retry_queue;
    if let Err(e) = retry_queue.flush(&app_state.db).await {
        log::error!("Found pending DB operations. Failed save them to DB: {e}");
    }

    //Load Group
    let group = match app_state.db.get_group(schedule.group_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            let err_msg = format!("Group not found for schedule id={}", schedule.id);
            log::error!("{}", err_msg);
            return app_state
                .db
                .update_schedule_status(schedule.id, "failed", Some(err_msg))
                .await;
        }
        Err(e) => {
            let err_msg = format!(
                "Database error retrieving group for schedule id={}: {}",
                schedule.id, e
            );
            log::error!("{}", err_msg);
            return app_state
                .db
                .update_schedule_status(schedule.id, "failed", Some(err_msg))
                .await;
        }
    };

    //Load Buyer
    let buyer = match app_state
        .db
        .get_buyer_by_wallet(&schedule.buyer_wallet)
        .await
    {
        Ok(Some(b)) => b,
        Ok(None) => {
            let err_msg = format!("Buyer not found for schedule id={}", schedule.id);
            log::error!("{}", err_msg);
            return app_state
                .db
                .update_schedule_status(schedule.id, "failed", Some(err_msg))
                .await;
        }
        Err(e) => {
            let err_msg = format!(
                "Database error retrieving buyer for schedule id={}: {}",
                schedule.id, e
            );
            log::error!("{}", err_msg);
            return app_state
                .db
                .update_schedule_status(schedule.id, "failed", Some(err_msg))
                .await;
        }
    };

    //Prepare transaction record
    let mut tx_record = Transaction::new(
        schedule.buyer_wallet.clone(),
        schedule.group_id,
        schedule.amount_lamports,
        schedule.percent,
        "success".to_string(),
    );

    //Attempt token transfer
    match transfer_tokens_for_schedule(app_state, schedule, &buyer, token_decimals).await {
        Ok(_) => {
            log::info!("Tokens transferred for schedule id={}", schedule.id);

            //Save transaction
            tx_record.sent_at = Some(Utc::now().naive_utc());
            if let Err(e) = app_state.db.save_transaction(tx_record.clone()).await {
                log::error!(
                    "Failed to save transaction for schedule id={}: {}",
                    schedule.id,
                    e
                );
                if let Err(e) = retry_queue
                    .push_and_persist(PendingOp::SaveTransaction(tx_record.clone()))
                    .await
                {
                    log::error!("Failed to enqueue SaveTransaction: {}", e);
                }
            }

            //Update buyer balances
            let total_spl = buyer.paid_lamports / group.spl_price_lamports;
            let new_received_spl = buyer.received_spl_lamports + schedule.amount_lamports;
            let new_pending_spl = total_spl - new_received_spl;
            let percent = schedule.percent;

            if let Err(e) = app_state
                .db
                .update_buyer(
                    &buyer.wallet.to_string(),
                    new_received_spl,
                    percent,
                    new_pending_spl,
                )
                .await
            {
                log::error!(
                    "Failed to update buyer after transfer for schedule id={}: {}",
                    schedule.id,
                    e
                );
                if let Err(e) = retry_queue
                    .push_and_persist(PendingOp::UpdateBuyer {
                        wallet: buyer.wallet.to_string(),
                        received_spl: new_received_spl,
                        received_percent: percent,
                        pending_spl: new_pending_spl,
                    })
                    .await
                {
                    log::error!("Failed to enqueue UpdateBuyer: {}", e);
                }
            }

            //Mark schedule as success
            match app_state
                .db
                .update_schedule_status(schedule.id, "success", None)
                .await
            {
                Ok(updated) => {
                    log::info!("Schedule id={} marked success", schedule.id);
                    Ok(updated)
                }
                Err(e) => {
                    if let Err(e) = retry_queue
                        .push_and_persist(PendingOp::UpdateSchedule {
                            schedule_id: schedule.id,
                            status: "success".into(),
                            error_message: None,
                        })
                        .await
                    {
                        log::error!("Failed to enqueue UpdateSchedule: {}", e);
                    }
                    anyhow::bail!(
                        "Failed to update schedule status to success for id={}: {}",
                        schedule.id,
                        e
                    )
                }
            }
        }
        Err(e) => {
            let err_msg = format!(
                "Token transfer failed for schedule id={} buyer={} group={} amount={}: {}",
                schedule.id, schedule.buyer_wallet, schedule.group_id, schedule.amount_lamports, e
            );
            log::error!("{}", err_msg);

            //Record failed transaction
            tx_record.status = "failed".to_string();
            tx_record.error_message = Some(err_msg.clone());
            tx_record.sent_at = Some(Utc::now().naive_utc());

            if let Err(e) = app_state.db.save_transaction(tx_record.clone()).await {
                log::error!(
                    "Failed to save failed transaction for schedule id={}: {}",
                    schedule.id,
                    e
                );
                if let Err(e) = retry_queue
                    .push_and_persist(PendingOp::SaveTransaction(tx_record))
                    .await
                {
                    log::error!("Failed to enqueue SaveTransaction: {}", e);
                }
            }

            //Mark schedule as failed
            match app_state
                .db
                .update_schedule_status(schedule.id, "failed", Some(err_msg.clone()))
                .await
            {
                Ok(updated) => {
                    log::info!("Schedule id={} marked failed", schedule.id);
                    Ok(updated)
                }
                Err(e) => {
                    if let Err(e) = retry_queue
                        .push_and_persist(PendingOp::UpdateSchedule {
                            schedule_id: schedule.id,
                            status: "failed".into(),
                            error_message: Some(err_msg),
                        })
                        .await
                    {
                        log::error!("Failed to enqueue UpdateSchedule: {}", e);
                    }
                    anyhow::bail!(
                        "Failed to update schedule status to failed for id={}: {}",
                        schedule.id,
                        e
                    )
                }
            }
        }
    }
}
