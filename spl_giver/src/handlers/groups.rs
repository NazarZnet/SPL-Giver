use crate::state::AppState;
use actix_web::{Error, HttpResponse, error::InternalError, get, http::StatusCode, web};

#[get("/groups")]
pub async fn get_all_groups(app_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let groups_result = app_state.db.get_all_groups().await;

    let groups = groups_result.map_err(|e| {
        log::error!("Failed to get groups: {}", e);
        InternalError::new(
            "Failed to fetch groups. Please try again later.",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    Ok(HttpResponse::Ok().json(groups))
}

#[get("/groups/{group_id}")]
pub async fn get_group_by_id(
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let group_id = path.into_inner();

    let group = app_state.db.get_group(group_id).await.map_err(|e| {
        log::error!("Failed to fetch group {}: {}", group_id, e);
        InternalError::new(
            "Group with provided group ID not found.",
            StatusCode::NOT_FOUND,
        )
    })?;

    Ok(HttpResponse::Ok().json(group))
}
