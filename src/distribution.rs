use crate::{
    schema::Group,
    state::{AppState, SplTokenContext},
};
use actix_web::web;
use tokio::time::{Duration, sleep};

pub async fn distribute_tokens(data: web::Data<AppState>) -> anyhow::Result<()> {
    let mut errors = vec![];
    let token_decimals = 9u8;
    let groups = data.db.get_groups().await?;

    for group in groups.into_iter() {
        log::info!("Distributing tokens for group: {}", group.id);
        let buyers = data.db.get_buyers_by_group(group.id).await?;
        for buyer in &buyers {
            // Check if buyer has already received tokens
            let buyer_spl = buyer.paid_sol / group.spl_price;
            if buyer.received_spl >= buyer_spl {
                log::info!(
                    "Buyer {} already received all tokens: received_spl {}, paid_sol {}, spl_price {}",
                    buyer.wallet,
                    buyer.received_spl,
                    buyer.paid_sol,
                    group.spl_price
                );
                continue;
            }
            let persents = if buyer.received_spl > 0.0 {
                group.unlock_percent_per_interval
            } else {
                group.initial_unlock_percent
            };
            let initial_spl = buyer_spl * persents;
            let initial_spl_lamports =
                (initial_spl * 10f64.powi(token_decimals as i32)).round() as u64;

            log::info!(
                "Buyer {}: paid_sol {}, group.spl_price {}, initial_spl {} (lamports: {})",
                buyer.wallet,
                buyer.paid_sol,
                group.spl_price,
                initial_spl,
                initial_spl_lamports
            );
            //TODO: make this logic as a normal method not like static method
            let ata = match SplTokenContext::get_or_create_associated_token_account(
                &data.spl_token_context.client,
                &buyer.wallet,
                &data.spl_token_context.main_wallet,
                &data.spl_token_context.mint,
            )
            .await
            {
                Ok(ata) => ata,
                Err(e) => {
                    errors.push(format!("ATA error for {}: {}", buyer.wallet, e));
                    log::error!("ATA error for {}: {}", buyer.wallet, e);
                    continue;
                }
            };
            //TODO: rewrite this logic as a function
            let mut attempt = 0;
            let max_attempts = 4;
            let mut last_err = None;
            while attempt < max_attempts {
                match data
                    .spl_token_context
                    .transfer_tokens(&ata, initial_spl_lamports, token_decimals)
                    .await
                {
                    Ok(_) => {
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e.to_string());
                        log::warn!(
                            "Send error for {} (attempt {}/{}): {}",
                            buyer.wallet,
                            attempt + 1,
                            max_attempts,
                            last_err.as_ref().unwrap()
                        );
                        attempt += 1;
                        sleep(Duration::from_secs(2)).await; // wait 2 seconds before retry
                    }
                }
            }
            if let Some(e) = last_err {
                errors.push(format!(
                    "Send error for {} after {} attempts: {}",
                    buyer.wallet, max_attempts, e
                ));
                log::error!(
                    "Send error for {} after {} attempts: {}",
                    buyer.wallet,
                    max_attempts,
                    e
                );
                continue;
            }
            log::info!(
                "Minted {} tokens (lamports: {}) to {}",
                initial_spl,
                initial_spl_lamports,
                ata
            );
            //save buyer modifications
            let received_spl = buyer.received_spl + initial_spl;
            let pending_spl = if buyer.pending_spl > 0.0 {
                buyer.pending_spl - initial_spl
            } else {
                buyer_spl
            };
            match data
                .db
                .update_buyer(buyer.wallet.to_string().as_str(), received_spl, pending_spl)
                .await
            {
                Ok(buyer) => {
                    log::info!(
                        "Buyer {} updated: received_spl {}, pending_spl {}",
                        buyer.wallet,
                        buyer.received_spl,
                        buyer.pending_spl
                    );
                }
                Err(e) => {
                    errors.push(format!("Failed to update buyer {}: {}", buyer.wallet, e));
                    log::error!("Failed to update buyer {}: {}", buyer.wallet, e);
                }
            }
        }
        spawn_group_unlock_scheduler(group, data.clone(), token_decimals).await;
    }

    Ok(())
}

