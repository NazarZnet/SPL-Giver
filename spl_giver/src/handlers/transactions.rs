use actix_web::{Error, HttpResponse, error::InternalError, get, http::StatusCode, web};
use serde::Deserialize;

use crate::state::AppState;
#[derive(Debug, Deserialize)]
struct TransactionQuery {
    #[serde(default)]
    status: Option<String>,
}

#[get("/transactions")]
pub async fn get_transactions(
    query: web::Query<TransactionQuery>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    // Validate `status` if provided
    if let Some(ref status) = query.status {
        let valid_statuses = ["success", "failed"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(InternalError::new(
                "Transaction status must be either 'success' or 'failed'.",
                StatusCode::BAD_REQUEST,
            )
            .into());
        }
    }

    // Fetch transactions depending on status filter
    let transactions_result = match query.status.as_deref() {
        Some("success") | Some("failed") => {
            app_state
                .db
                .get_transactions_by_status(query.status.as_ref().unwrap())
                .await
        }
        None => app_state.db.get_all_transactions().await,
        _ => unreachable!(),
    };

    let transactions = transactions_result.map_err(|e| {
        log::error!("Failed to get transactions: {}", e);
        InternalError::new(
            "Failed to get transactions. Please try again later.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    Ok(HttpResponse::Ok().json(transactions))
}
