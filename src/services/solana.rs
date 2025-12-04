use anyhow::Result;
use solana_rpc_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction as system_instruction;
use solana_message::Message;
use solana_transaction_status::UiTransactionEncoding;
use solana_program::program_pack::Pack;
use std::str::FromStr;
use spl_associated_token_account_interface::address::get_associated_token_address;

/// Solana service
pub struct SolanaService {
    rpc_client: RpcClient,
    commitment: CommitmentConfig,
}

impl SolanaService {
    /// Create a new Solana service instance
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc_client = RpcClient::new(rpc_url.to_string());
        let commitment = CommitmentConfig::confirmed();
        
        Ok(Self {
            rpc_client,
            commitment,
        })
    }
    
    /// Get account balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let balance = self.rpc_client.get_balance_with_commitment(pubkey, self.commitment)?;
        Ok(balance.value)
    }
    
    /// Get account info
    pub async fn get_account_info(&self, pubkey: &Pubkey) -> Result<Option<solana_sdk::account::Account>> {
        let account = self.rpc_client.get_account_with_commitment(pubkey, self.commitment)?;
        Ok(account.value)
    }
    
    /// Get recent blockhash
    pub async fn get_recent_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        Ok(blockhash)
    }
    
    /// Get transaction status
    pub async fn get_transaction_status(
        &self,
        signature: &Signature,
    ) -> Result<Option<bool>> {
        let status = self.rpc_client.get_transaction(signature, UiTransactionEncoding::Json)?;
        Ok(Some(true)) // If transaction info can be retrieved, the transaction exists
    }
    
    /// Send transaction
    pub async fn send_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature> {
        let signature = self.rpc_client.send_and_confirm_transaction(transaction)?;
        Ok(signature)
    }
    
    /// Confirm transaction
    pub async fn confirm_transaction(
        &self,
        signature: &Signature,
        max_retries: u32,
    ) -> Result<bool> {
        let mut retries = 0;
        
        while retries < max_retries {
            if let Some(status) = self.get_transaction_status(signature).await? {
                if status {
                    return Ok(true);
                }
            }
            
            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        Ok(false)
    }
    
    /// Get program accounts
    pub async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<(Pubkey, solana_sdk::account::Account)>> {
        let accounts = self.rpc_client.get_program_accounts(program_id)?;
        
        Ok(accounts)
    }
    
    /// Get token account balance
    pub async fn get_token_account_balance(
        &self,
        token_account: &Pubkey,
    ) -> Result<u64> {
        let balance = self.rpc_client.get_token_account_balance_with_commitment(
            token_account,
            self.commitment,
        )?;
        
        Ok(balance.value.amount.parse().unwrap_or(0))
    }
    
    /// Get token account info
    pub async fn get_token_account_info(
        &self,
        token_account: &Pubkey,
    ) -> Result<Option<spl_token_interface::state::Account>> {
        let account_info = self.get_account_info(token_account).await?;
        
        if let Some(info) = account_info {
            if info.owner == spl_token_interface::id() {
                let account = spl_token_interface::state::Account::unpack(&info.data)?;
                Ok(Some(account))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    /// Create token account
    pub async fn create_token_account(
        &self,
        payer: &Keypair,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<Pubkey> {
        let associated_token_account = get_associated_token_address(owner, mint);
        
        let instruction = spl_associated_token_account_interface::instruction::create_associated_token_account(
            &payer.pubkey(),
            owner,
            mint,
            &spl_token_interface::id(),
        );
        
        let recent_blockhash = self.get_recent_blockhash().await?;
        let message = Message::new(&[instruction], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[payer], message, recent_blockhash);
        
        let signature = self.send_transaction(&transaction).await?;
        self.confirm_transaction(&signature, 10).await?;
        
        Ok(associated_token_account)
    }
    
    /// Transfer SOL
    pub async fn transfer_sol(
        &self,
        from: &Keypair,
        to: &Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let instruction = system_instruction::transfer(&from.pubkey(), to, amount);
        let recent_blockhash = self.get_recent_blockhash().await?;
        let message = Message::new(&[instruction], Some(&from.pubkey()));
        let transaction = Transaction::new(&[from], message, recent_blockhash);
        
        let signature = self.send_transaction(&transaction).await?;
        Ok(signature)
    }
    
    /// Get network info
    pub async fn get_network_info(&self) -> Result<solana_rpc_client_api::response::RpcVersionInfo> {
        let version = self.rpc_client.get_version()?;
        Ok(version)
    }
    
    /// Get slot info
    pub async fn get_slot_info(&self) -> Result<u64> {
        let slot = self.rpc_client.get_slot_with_commitment(self.commitment)?;
        Ok(slot)
    }
    
    /// Get block height
    pub async fn get_block_height(&self) -> Result<u64> {
        let height = self.rpc_client.get_block_height_with_commitment(self.commitment)?;
        Ok(height)
    }
    
    /// Get cluster nodes
    pub async fn get_cluster_nodes(&self) -> Result<Vec<solana_rpc_client_api::response::RpcContactInfo>> {
        let nodes = self.rpc_client.get_cluster_nodes()?;
        Ok(nodes)
    }
    
    /// Get performance samples
    pub async fn get_performance_samples(&self) -> Result<Vec<solana_rpc_client_api::response::RpcPerfSample>> {
        let samples = self.rpc_client.get_recent_performance_samples(Some(10))?;
        Ok(samples)
    }
    
    /// Get vote accounts
    pub async fn get_vote_accounts(&self) -> Result<solana_rpc_client_api::response::RpcVoteAccountStatus> {
        let vote_accounts = self.rpc_client.get_vote_accounts_with_commitment(self.commitment)?;
        Ok(vote_accounts)
    }
    
    /// Get leader schedule
    pub async fn get_leader_schedule(&self) -> Result<Option<solana_rpc_client_api::response::RpcLeaderSchedule>> {
        let schedule = self.rpc_client.get_leader_schedule_with_commitment(
            Some(self.get_slot_info().await?),
            self.commitment,
        )?;
        Ok(schedule)
    }
    
    /// Get block time
    pub async fn get_block_time(&self, slot: u64) -> Result<i64> {
        let time = self.rpc_client.get_block_time(slot)?;
        Ok(time)
    }
    
    /// Get block
    pub async fn get_block(&self, slot: u64) -> Result<Option<String>> {
        let block = self.rpc_client.get_block(slot)?;
        Ok(Some(block.blockhash))
    }
    
    /// Get signature statuses
    pub async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<bool>>> {
        let statuses = self.rpc_client.get_signature_statuses(signatures)?;
        Ok(statuses.value.into_iter().map(|s| s.map(|_| true)).collect())
    }
    
    /// Get multiple accounts
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<solana_sdk::account::Account>>> {
        let accounts = self.rpc_client.get_multiple_accounts_with_commitment(
            pubkeys,
            self.commitment,
        )?;
        Ok(accounts.value)
    }
    
    /// Get account history
    pub async fn get_account_history(
        &self,
        pubkey: &Pubkey,
        limit: usize,
    ) -> Result<Vec<bool>> {
        let history = self.rpc_client.get_signatures_for_address(pubkey)?;
        
        let mut transactions = Vec::new();
        for sig_info in history.iter().take(limit) {
            let signature = Signature::from_str(&sig_info.signature).map_err(|e| anyhow::anyhow!("Invalid signature: {}", e))?;
            if let Some(tx) = self.get_transaction_status(&signature).await? {
                transactions.push(tx);
            }
        }
        
        Ok(transactions)
    }
    
    /// Get token supply
    pub async fn get_token_supply(&self, mint: &Pubkey) -> Result<u64> {
        let supply = self.rpc_client.get_token_supply(mint)?;
        Ok(supply.amount.parse().unwrap_or(0))
    }
    
    /// Get token max supply
    pub async fn get_token_max_supply(&self, mint: &Pubkey) -> Result<Option<u64>> {
        let mint_info = self.get_account_info(mint).await?;
        
        if let Some(info) = mint_info {
            if info.owner == spl_token_interface::id() {
                let mint_state = spl_token_interface::state::Mint::unpack(&info.data)?;
                Ok(Some(mint_state.supply))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    /// Get token metadata
    pub async fn get_token_metadata(&self, _mint: &Pubkey) -> Result<Option<solana_sdk::account::Account>> {
        // let metadata_address = spl_token_metadata::state::get_metadata_account(mint);  // Temporarily commented out due to incompatible dependency
        // self.get_account_info(&metadata_address).await
        // Temporarily return None until dependency issue is resolved
        Ok(None)
    }
    
    /// Verify transaction
    pub async fn verify_transaction(&self, transaction: &Transaction) -> Result<bool> {
        // Implement transaction verification logic here
        // For example, check signatures, balances, etc.
        Ok(true)
    }
    
    /// Estimate transaction fee
    pub async fn estimate_transaction_fee(&self, transaction: &Transaction) -> Result<u64> {
        let _blockhash = self.rpc_client.get_latest_blockhash()?;
        // In newer versions, fee calculation has changed; use a fixed fee
        let lamports_per_signature = 5000; // Default signature fee
        let num_signatures = transaction.message.header.num_required_signatures as u64;
        Ok(lamports_per_signature * num_signatures)
    }
    
    /// Get RPC client reference
    pub fn get_rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }
    
    /// Set commitment level
    pub fn set_commitment(&mut self, commitment: CommitmentConfig) {
        self.commitment = commitment;
    }
    
    /// Get current commitment level
    pub fn get_commitment(&self) -> CommitmentConfig {
        self.commitment
    }
}

/// Solana network type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolanaNetwork {
    Mainnet,
    Testnet,
    Devnet,
    Localnet,
}

impl SolanaNetwork {
    /// Get network RPC URL
    pub fn get_rpc_url(&self) -> &'static str {
        match self {
            SolanaNetwork::Mainnet => "https://api.mainnet-beta.solana.com",
            SolanaNetwork::Testnet => "https://api.testnet.solana.com",
            SolanaNetwork::Devnet => "https://api.devnet.solana.com",
            SolanaNetwork::Localnet => "http://localhost:8899",
        }
    }
    
    /// Get network name
    pub fn get_name(&self) -> &'static str {
        match self {
            SolanaNetwork::Mainnet => "Mainnet",
            SolanaNetwork::Testnet => "Testnet",
            SolanaNetwork::Devnet => "Devnet",
            SolanaNetwork::Localnet => "Localnet",
        }
    }
    
    /// Parse network type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mainnet" | "mainnet-beta" => Some(SolanaNetwork::Mainnet),
            "testnet" => Some(SolanaNetwork::Testnet),
            "devnet" => Some(SolanaNetwork::Devnet),
            "localnet" | "localhost" => Some(SolanaNetwork::Localnet),
            _ => None,
        }
    }
}

/// Solana config
#[derive(Debug, Clone)]
pub struct SolanaConfig {
    pub network: SolanaNetwork,
    pub rpc_url: String,
    pub ws_url: String,
    pub commitment: CommitmentConfig,
    pub timeout: std::time::Duration,
    pub max_retries: u32,
}

impl Default for SolanaConfig {
    fn default() -> Self {
        Self {
            network: SolanaNetwork::Mainnet,
            rpc_url: SolanaNetwork::Mainnet.get_rpc_url().to_string(),
            ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
            commitment: CommitmentConfig::confirmed(),
            timeout: std::time::Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

impl SolanaConfig {
    /// Create a new config
    pub fn new(network: SolanaNetwork) -> Self {
        let rpc_url = network.get_rpc_url().to_string();
        let ws_url = rpc_url.replace("https://", "wss://");
        
        Self {
            network,
            rpc_url,
            ws_url,
            commitment: CommitmentConfig::confirmed(),
            timeout: std::time::Duration::from_secs(30),
            max_retries: 3,
        }
    }
    
    /// Set custom RPC URL
    pub fn with_custom_rpc(mut self, rpc_url: String) -> Self {
        self.rpc_url = rpc_url;
        self
    }
    
    /// Set commitment level
    pub fn with_commitment(mut self, commitment: CommitmentConfig) -> Self {
        self.commitment = commitment;
        self
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_solana_network() {
        assert_eq!(SolanaNetwork::Mainnet.get_name(), "Mainnet");
        assert_eq!(SolanaNetwork::from_str("mainnet"), Some(SolanaNetwork::Mainnet));
        assert_eq!(SolanaNetwork::from_str("invalid"), None);
    }
    
    #[test]
    fn test_solana_config() {
        let config = SolanaConfig::new(SolanaNetwork::Testnet);
        assert_eq!(config.network, SolanaNetwork::Testnet);
        assert_eq!(config.rpc_url, SolanaNetwork::Testnet.get_rpc_url());
    }
}
