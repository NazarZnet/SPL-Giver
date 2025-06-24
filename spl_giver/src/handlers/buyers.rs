use std::collections::HashMap;

use crate::state::AppState;
use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use actix_web::{Error, HttpResponse, error::InternalError, get, http::StatusCode, post, web};
use common::{Buyer, Schedule};
use serde::Deserialize;

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

    let maybe_buyer = app_state
        .db
        .get_buyer_by_wallet(&wallet)
        .await
        .map_err(|e| {
            log::error!("DB error fetching buyer `{}`: {}", wallet, e);
            InternalError::new(
                "Internal server error while fetching buyer.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    let buyer = match maybe_buyer {
        Some(b) => b,
        None => {
            log::warn!("Buyer not found: {}", wallet);
            return Err(InternalError::new(
                "Buyer with provided wallet not found.",
                StatusCode::NOT_FOUND,
            )
            .into());
        }
    };

    Ok(HttpResponse::Ok().json(buyer))
}

#[post("/buyers/upload")]
pub async fn upload_buyers_csv(
    MultipartForm(form): MultipartForm<CsvUploadForm>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let filename = form.file.file_name.as_deref().unwrap_or("");
    if !filename.to_lowercase().ends_with(".csv") {
        return Err(InternalError::new("Only CSV files allowed", StatusCode::BAD_REQUEST).into());
    }

    let groups = app_state.db.get_all_groups().await.map_err(|e| {
        log::error!("Failed to fetch groups: {}", e);
        InternalError::new(
            "Failed to fetch groups from database",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    let path = form.file.file.path().to_string_lossy().to_string();

    let buyers = Buyer::load_from_csv(&path, &groups).await.map_err(|e| {
        log::error!("Failed to parse CSV: {}", e);
        InternalError::new(
            format!("Failed to parse CSV: {}", e),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let mut imported = Vec::new();
    let mut skipped = Vec::new();

    for b in &buyers {
        let wallet = b.wallet.to_string();
        match app_state.db.save_buyer(b).await {
            Ok(true) => {
                // newly inserted
                if let Ok(Some(saved)) = app_state.db.get_buyer_by_wallet(&wallet).await {
                    imported.push(saved);
                }
            }
            Ok(false) => {
                // already existed
                if let Ok(Some(existing)) = app_state.db.get_buyer_by_wallet(&wallet).await {
                    skipped.push(existing);
                }
            }
            Err(e) => {
                log::error!("Failed to save buyer {}: {}", wallet, e);
                // treat as skipped
                if let Ok(Some(existing)) = app_state.db.get_buyer_by_wallet(&wallet).await {
                    skipped.push(existing);
                }
            }
        }
    }

    if !imported.is_empty() {
        if let Err(e) = crate::distribution::initialize_schedules(&app_state).await {
            log::error!("Failed to initialize schedules for new buyers: {}", e);
        }
    }

    let mut schedules_map = HashMap::new();
    for buyer in imported.iter().chain(skipped.iter()) {
        let w = buyer.wallet.to_string();
        if let Ok(list) = app_state
            .db
            .get_schedules_by_buyer_and_group(&w, buyer.group_id)
            .await
        {
            schedules_map.insert(w, list);
        }
    }

    log::info!(
        "Uploaded buyers from CSV. Imported {} buyers, skipped {} buyers",
        imported.len(),
        skipped.len()
    );

    let response = UploadBuyersResponse {
        imported,
        skipped,
        schedules: schedules_map,
    };
    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, Deserialize)]
struct BuyerQuery {
    group_id: Option<i64>,
}

#[derive(Debug, MultipartForm)]
struct CsvUploadForm {
    #[multipart(limit = "10MB")]
    file: TempFile,
}

#[derive(serde::Serialize)]
struct UploadBuyersResponse {
    imported: Vec<Buyer>,
    skipped: Vec<Buyer>,
    schedules: HashMap<String, Vec<Schedule>>,
}
