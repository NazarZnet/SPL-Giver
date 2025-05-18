mod distribution;
mod state;

use actix_web::{App, HttpResponse, HttpServer, Responder, get, middleware::Logger, web};

use dotenv::dotenv;
use pretty_env_logger::env_logger::{Builder, Env};

use state::AppState;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Spl Token Service")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let logger_env = Env::default().default_filter_or("debug");
    let mut logger_builder = Builder::from_env(logger_env);
    logger_builder.init();

    //Create buyers_list.csv. Onlu for testing
    // let _ = state::generate_test_buyers_csv_async("buyers_list.csv", 20, 2)
    //     .await
    //     .map_err(|e| {
    //         log::error!("Failed to generate buyers_list.csv: {:#?}", e);
    //         std::process::exit(1);
    //     });

    let state = AppState::new("groups.yaml", "buyers_list.csv")
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to generate app state: {:#?}", e);
            std::process::exit(1);
        });
    log::info!("App state generated successfully");

    let data = web::Data::new(state);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::new("%a %t %r %s  %{Referer}i %Dms"))
            .service(index)
            .service(distribution::distribute_tokens)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
