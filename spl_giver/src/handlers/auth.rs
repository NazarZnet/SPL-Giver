use crate::state::AppState;
use actix_jwt_auth_middleware::TokenSigner;
use actix_web::Error;
use actix_web::{HttpResponse, error::InternalError, http::StatusCode, post, web};
use common::User;
use jwt_compact::alg::Ed25519;

#[derive(Debug, serde::Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

#[post("/login")]
pub async fn login(
    login_data: web::Json<LoginData>,
    app_state: web::Data<AppState>,
    cookie_signer: web::Data<TokenSigner<User, Ed25519>>,
) -> Result<HttpResponse, Error> {
    let user = match app_state
        .db
        .get_user(&login_data.username)
        .await
        .map_err(|e| {
            log::error!(
                "Database error while looking up user `{}`: {}",
                login_data.username,
                e
            );
            InternalError::new(
                "Internal error looking up user",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })? {
        Some(u) => u,
        None => {
            log::warn!(
                "Login attempt for non-existent user `{}`",
                login_data.username
            );
            return Err(InternalError::new(
                "User with provided username not found!",
                StatusCode::UNAUTHORIZED,
            )
            .into());
        }
    };

    if let Err(err) = user.verify_password(&login_data.password) {
        log::warn!(
            "Invalid password for user {}: {:?}",
            login_data.username,
            err
        );
        return Ok(HttpResponse::Unauthorized().body("Invalid username or password"));
    }

    let access_cookie = cookie_signer.create_access_cookie(&user).map_err(|err| {
        log::error!("Failed to create access token: {:?}", err);
        InternalError::new("Token error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let refresh_cookie = cookie_signer.create_refresh_cookie(&user).map_err(|err| {
        log::error!("Failed to create refresh token: {:?}", err);
        InternalError::new("Token error", StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    Ok(HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .body("Login successful."))
}
