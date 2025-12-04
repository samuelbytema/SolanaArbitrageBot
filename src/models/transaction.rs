use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    transaction::Transaction,
    signature::Signature,
};
use solana_commitment_config::CommitmentConfig;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub id: String,
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: Decimal,
    pub token_mint: Pubkey,
    pub fee: Option<u64>,
    pub priority_fee: Option<u64>,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub request_id: String,
    pub signature: Signature,
    pub status: TransactionStatus,
    pub block_time: Option<DateTime<Utc>>,
    pub slot: Option<u64>,
    pub confirmation_status: Option<String>,
    pub error: Option<String>,
    pub gas_used: Option<u64>,
    pub gas_price: Option<u64>,
    pub total_cost: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    pub signature: Signature,
    pub slot: u64,
    pub block_time: DateTime<Utc>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub pre_token_balances: Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
    pub log_messages: Vec<String>,
    pub inner_instructions: Vec<InnerInstruction>,
    pub instructions: Vec<InstructionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub account_index: u8,
    pub mint: Pubkey,
    pub owner: Option<Pubkey>,
    pub program_id: Option<Pubkey>,
    pub ui_token_amount: UiTokenAmount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTokenAmount {
    pub token_amount: String,
    pub decimals: u8,
    pub ui_amount: Option<f64>,
    pub ui_amount_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerInstruction {
    pub index: u8,
    pub instructions: Vec<InstructionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionInfo {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfig {
    pub commitment: CommitmentConfig,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_seconds: u64,
    pub priority_fee_multiplier: f64,
    pub max_priority_fee: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPool {
    pub pending_transactions: Vec<TransactionRequest>,
    pub confirmed_transactions: Vec<TransactionResponse>,
    pub failed_transactions: Vec<TransactionResponse>,
    pub max_pool_size: usize,
}

impl TransactionRequest {
    pub fn new(
        from: Pubkey,
        to: Pubkey,
        amount: Decimal,
        token_mint: Pubkey,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            amount,
            token_mint,
            fee: None,
            priority_fee: None,
            max_retries: 3,
            timeout_seconds: 30,
            created_at: Utc::now(),
        }
    }

    pub fn with_fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }

    pub fn with_priority_fee(mut self, priority_fee: u64) -> Self {
        self.priority_fee = Some(priority_fee);
        self
    }

    pub fn with_retry_config(mut self, max_retries: u32, timeout_seconds: u64) -> Self {
        self.max_retries = max_retries;
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.created_at);
        elapsed.num_seconds() > self.timeout_seconds as i64
    }
}

impl TransactionResponse {
    pub fn new(request_id: String, signature: Signature) -> Self {
        Self {
            request_id,
            signature,
            status: TransactionStatus::Submitted,
            block_time: None,
            slot: None,
            confirmation_status: None,
            error: None,
            gas_used: None,
            gas_price: None,
            total_cost: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn update_status(&mut self, status: TransactionStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    pub fn confirm(&mut self, slot: u64, block_time: DateTime<Utc>) {
        self.status = TransactionStatus::Confirmed;
        self.slot = Some(slot);
        self.block_time = Some(block_time);
        self.confirmation_status = Some("confirmed".to_string());
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self, error: String) {
        self.status = TransactionStatus::Failed;
        self.error = Some(error);
        self.updated_at = Utc::now();
    }

    pub fn is_confirmed(&self) -> bool {
        matches!(self.status, TransactionStatus::Confirmed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, TransactionStatus::Failed)
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.status, TransactionStatus::Pending | TransactionStatus::Submitted)
    }
}

impl TransactionConfig {
    pub fn new() -> Self {
        Self {
            commitment: CommitmentConfig::confirmed(),
            max_retries: 3,
            retry_delay_ms: 1000,
            timeout_seconds: 30,
            priority_fee_multiplier: 1.1,
            max_priority_fee: 1_000_000, // 1 SOL
        }
    }

    pub fn with_commitment(mut self, commitment: CommitmentConfig) -> Self {
        self.commitment = commitment;
        self
    }

    pub fn with_retry_config(mut self, max_retries: u32, retry_delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_ms = retry_delay_ms;
        self
    }

    pub fn with_fee_config(mut self, priority_fee_multiplier: f64, max_priority_fee: u64) -> Self {
        self.priority_fee_multiplier = priority_fee_multiplier;
        self.max_priority_fee = max_priority_fee;
        self
    }
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionPool {
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pending_transactions: Vec::new(),
            confirmed_transactions: Vec::new(),
            failed_transactions: Vec::new(),
            max_pool_size,
        }
    }

    pub fn add_pending(&mut self, transaction: TransactionRequest) -> Result<(), String> {
        if self.pending_transactions.len() >= self.max_pool_size {
            return Err("Transaction pool is full".to_string());
        }
        self.pending_transactions.push(transaction);
        Ok(())
    }

    pub fn remove_pending(&mut self, id: &str) -> Option<TransactionRequest> {
        if let Some(index) = self.pending_transactions.iter().position(|t| t.id == id) {
            Some(self.pending_transactions.remove(index))
        } else {
            None
        }
    }

    pub fn add_confirmed(&mut self, transaction: TransactionResponse) {
        self.confirmed_transactions.push(transaction);
    }

    pub fn add_failed(&mut self, transaction: TransactionResponse) {
        self.failed_transactions.push(transaction);
    }

    pub fn get_pending(&self) -> &[TransactionRequest] {
        &self.pending_transactions
    }

    pub fn get_confirmed(&self) -> &[TransactionResponse] {
        &self.confirmed_transactions
    }

    pub fn get_failed(&self) -> &[TransactionResponse] {
        &self.failed_transactions
    }

    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.pending_transactions.retain(|t| !t.is_expired());
        self.confirmed_transactions.retain(|t| {
            let elapsed = now.signed_duration_since(t.updated_at);
            elapsed.num_hours() < 24 // Keep confirmed transactions for 24 hours
        });
        self.failed_transactions.retain(|t| {
            let elapsed = now.signed_duration_since(t.updated_at);
            elapsed.num_hours() < 24 // Keep failed transactions for 24 hours
        });
    }
}

impl std::fmt::Display for TransactionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Transaction {}: {} -> {} (Amount: {})",
            self.id, self.from, self.to, self.amount
        )
    }
}

impl std::fmt::Display for TransactionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Transaction {}: {} ({:?})",
            self.request_id, self.signature, self.status
        )
    }
}
