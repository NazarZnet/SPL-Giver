mod args;

pub use args::{Args, Commands, CreateSuperuserArgs};
use clap::Parser;
use common::{Buyer, Database, SplToken, User};

/// Runs the CLI command parser and executes the selected command.
/// Returns true if a CLI command was handled, false otherwise.
pub async fn run_cli() -> bool {
    let args = Args::parse();
    match &args.command {
        Some(Commands::CreateSuperuser(superuser_args)) => {
            if let Err(e) = create_superuser(
                &superuser_args.username,
                &superuser_args.email,
                &superuser_args.password,
            )
            .await
            {
                eprintln!("Failed to create superuser: {e}");
            }
            true
        }
        Some(Commands::CreateWallet) => {
            match get_client_url() {
                Ok(client_url) => match generate_main_wallet(&client_url).await {
                    Ok((wallet_pubkey, wallet_str)) => println!(
                        "Wallet successfully generated!\n Pubkey:{} Base58 Keypair: {}",
                        wallet_pubkey, wallet_str
                    ),
                    Err(e) => eprintln!("Failed to generate wallet: {e}"),
                },
                Err(e) => eprintln!("{e}"),
            }
            true
        }
        Some(Commands::CreateMint(create_mint_args)) => {
            match get_client_url() {
                Ok(client_url) => {
                    match generate_mint(
                        &client_url,
                        &create_mint_args.wallet,
                        create_mint_args.decimals,
                    )
                    .await
                    {
                        Ok(mint_str) => println!(
                            "Mint token successfully generated! Base58 Pubkey: {}",
                            mint_str
                        ),
                        Err(e) => eprintln!("Failed to generate mint token: {e}"),
                    }
                }
                Err(e) => eprintln!("{e}"),
            }
            true
        }
        Some(Commands::MintTokens(mint_tokens_args)) => {
            match get_client_url() {
                Ok(client_url) => match mint_tokens(
                    &client_url,
                    &mint_tokens_args.wallet,
                    &mint_tokens_args.mint,
                    mint_tokens_args.amount,
                )
                .await
                {
                    Ok(_) => println!("Successfully minted {} tokens!", mint_tokens_args.amount),
                    Err(e) => eprintln!("Failed to mint tokens: {e}"),
                },
                Err(e) => eprintln!("{e}"),
            }
            true
        }
        Some(Commands::GenerateBuyers(generate_buyers_args)) => {
            match Buyer::generate_test_buyers_csv_async(
                &generate_buyers_args.out,
                generate_buyers_args.count,
                generate_buyers_args.group_count,
            )
            .await
            {
                Ok(_) => println!(
                    "Successfully generated buyers to: {}",
                    generate_buyers_args.out
                ),
                Err(e) => eprintln!("Failed to generate buyers: {e}"),
            }
            true
        }
        None => {
            println!("No CLI command provided. Use --help to see available commands.");
            false
        }
    }
}

/// Creates a superuser: validates input, hashes password, checks for duplicates, and saves to DB.
async fn create_superuser(username: &str, email: &str, password: &str) -> anyhow::Result<()> {
    // Validate and hash
    let user = User::new(username, email, password, true)
        .map_err(|e| anyhow::anyhow!("Validation error: {e}"))?;

    // Connect to DB (adjust as needed for your project)
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL not set"))?;
    let db = Database::new(&database_url).await?;

    // Check if user already exists
    if db.get_user(username).await?.is_some() {
        return Err(anyhow::anyhow!(
            "A user with username '{}' already exists.",
            username
        ));
    }

    // Save to DB
    db.save_user(&user)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {e}"))?;

    println!("Superuser '{}' created successfully.", username);
    Ok(())
}

/// Helper to fetch CLIENT_URL from environment.
fn get_client_url() -> Result<String, String> {
    std::env::var("CLIENT_URL")
        .map_err(|e| format!("Error: CLIENT_URL environment variable not set: {e}"))
}

/// Generates a new Solana wallet and returns its PublicKey and Base58 keypair string.
async fn generate_main_wallet(client_url: &str) -> anyhow::Result<(String, String)> {
    let client = SplToken::connect(client_url).await;
    let wallet = common::SplToken::generate_wallet(&client).await?;
    let wallet_str = wallet.to_base58_string();
    let wallet_pubkey = common::SplToken::pubkey_from_keypair(&wallet).to_string();
    Ok((wallet_pubkey, wallet_str))
}

/// Creates a new SPL mint using the provided wallet keypair string and mint decimals. Returns the mint's base58 pubkey.
async fn generate_mint(client_url: &str, wallet_str: &str, decimals: u8) -> anyhow::Result<String> {
    let client = SplToken::connect(client_url).await;
    let wallet = SplToken::keypair_from_str(wallet_str);
    let mint = SplToken::create_mint(&client, &wallet, decimals).await?;
    let mint_str = mint.to_string();
    Ok(mint_str)
}

/// Mints tokens to a wallet using the given mint and amount.
async fn mint_tokens(
    client_url: &str,
    wallet_str: &str,
    mint_str: &str,
    amount: u64,
) -> anyhow::Result<()> {
    let client = SplToken::connect(client_url).await;
    let wallet = SplToken::keypair_from_str(wallet_str);
    let wallet_pubkey = SplToken::pubkey_from_keypair(&wallet);
    let mint = SplToken::pubkey_from_str(mint_str)?;
    let token_account =
        SplToken::get_or_create_associated_token_account(&client, &wallet_pubkey, &wallet, &mint)
            .await?;
    SplToken::mint_tokens(&client, &wallet, &mint, &token_account, amount).await
}
