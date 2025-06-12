use std::sync::Arc;

use anyhow::{Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
    system_instruction::create_account, transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::{
    extension::{ExtensionType, metadata_pointer},
    id as token_2022_program_id,
    instruction::{initialize_mint, mint_to, transfer_checked},
    state::Mint,
    ui_amount_to_amount,
};
use spl_token_metadata_interface::state::TokenMetadata;
pub struct SplTokenContext {
    pub mint: Pubkey,
    token_acount: Pubkey,
    pub main_wallet: Arc<Keypair>,
    pub client: RpcClient,
    pub amount: u64,
}

impl SplTokenContext {
    pub async fn new(
        client: RpcClient,
        main_wallet: Keypair,
        mint: Pubkey,
        token_account: Pubkey,
        amount: u64,
    ) -> Result<Self> {
        let current_token_amount = Self::get_token_account_balance(&client, &token_account).await?;
        if current_token_amount < amount {
            Self::mint_tokens(
                &client,
                &main_wallet,
                &mint,
                &token_account,
                amount - current_token_amount,
            )
            .await?;
        }
        Ok(Self {
            mint,
            token_acount: token_account,
            main_wallet: Arc::new(main_wallet),
            client,
            amount,
        })
    }
    pub async fn generate_wallet(client: &RpcClient) -> Result<Keypair> {
        let wallet = Keypair::new();

        // Airdrop 1 SOL to fee payer
        let airdrop_signature = client
            .request_airdrop(&wallet.pubkey(), LAMPORTS_PER_SOL)
            .await?;
        client.confirm_transaction(&airdrop_signature).await?;

        loop {
            let confirmed = client.confirm_transaction(&airdrop_signature).await?;
            if confirmed {
                break;
            }
        }

        Ok(wallet)
    }
    pub async fn check_wallet_balance(client: &RpcClient, wallet: &Pubkey) -> Result<u64> {
        let balance = client
            .get_balance(wallet)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get balance: {}", e))?;
        Ok(balance)
    }

    pub async fn create_mint(client: &RpcClient, fee_payer: &Keypair) -> Result<Pubkey> {
        let recent_blockhash = client.get_latest_blockhash().await?;
        let mint = Keypair::new();

        let metadata = TokenMetadata {
            update_authority: Some(fee_payer.pubkey()).try_into()?,
            mint: mint.pubkey(),
            name: String::from("OPOS"),
            symbol: String::from("OPOS"),
            uri: String::from(
                "https://raw.githubusercontent.com/solana-developers/opos-asset/main/assets/DeveloperPortal/metadata.json",
            ),
            additional_metadata: [(
                "description".to_string(),
                "Only Possible On Solana".to_string(),
            )]
            .to_vec(),
        };

        let metadata_len = metadata.tlv_size_of()?;

        let space =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])?;

        let rent = client
            .get_minimum_balance_for_rent_exemption(space + 4 + metadata_len)
            .await?;

        let create_account_instruction = create_account(
            &fee_payer.pubkey(),
            &mint.pubkey(),
            rent,
            space as u64,
            &token_2022_program_id(),
        );

        let metadata_pointer_instruction = metadata_pointer::instruction::initialize(
            &token_2022_program_id(),
            &mint.pubkey(),
            Some(fee_payer.pubkey()),
            Some(mint.pubkey()),
        )?;

        let initialize_mint_instruction = initialize_mint(
            &token_2022_program_id(),
            &mint.pubkey(),
            &fee_payer.pubkey(),
            Some(&fee_payer.pubkey()),
            9, // decimals
        )?;

        let metadata_instruction = spl_token_metadata_interface::instruction::initialize(
            &token_2022_program_id(),
            &mint.pubkey(),
            &fee_payer.pubkey(),
            &mint.pubkey(),
            &fee_payer.pubkey(),
            metadata.name,
            metadata.symbol,
            metadata.uri,
        );

        let update_metadata_fields_instruction =
            spl_token_metadata_interface::instruction::update_field(
                &token_2022_program_id(),
                &mint.pubkey(),
                &fee_payer.pubkey(),
                spl_token_metadata_interface::state::Field::Key(
                    metadata.additional_metadata[0].0.clone(),
                ),
                metadata.additional_metadata[0].1.clone(),
            );

        let transaction = Transaction::new_signed_with_payer(
            &[
                create_account_instruction,
                metadata_pointer_instruction,
                initialize_mint_instruction,
                metadata_instruction,
                update_metadata_fields_instruction,
            ],
            Some(&fee_payer.pubkey()),
            &[fee_payer, &mint],
            recent_blockhash,
        );

        client.send_and_confirm_transaction(&transaction).await?;
        Ok(mint.pubkey())
    }
    pub async fn get_or_create_associated_token_account(
        client: &RpcClient,
        wallet: &Pubkey,
        fee_payer: &Keypair,
        mint_pubkey: &Pubkey,
    ) -> Result<Pubkey> {
        let associated_token_address = get_associated_token_address_with_program_id(
            wallet,
            mint_pubkey,
            &token_2022_program_id(),
        );

        // Check if the associated token account already exists
        if let Ok(_account) = client.get_account(&associated_token_address).await {
            return Ok(associated_token_address);
        }

        // If the account does not exist, create it
        let recent_blockhash = client.get_latest_blockhash().await?;
        let create_ata_instruction = create_associated_token_account(
            &fee_payer.pubkey(),
            wallet,
            mint_pubkey,
            &token_2022_program_id(),
        );

        let transaction = Transaction::new_signed_with_payer(
            &[create_ata_instruction],
            Some(&fee_payer.pubkey()),
            &[fee_payer],
            recent_blockhash,
        );

        client.send_and_confirm_transaction(&transaction).await?;

        Ok(associated_token_address)
    }
    pub async fn get_token_account_balance(
        client: &RpcClient,
        token_account: &Pubkey,
    ) -> Result<u64> {
        let token_ui_amount = client
            .get_token_account_balance(token_account)
            .await
            .context("Failed to get token account balance")?;
        let row_ammount = ui_amount_to_amount(
            token_ui_amount.ui_amount.unwrap_or(0.0),
            token_ui_amount.decimals,
        );
        Ok(row_ammount)
    }
    pub async fn mint_tokens(
        client: &RpcClient,
        fee_payer: &Keypair,
        mint_pubkey: &Pubkey,
        associated_token_address: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let recent_blockhash = client.get_latest_blockhash().await?;

        let mint_to_instruction = mint_to(
            &token_2022_program_id(),
            mint_pubkey,
            associated_token_address,
            &fee_payer.pubkey(),
            &[&fee_payer.pubkey()],
            amount,
        )?;

        let transaction = Transaction::new_signed_with_payer(
            &[mint_to_instruction],
            Some(&fee_payer.pubkey()),
            &[fee_payer],
            recent_blockhash,
        );

        client.send_and_confirm_transaction(&transaction).await?;
        Ok(())
    }

    pub async fn transfer_tokens(
        &self,
        destination_token_account: &Pubkey,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        let recent_blockhash = self.client.get_latest_blockhash().await?;

        let transfer_instruction = transfer_checked(
            &token_2022_program_id(),
            &self.token_acount,
            &self.mint,
            destination_token_account,
            &self.main_wallet.pubkey(),
            &[&self.main_wallet.pubkey()],
            amount,
            decimals,
        )?;

        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&self.main_wallet.pubkey()),
            &[Arc::clone(&self.main_wallet)],
            recent_blockhash,
        );

        self.client
            .send_and_confirm_transaction(&transaction)
            .await?;
        Ok(())
    }
}
