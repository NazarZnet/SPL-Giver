use clap::{Args as ClapArgs, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about = "SPL Giver CLI - manage admin users and more")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a superuser (admin) account
    CreateSuperuser(CreateSuperuserArgs),

    /// Create a new Solana wallet (for testing only)
    ///
    /// This command generates a new Solana wallet and saves the keypair to a file.
    /// For testing purposes only.
    CreateWallet,

    /// Create a new SPL mint (for testing only)
    ///
    /// This command creates a new SPL mint using the provided wallet.
    /// For testing purposes only.
    CreateMint(CreateMintArgs),

    /// Generate test buyers and save to CSV (for testing only)
    ///
    /// This command generates random buyers and saves them to a CSV file.
    /// For testing purposes only.
    GenerateBuyers(GenerateBuyersArgs),

    /// Mint tokens to a wallet (for testing only)
    ///
    /// This command mints a specified amount of tokens to a wallet using a given mint.
    /// For testing purposes only.
    /// Note: The amount should be specified in the smallest units (according to the mint's decimals).
    MintTokens(MintTokensArgs),
}

#[derive(ClapArgs, Debug)]
pub struct CreateSuperuserArgs {
    /// Username for the superuser
    #[arg(short, long, help = "Username for the superuser")]
    pub username: String,

    /// Email address for the superuser
    #[arg(short, long, help = "Email address for the superuser")]
    pub email: String,

    /// Password for the superuser
    #[arg(short, long, help = "Password for the superuser")]
    pub password: String,
}

#[derive(ClapArgs, Debug)]
pub struct CreateMintArgs {
    /// Base58-encoded wallet keypair (for testing only)
    #[arg(short, long, help = "Base58-encoded wallet keypair")]
    pub wallet: String,

    /// Number of decimals for the mint (for testing only)
    #[arg(short, long, help = "Number of decimals for the new mint")]
    pub decimals: u8,
}

#[derive(ClapArgs, Debug)]
pub struct GenerateBuyersArgs {
    /// Number of buyers to generate (for testing only)
    #[arg(short, long, help = "Number of buyers to generate")]
    pub count: i64,

    /// Number of groups for random generation (for testing only)
    #[arg(short, long, help = "Number of groups for random generation")]
    pub group_count: i64,

    /// Output CSV file path (for testing only)
    #[arg(short, long, help = "Output CSV file path")]
    pub out: String,
}

#[derive(ClapArgs, Debug)]
pub struct MintTokensArgs {
    /// Base58-encoded wallet address to receive tokens (for testing only)
    #[arg(short, long, help = "Base58-encoded wallet address to receive tokens")]
    pub wallet: String,

    /// Base58-encoded mint address (for testing only)
    #[arg(short, long, help = "Base58-encoded mint address")]
    pub mint: String,

    /// Amount of tokens to mint (for testing only)
    #[arg(short, long, help = "Amount of tokens to mint")]
    pub amount: u64,
}
