mod distribution;
mod schema;
mod state;

use actix_web::{App, HttpResponse, HttpServer, Responder, get, middleware::Logger, web};

use dotenv::dotenv;
use pretty_env_logger::env_logger::{Builder, Env};

use state::AppState;

use crate::{
    distribution::{check_group_token_funding, initialize_schedules},
    schema::{Buyer, Group},
};

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Spl Token Service")
}

//DONE: Check transaction send some times
//DONE: Create database with transations history
// DONE: Make after fall start distribution from history
// DONE: Check that group has enought tokens
//TODO: create routes to get transaction history and all information about buyers and so on
//TODO: create authorization for all routes
//TODO: create documentation and api doc
//DONE: rewrite distribute not shedule task. Create loop that will check if there are any sheduled tasks exists and run them

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let logger_env = Env::default().default_filter_or("debug");
    let mut logger_builder = Builder::from_env(logger_env);
    logger_builder.init();

    //Create buyers_list.csv. Only for testing
    // let _ = Buyer::generate_test_buyers_csv_async("buyers_list.csv", 5, 2)
    //     .await
    //     .map_err(|e| {
    //         log::error!("Failed to generate buyers_list.csv: {:#?}", e);
    //         std::process::exit(1);
    //     });

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        log::error!("DATABASE_URL is not set in environment variables");
        std::process::exit(1);
    });
    let client_url = std::env::var("CLIENT_URL").unwrap_or_else(|_| {
        log::error!("CLIENT_URL is not set in environment variables");
        std::process::exit(1);
    });

    let state = AppState::new(&database_url, client_url)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to generate app state: {:#?}", e);
            std::process::exit(1);
        });
    log::info!("App state generated successfully");

    //TODO: Move this logic to external function
    let groups = Group::from_yaml_file("groups.yaml", state.spl_token_context.balance as f64)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to load groups from YAML file: {:#?}", e);
            std::process::exit(1);
        });
    for group in groups {
        state.db.save_group(&group).await.unwrap_or_else(|e| {
            log::error!("Failed to save group to database: {:#?}", e);
            std::process::exit(1);
        });
    }

    let buyers = Buyer::load_from_csv("buyers_list.csv")
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to load buyers from CSV file: {:#?}", e);
            std::process::exit(1);
        });
    for buyer in buyers {
        state.db.save_buyer(&buyer).await.unwrap_or_else(|e| {
            log::error!("Failed to save buyer to database: {:#?}", e);
            std::process::exit(1);
        });
    }
    // Check admin ATA balance
    if let Err(e) = check_group_token_funding(&state).await {
        log::error!("{}", e);
        std::process::exit(1);
    }

    let data = web::Data::new(state);

    // Run distribution in background
    if let Err(e) = initialize_schedules(data.clone()).await {
        log::error!("Failed to make sheduled tasks: {:#?}", e);
        std::process::exit(1);
    }
    tokio::spawn(distribution::start_schedule_runner(data.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::new("%a %t %r %s  %{Referer}i %Dms"))
            .service(index)
        // .service(distribution::distribute_tokens)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
