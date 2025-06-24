use crate::state::AppState;
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

pub async fn initialize_schedules(data: web::Data<AppState>) -> anyhow::Result<()> {
    let groups = data.db.get_all_groups().await?;

    for group in groups.into_iter() {
        log::info!("Distributing tokens for group: {}", group.id);
        let buyers = data.db.get_buyers_by_group(group.id).await?;
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
            let existing_schedules = data
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
                let interval_amount = (buyer_spl as f64 * percent).round() as u64;
                current_percent += percent;
                let percent_key = (current_percent * 1_000_000.0).round() as u64;

                if !existing_percents.contains(&percent_key) {
                    unlocks.push((
                        unlock_time,
                        interval_amount.min(remaining_spl_lamports),
                        current_percent,
                    ));
                }
                remaining_spl_lamports -= interval_amount.min(remaining_spl_lamports);
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
                if let Err(e) = data.db.save_schedule(&schedule).await {
                    log::error!("Failed to save schedule for {}: {}", buyer.wallet, e);
                }
            }
        }
    }
    log::info!("Schedules created successfully");

    Ok(())
}

pub async fn start_schedule_runner(data: web::Data<AppState>) {
    let handle = tokio::spawn(async move {
        loop {
            let now = Utc::now().naive_utc();
            match data.db.get_schedules_due(now).await {
                Ok(schedules) => {
                    for schedule in schedules {
                        log::info!(
                            "Schedule ready: id={:?} buyer={} group={} amount_lamports={} scheduled_at={}",
                            schedule.id,
                            schedule.buyer_wallet,
                            schedule.group_id,
                            schedule.amount_lamports,
                            schedule.scheduled_at
                        );
                        //TODO: get decimals from config
                        let _ = process_schedule(&data, &schedule, 9).await;
                    }
                }
                Err(e) => {
                    log::error!("Error fetching due schedules: {}", e);
                    std::process::exit(1);
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    });

    if let Err(e) = handle.await {
        log::error!("Schedule runner task stopped unexpectedly: {:?}", e);
        std::process::exit(1);
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
    data: &AppState,
    schedule: &Schedule,
    token_decimals: u8,
) -> anyhow::Result<Schedule> {
    // Try to get group and buyer info
    let group = match data.db.get_group(schedule.group_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            let error_message = format!("Group not found for schedule id={}", schedule.id);
            log::error!("{}", error_message);
            return data
                .db
                .update_schedule_status(schedule.id, "failed", Some(error_message))
                .await;
        }
        Err(e) => {
            let error_message =
                format!("Failed to get group for schedule id={}: {}", schedule.id, e);
            log::error!("{}", error_message);
            return data
                .db
                .update_schedule_status(schedule.id, "failed", Some(error_message))
                .await;
        }
    };
    let buyer = match data.db.get_buyer_by_wallet(&schedule.buyer_wallet).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            let error_message = format!("Buyer not found for schedule id={}", schedule.id);
            log::error!("{}", error_message);
            return data
                .db
                .update_schedule_status(schedule.id, "failed", Some(error_message))
                .await;
        }
        Err(e) => {
            let error_message =
                format!("Failed to get buyer for schedule id={}: {}", schedule.id, e);
            log::error!("{}", error_message);
            return data
                .db
                .update_schedule_status(schedule.id, "failed", Some(error_message))
                .await;
        }
    };

    let mut transaction = Transaction::new(
        schedule.buyer_wallet.clone(),
        schedule.group_id,
        schedule.amount_lamports,
        schedule.percent,
        "success".to_string(),
    );

    // Try to transfer tokens
    match transfer_tokens_for_schedule(data, schedule, &buyer, token_decimals).await {
        Ok(_) => {
            // Save successful transaction
            transaction.sent_at = Some(Utc::now().naive_utc());
            if let Err(e) = data.db.save_transaction(transaction).await {
                let error_message = format!(
                    "Failed to save transaction for schedule id={:?}: {}",
                    schedule.id, e
                );
                log::error!("{}", error_message);
            }

            // Update buyer's received_spl, received_percent, pending_spl
            let buyer_spl_lamports = buyer.paid_lamports / group.spl_price_lamports;
            let received_spl_lamports = buyer.received_spl_lamports + schedule.amount_lamports;
            let pending_spl = buyer_spl_lamports - received_spl_lamports;
            let received_percent = schedule.percent;

            if let Err(e) = data
                .db
                .update_buyer(
                    &buyer.wallet.to_string(),
                    received_spl_lamports,
                    received_percent,
                    pending_spl,
                )
                .await
            {
                //TODO: Improve error handling to try again update buyer. If no save errors for admin in another storage
                let error_message = format!(
                    "Failed to update buyer after transfer for schedule id={:?}: {}",
                    schedule.id, e
                );
                log::error!("{}", error_message);
                return data
                    .db
                    .update_schedule_status(schedule.id, "failed", Some(error_message))
                    .await;
            }

            // Mark schedule as success
            data.db
                .update_schedule_status(schedule.id, "success", None)
                .await
        }
        Err(e) => {
            let error_message = format!(
                "Failed to transfer tokens for schedule id={:?} buyer={} group={} amount_lamports={}: {}",
                schedule.id, schedule.buyer_wallet, schedule.group_id, schedule.amount_lamports, e
            );
            log::error!("{}", error_message);

            // Update transaction for failure
            transaction.status = "failed".to_string();
            transaction.error_message = Some(error_message.clone());
            transaction.sent_at = Some(chrono::Utc::now().naive_utc());

            // Save failed transaction
            if let Err(e) = data.db.save_transaction(transaction).await {
                log::error!(
                    "Failed to save failed transaction for schedule id={:?}: {}",
                    schedule.id,
                    e
                );
            }
            data.db
                .update_schedule_status(schedule.id, "failed", Some(error_message))
                .await
        }
    }
}
