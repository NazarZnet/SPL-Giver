use actix_web::{Error, HttpResponse, error::InternalError, get, http::StatusCode, post, web};
use serde::Deserialize;
use serde_json::json;

use crate::{distribution::process_schedule, state::AppState};
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
                .get_schedules_by_status(query.status.as_ref().unwrap())
                .await
        }
        None => app_state.db.get_all_schedules().await,
        _ => unreachable!(),
    };

    let schedules = schedule_result.map_err(|e| {
        log::error!("Failed to get schedule: {}", e);
        InternalError::new(
            "Failed to get schedule. Please try again later.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    Ok(HttpResponse::Ok().json(schedules))
}

#[post("/schedule/retry")]
pub async fn retry_failed_schedule(app_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let schedules = app_state
        .db
        .get_schedules_by_status("failed")
        .await
        .map_err(|e| {
            log::error!("Failed to get schedules with status 'failed': {}", e);
            InternalError::new(
                "Failed to get schedules. Please try again later.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    if schedules.is_empty() {
        return Ok(HttpResponse::Ok().json(json!({
            "retried": [],
            "failed": [],
            "message": "Nothing to retry — all schedules already processed successfully."
        })));
    }

    let mut retried = Vec::new();
    let mut failed = Vec::new();

    for schedule in schedules {
        match process_schedule(&app_state, &schedule, app_state.spl_token.decimals).await {
            Ok(updated) => retried.push(updated),
            Err(e) => {
                log::error!("Failed to retry schedule {}: {}", schedule.id, e);
                failed.push(FailedRetry {
                    schedule_id: schedule.id,
                    error: e.to_string(),
                });
            }
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "retried": retried,
        "failed": failed,
        "message": format!(
            "Retried {} schedules, {} failed.",
            retried.len(),
            failed.len()
        )
    })))
}

#[derive(serde::Serialize)]
struct FailedRetry {
    schedule_id: i64,
    error: String,
}
