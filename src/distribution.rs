use std::sync::Arc;

use crate::state::{AppState, Group, SplTokenContext};
use actix_web::{HttpResponse, Responder, post, web};
use tokio::{
    sync::Mutex,
    time::{Duration, sleep},
};

#[post("/distribute")]
pub async fn distribute_tokens(data: web::Data<AppState>) -> impl Responder {
    let mut errors = vec![];
    let token_decimals = 9u8;

    for group_arc in &data.group_context.groups {
        let group = group_arc.lock().await;
        for buyer in &group.buyers {
            let buyer_spl = buyer.paid_sol / group.spl_price;
            let initial_spl = buyer_spl * group.initial_unlock_percent;
            let initial_spl_lamports =
                (initial_spl * 10f64.powi(token_decimals as i32)).round() as u64;

            log::debug!(
                "Buyer {}: paid_sol {}, group.spl_price {}, initial_spl {} (lamports: {})",
                buyer.wallet,
                buyer.paid_sol,
                group.spl_price,
                initial_spl,
                initial_spl_lamports
            );

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

            if let Err(e) = data
                .spl_token_context
                .transfer_tokens(&ata, initial_spl_lamports, token_decimals)
                .await
            {
                errors.push(format!("Send error for {}: {}", buyer.wallet, e));
                log::error!("Send error for {}: {}", buyer.wallet, e);
                continue;
            }
            log::info!(
                "Minted {} tokens (lamports: {}) to {}",
                initial_spl,
                initial_spl_lamports,
                ata
            );
        }
    }

    for group_arc in &data.group_context.groups {
        spawn_group_unlock_scheduler(Arc::clone(group_arc), data.clone(), token_decimals).await;
    }

    if errors.is_empty() {
        HttpResponse::Ok().body("Distribution finished successfully")
    } else {
        HttpResponse::Ok().body(format!("Distribution finished with errors: {:?}", errors))
    }
}

pub async fn spawn_group_unlock_scheduler(
    group: Arc<Mutex<Group>>,
    data: web::Data<AppState>,
    token_decimals: u8,
) {
    {
        let group = group.lock().await;
        if group.unlock_task_spawned {
            log::warn!("Unlock scheduler already spawned for group {}", group.id);
            return;
        }
    }

    let group_clone = Arc::clone(&group);
    let handle = tokio::spawn(async move {
        let group = group_clone.lock().await;
        let mut unlock_percent = group.initial_unlock_percent;
        let total_unlocks = ((1.0 - group.initial_unlock_percent)
            / group.unlock_percent_per_interval)
            .ceil() as usize;

        for unlock_idx in 0..total_unlocks {
            sleep(Duration::from_secs(group.unlock_interval_seconds)).await;
            unlock_percent += group.unlock_percent_per_interval;
            for buyer in &group.buyers {
                let buyer_spl = buyer.paid_sol / group.spl_price;
                let to_unlock = (buyer_spl
                    * group.unlock_percent_per_interval
                    * 10f64.powi(token_decimals as i32))
                .round() as u64;

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
                        log::error!("ATA error for {}: {}", buyer.wallet, e);
                        continue;
                    }
                };
                if let Err(e) = data
                    .spl_token_context
                    .transfer_tokens(&ata, to_unlock, token_decimals)
                    .await
                {
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
            }
        }
    });

    let mut group = group.lock().await;
    group.unlock_task = Some(Arc::new(Mutex::new(handle)));
    group.unlock_task_spawned = true;
    log::info!("Unlock scheduler spawned for group {}", group.id);
}
