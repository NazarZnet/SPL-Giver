mod auth;
use actix_web::{HttpResponse, Responder, get};
pub use auth::*;

#[get("/")]
pub async fn index() -> impl Responder {
    HttpResponse::Ok().body("Welcome to Spl Token Service!")
}
