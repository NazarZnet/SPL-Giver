use actix_web::{Error, HttpResponse, error::InternalError, get, http::StatusCode, web};
use serde::Deserialize;

use crate::state::AppState;
#[derive(Debug, Deserialize)]
struct ScheduleQuery {
    #[serde(default)]
    status: Option<String>,
}

#[get("/schedule")]
pub async fn get_schedule(
    query: web::Query<ScheduleQuery>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    // Validate status if provided
    if let Some(ref status) = query.status {
        let valid_statuses = ["pending", "success", "failed"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(InternalError::new(
                "Schedule status must be either 'pending', 'success', or 'failed'.",
                StatusCode::BAD_REQUEST,
            )
            .into());
        }
    }

    // Fetch data based on presence of status
    let schedule_result = match query.status.as_deref() {
        Some("pending") | Some("success") | Some("failed") => {
            app_state
                .db
                .get_transactions_by_status(query.status.as_ref().unwrap())
                .await
        }
        None => app_state.db.get_all_transactions().await,
        _ => unreachable!(),
    };

    let schedule = schedule_result.map_err(|e| {
        log::error!("Failed to get schedule: {}", e);
        InternalError::new(
            "Failed to get schedule. Please try again later.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    Ok(HttpResponse::Ok().json(schedule))
}
