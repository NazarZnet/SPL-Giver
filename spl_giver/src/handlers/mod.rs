mod auth;
mod buyers;
mod groups;
mod schedule;
mod transactions;

use actix_web::{HttpResponse, Responder, get};
pub use auth::*;
pub use buyers::*;
pub use groups::*;
pub use schedule::*;
pub use transactions::*;

#[get("/")]
pub async fn index() -> impl Responder {
    HttpResponse::Ok().body("Welcome to Spl Token Service!")
}
