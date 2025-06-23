use crate::state::AppState;
use actix_web::{Error, HttpResponse, error::InternalError, get, http::StatusCode, web};
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct BuyerQuery {
    group_id: Option<i64>,
}

#[get("/buyers")]
pub async fn get_buyers(
    query: web::Query<BuyerQuery>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let buyers_result = match query.group_id {
        Some(group_id) => app_state.db.get_buyers_by_group(group_id).await,
        None => app_state.db.get_all_buyers().await,
    };

    let buyers = buyers_result.map_err(|e| {
        log::error!("Failed to fetch buyers: {}", e);
        InternalError::new(
            "Failed to get buyers. Please try again later.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    Ok(HttpResponse::Ok().json(buyers))
}

#[get("/buyers/{wallet}")]
pub async fn get_buyer_by_wallet(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let wallet = path.into_inner();

    let buyer = app_state
        .db
        .get_buyer_by_wallet(&wallet)
        .await
        .map_err(|e| {
            log::error!("Failed to get buyer by wallet {}: {}", wallet, e);
            InternalError::new(
                "Buyer with provided wallet not found.",
                StatusCode::NOT_FOUND,
            )
        })?;

    Ok(HttpResponse::Ok().json(buyer))
}
