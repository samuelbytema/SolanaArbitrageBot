use async_trait::async_trait;
use anyhow::Result;
use rust_decimal::Decimal;
use solana_program::pubkey::Pubkey;
use crate::models::{Token, Pool, PoolQuote, PoolState, PoolMetrics};
use crate::dex::DexType;

/// Common DEX interface; all DEX implementations must implement this trait
#[async_trait]
pub trait DexInterface: Send + Sync {
    /// Get DEX type
    fn get_dex_type(&self) -> DexType;
    
    /// Get DEX name
    fn get_name(&self) -> &str;
    
    /// Get DEX version
    fn get_version(&self) -> &str;
    
    /// Check DEX connection status
    async fn is_connected(&self) -> Result<bool>;
    
    /// Get all active liquidity pools
    async fn get_pools(&self) -> Result<Vec<Pool>>;
    
    /// Get liquidity pools by token pair
    async fn get_pools_by_tokens(&self, token_a: &Token, token_b: &Token) -> Result<Vec<Pool>>;
    
    /// Get state of a specific pool
    async fn get_pool_state(&self, pool_address: &Pubkey) -> Result<PoolState>;
    
    /// Get token price
    async fn get_token_price(&self, token: &Token, quote_token: &Token) -> Result<Decimal>;
    
    /// Get swap quote
    async fn get_quote(
        &self,
        input_token: &Token,
        output_token: &Token,
        input_amount: Decimal,
        pool_address: Option<&Pubkey>,
    ) -> Result<PoolQuote>;
    
    /// Execute token swap
    async fn execute_swap(
        &self,
        quote: &PoolQuote,
        wallet: &Pubkey,
        slippage_tolerance: Decimal,
    ) -> Result<String>; // Returns transaction signature
    
    /// Get pool metrics
    async fn get_pool_metrics(&self, pool_address: &Pubkey) -> Result<PoolMetrics>;
    
    /// Get DEX-level metrics
    async fn get_dex_metrics(&self) -> Result<DexMetrics>;
    
    /// Subscribe to pool updates
    async fn subscribe_pool_updates(&self, pool_address: &Pubkey) -> Result<PoolUpdateStream>;
    
    /// Get supported token list
    async fn get_supported_tokens(&self) -> Result<Vec<Token>>;
    
    /// Validate a transaction
    async fn validate_transaction(&self, transaction_data: &[u8]) -> Result<bool>;
}

/// DEX metrics
#[derive(Debug, Clone)]
pub struct DexMetrics {
    pub total_volume_24h: Decimal,
    pub total_tvl: Decimal,
    pub total_pools: u64,
    pub active_pools: u64,
    pub total_trades_24h: u64,
    pub average_gas_price: Decimal,
}

/// Pool update stream
pub struct PoolUpdateStream {
    pub pool_address: Pubkey,
    pub update_receiver: tokio::sync::mpsc::Receiver<PoolUpdate>,
}

/// Pool update event
#[derive(Debug, Clone)]
pub enum PoolUpdate {
    ReserveChange {
        reserve_a: Decimal,
        reserve_b: Decimal,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    PriceChange {
        old_price: Decimal,
        new_price: Decimal,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    LiquidityChange {
        old_tvl: Decimal,
        new_tvl: Decimal,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// DEX connection config
#[derive(Debug, Clone)]
pub struct DexConnectionConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub rate_limit: u32,
}

/// DEX error type
#[derive(Debug, thiserror::Error)]
pub enum DexError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Pool not found: {0}")]
    PoolNotFound(String),
    
    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Slippage exceeded: {0}")]
    SlippageExceeded(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// DEX connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DexConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

/// DEX health check result
#[derive(Debug, Clone)]
pub struct DexHealthCheck {
    pub status: DexConnectionStatus,
    pub response_time_ms: u64,
    pub last_successful_request: Option<chrono::DateTime<chrono::Utc>>,
    pub error_count: u64,
    pub success_rate: f64,
}

// Helper functions for default implementations
pub struct DexHelpers;

impl DexHelpers {
    /// Default impl: calculate price impact
    pub fn calculate_price_impact(
        input_amount: Decimal,
        input_reserve: Decimal,
        output_reserve: Decimal,
        fee_rate: Decimal,
    ) -> Result<Decimal> {
        if input_reserve <= Decimal::ZERO || output_reserve <= Decimal::ZERO {
            return Err(DexError::InsufficientLiquidity("Invalid reserves".to_string()).into());
        }
        
        let fee_multiplier = Decimal::ONE - fee_rate;
        let input_with_fee = input_amount * fee_multiplier;
        let numerator = input_with_fee * output_reserve;
        let denominator = input_reserve + input_with_fee;
        
        if denominator <= Decimal::ZERO {
            return Err(DexError::Internal("Division by zero".to_string()).into());
        }
        
        let output_amount = numerator / denominator;
        let price_impact = input_amount / (input_reserve + input_amount);
        
        Ok(price_impact)
    }
    
    /// Default impl: validate slippage
    pub fn validate_slippage(
        expected_output: Decimal,
        actual_output: Decimal,
        max_slippage: Decimal,
    ) -> bool {
        if expected_output <= Decimal::ZERO {
            return false;
        }
        
        let slippage = (expected_output - actual_output).abs() / expected_output;
        slippage <= max_slippage
    }
    
    /// Default impl: find optimal trading route
    pub fn find_optimal_route(
        pools: &[Pool],
        input_token: &Token,
        output_token: &Token,
        input_amount: Decimal,
    ) -> Result<Vec<Pool>> {
        if pools.is_empty() {
            return Err(DexError::PoolNotFound("No pools available".to_string()).into());
        }
        
        // Simple direct path search; real implementations may need more complex algorithms
        let direct_pools: Vec<Pool> = pools
            .iter()
            .filter(|pool| {
                (pool.token_a.mint == input_token.mint && pool.token_b.mint == output_token.mint)
                    || (pool.token_a.mint == output_token.mint && pool.token_b.mint == input_token.mint)
            })
            .cloned()
            .collect();
        
        if direct_pools.is_empty() {
            return Err(DexError::PoolNotFound("No direct path found".to_string()).into());
        }
        
        // Choose the pool with the highest liquidity
        let best_pool = direct_pools
            .iter()
            .max_by(|a, b| {
                let liquidity_a = a.reserve_a + a.reserve_b;
                let liquidity_b = b.reserve_a + b.reserve_b;
                liquidity_a.partial_cmp(&liquidity_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| DexError::Internal("Failed to find best pool".to_string()))?;
        
        Ok(vec![best_pool.clone()])
    }
}

/// Implement From trait for DexError
impl From<reqwest::Error> for DexError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            DexError::Timeout(err.to_string())
        } else if err.is_connect() {
            DexError::ConnectionFailed(err.to_string())
        } else {
            DexError::Internal(err.to_string())
        }
    }
}

impl From<std::io::Error> for DexError {
    fn from(err: std::io::Error) -> Self {
        DexError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for DexError {
    fn from(err: serde_json::Error) -> Self {
        DexError::InvalidResponse(err.to_string())
    }
}