pub async fn spawn_group_unlock_scheduler(
    group: Group,
    data: web::Data<AppState>,
    token_decimals: u8,
) {
    if group.unlock_task_spawned {
        log::warn!("Unlock scheduler already spawned for group {}", group.id);
        return;
    }
    let _handle = tokio::spawn(async move {
        let mut unlock_percent = group.initial_unlock_percent;
        let total_unlocks = ((1.0 - group.initial_unlock_percent)
            / group.unlock_percent_per_interval)
            .ceil() as usize;

        for unlock_idx in 0..total_unlocks {
            sleep(Duration::from_secs(group.unlock_interval_seconds as u64)).await;
            unlock_percent += group.unlock_percent_per_interval;
            let buyers = data
                .db
                .get_buyers_by_group(group.id)
                .await
                .unwrap_or_else(|e| {
                    log::error!("Failed to get buyers for group {}: {}", group.id, e);
                    vec![]
                });
            for buyer in &buyers {
                let buyer_spl = buyer.paid_sol / group.spl_price;
                if buyer.received_spl >= buyer_spl {
                    log::info!(
                        "Buyer {} already received all tokens: received_spl {}, paid_sol {}, spl_price {}",
                        buyer.wallet,
                        buyer.received_spl,
                        buyer.paid_sol,
                        group.spl_price
                    );
                    continue;
                }
                let initial_spl = buyer_spl * group.unlock_percent_per_interval;
                let to_unlock = (initial_spl * 10f64.powi(token_decimals as i32)).round() as u64;

                let ata = match SplTokenContext::get_or_create_associated_token_account(
                    &data.spl_token_context.client,
                    &buyer.wallet,
                    &data.spl_token_context.main_wallet,
                    &data.spl_token_context.mint,
                )
                .await
                {
                    Ok(ata) => ata,
                    Err(e) => {
                        //TODO: save error about transaction
                        log::error!("ATA error for {}: {}", buyer.wallet, e);
                        continue;
                    }
                };
                //TODO: rewrite this logic as a function
                let mut attempt = 0;
                let max_attempts = 4;
                let mut last_err = None;
                while attempt < max_attempts {
                    match data
                        .spl_token_context
                        .transfer_tokens(&ata, to_unlock, token_decimals)
                        .await
                    {
                        Ok(_) => {
                            last_err = None;
                            break;
                        }
                        Err(e) => {
                            last_err = Some(e.to_string());
                            log::warn!(
                                "Send error for {} (attempt {}/{}): {}",
                                buyer.wallet,
                                attempt + 1,
                                max_attempts,
                                last_err.as_ref().unwrap()
                            );
                            attempt += 1;
                            sleep(Duration::from_secs(2)).await; // wait 2 seconds before retry
                        }
                    }
                }
                if let Some(e) = last_err {
                    //TODO: save error about transaction
                    log::error!("Scheduled send error for {}: {}", buyer.wallet, e);
                    continue;
                }
                log::info!(
                    "Scheduled unlock {} for buyer {}: {} tokens. Unlocked {}% of tokens",
                    unlock_idx + 1,
                    buyer.wallet,
                    to_unlock,
                    unlock_percent * 100.0
                );
                //TODO: save buyer modifications
                let received_spl = buyer.received_spl + initial_spl;
                let pending_spl = if buyer.pending_spl > 0.0 {
                    buyer.pending_spl - initial_spl
                } else {
                    buyer_spl
                };
                match data
                    .db
                    .update_buyer(buyer.wallet.to_string().as_str(), received_spl, pending_spl)
                    .await
                {
                    Ok(buyer) => {
                        log::info!(
                            "Buyer {} updated: received_spl {}, pending_spl {}",
                            buyer.wallet,
                            buyer.received_spl,
                            buyer.pending_spl
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to update buyer {}: {}", buyer.wallet, e);
                    }
                }
            }
        }
    });

    log::info!("Unlock scheduler spawned for group {}", group.id);
}
