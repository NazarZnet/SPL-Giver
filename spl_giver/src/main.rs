mod distribution;
mod handlers;
mod state;

use actix_jwt_auth_middleware::{Authority, TokenSigner, use_jwt::UseJWTOnApp};
use actix_state_guards::UseStateGuardOnScope;
use actix_web::{App, HttpServer, error::InternalError, http::StatusCode, middleware::Logger, web};
use common::{Buyer, Group, User};
use dotenv::dotenv;
use ed25519_compact::KeyPair;
use jwt_compact::alg::Ed25519;
use pretty_env_logger::env_logger::{Builder, Env};

use distribution::{check_group_token_funding, initialize_schedules};
use state::AppState;

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

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        log::error!("DATABASE_URL is not set in environment variables");
        std::process::exit(1);
    });
    let client_url = std::env::var("CLIENT_URL").unwrap_or_else(|_| {
        log::error!("CLIENT_URL is not set in environment variables");
        std::process::exit(1);
    });

    let wallet = std::env::var("MAIN_WALLET").unwrap_or_else(|_| {
        log::error!("MAIN_WALLET is not set in environment variables");
        std::process::exit(1);
    });
    let mint = std::env::var("MINT_PUBKEY").unwrap_or_else(|_| {
        log::error!("MINT_PUBKEY is not set in environment variables");
        std::process::exit(1);
    });

    let state = AppState::new(&database_url, &client_url, &wallet, &mint)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to generate app state: {:#?}", e);
            std::process::exit(1);
        });
    log::info!("App state generated successfully");

    //TODO: Move this logic to external function
    let groups = Group::from_yaml_file("../groups.yaml", state.spl_token.balance)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to load groups from YAML file: {:#?}", e);
            std::process::exit(1);
        });
    let buyers = Buyer::load_from_csv("../buyers_list.csv", &groups)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to load buyers from CSV file: {:#?}", e);
            std::process::exit(1);
        });

    for group in groups {
        state.db.save_group(&group).await.unwrap_or_else(|e| {
            log::error!("Failed to save group to database: {:#?}", e);
            std::process::exit(1);
        });
    }

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
