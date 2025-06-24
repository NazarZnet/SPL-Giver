mod config;
mod distribution;
mod handlers;
mod state;

use actix_jwt_auth_middleware::{Authority, TokenSigner, use_jwt::UseJWTOnApp};
use actix_state_guards::UseStateGuardOnScope;
use actix_web::{
    App, HttpServer, error::InternalError, http::StatusCode, middleware::Logger, rt::System, web,
};
use common::User;
use dotenv::dotenv;
use ed25519_compact::KeyPair;
use jwt_compact::alg::Ed25519;
use pretty_env_logger::env_logger::{Builder, Env};

use distribution::{check_group_token_funding, initialize_schedules};

use crate::config::AppConfig;

//DONE: Check transaction send some times
//DONE: Create database with transations history
// DONE: Make after fall start distribution from history
// DONE: Check that group has enought tokens
//DONE: create routes to get transaction history and all information about buyers and so on
//DONE: create authorization for all routes
//DONE: Create route to try again transafer failed transactions
//TODO: create documentation and api doc
//DONE: rewrite distribute not shedule task. Create loop that will check if there are any sheduled tasks exists and run them

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    if cli::run_cli().await {
        return Ok(());
    }

    let logger_env = Env::default().default_filter_or("debug");
    let mut logger_builder = Builder::from_env(logger_env);
    logger_builder.init();

    let config = AppConfig::from_env().map_err(|e| {
        log::error!("Application initialization failed: {:#}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    })?;

    let state = config.create_app_state().await.map_err(|e| {
        log::error!("Application initialization failed: {:#}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    })?;

    log::info!("App state initialized successfully");

    state
        .initialize_data_from_files(&config.groups_yaml, &config.buyers_csv)
        .await
        .map_err(|e| {
            log::error!("Data initialization failed: {:#}", e);
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

    log::info!("Initial data loaded successfully");

    // Check admin ATA balance
    check_group_token_funding(&state).await.map_err(|e| {
        log::error!("Admin ATA balance check failed: {:#}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    })?;

    log::info!("Admin ATA balance is OK");

    // Initialize schedules
    initialize_schedules(&state).await.map_err(|e| {
        log::error!("Failed to initialize schedules: {:#}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    })?;
    log::info!("Schedules initialized successfully");

    let data = web::Data::new(state);

    // Spawn the schedule runner
    {
        let runner_state = data.clone();
        tokio::spawn(async move {
            if let Err(e) = distribution::start_schedule_runner(runner_state).await {
                log::error!("Schedule runner encountered an error: {:#}", e);
                // Gracefully stop the Actix system
                System::current().stop();
            }
        });
    }

    //Authorization
    let KeyPair {
        pk: public_key,
        sk: secret_key,
    } = KeyPair::generate();

    HttpServer::new(move || {
        let authority = Authority::<User, Ed25519, _, _>::new()
            .refresh_authorizer(|| async move { Ok(()) })
            .token_signer(Some(
                TokenSigner::new()
                    .signing_key(secret_key.clone())
                    .algorithm(Ed25519)
                    .build()
                    .expect("Failed to generate TokenSigner"),
            ))
            .verifying_key(public_key)
            .build()
            .expect("Failed to create Authority");

        App::new()
            .app_data(data.clone())
            .wrap(Logger::new("%a %t %r %s  %{Referer}i %Dms"))
            .service(handlers::login)
            .use_jwt(
                authority,
                web::scope("")
                    .service(handlers::index)
                    .service(handlers::get_transactions)
                    .service(handlers::get_schedule)
                    .service(handlers::retry_failed_schedule)
                    .service(handlers::get_buyer_by_wallet)
                    .service(handlers::get_buyers)
                    .service(handlers::get_all_groups)
                    .service(handlers::get_group_by_id)
                    .use_state_guard(
                        |user: User| async move {
                            if user.is_superuser {
                                Ok(())
                            } else {
                                Err(InternalError::new(
                                    "You are not an Admin",
                                    StatusCode::UNAUTHORIZED,
                                ))
                            }
                        },
                        web::scope("").service(handlers::index),
                    ),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
