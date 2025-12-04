use anyhow::Result;
use solana_sdk::{
    transaction::Transaction,
    signature::Signature,
    pubkey::Pubkey,
};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;

/// Jito MEV protection service
pub struct JitoService {
    client: Client,
    base_url: String,
    auth_header: String,
    timeout: Duration,
}

/// Jito transaction request
#[derive(Debug, Serialize)]
pub struct JitoTransactionRequest {
    pub transaction: String, // Base64-encoded transaction
    pub commitment: String,
    pub skip_preflight: bool,
    pub max_retries: Option<u32>,
    pub min_context_slot: Option<u64>,
}

/// Jito transaction response
#[derive(Debug, Deserialize)]
pub struct JitoTransactionResponse {
    pub signature: String,
    pub slot: u64,
    pub err: Option<serde_json::Value>,
}

/// Jito block builder info
#[derive(Debug, Deserialize)]
pub struct JitoBlockBuilderInfo {
    pub pubkey: String,
    pub fee_recipient: String,
    pub last_slot: u64,
    pub is_active: bool,
}

/// Jito MEV protection config
#[derive(Debug, Clone)]
pub struct JitoConfig {
    pub base_url: String,
    pub auth_header: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub skip_preflight: bool,
    pub commitment: String,
}

impl Default for JitoConfig {
    fn default() -> Self {
        Self {
            base_url: "https://jito-api.mainnet.solana.com".to_string(),
            auth_header: "".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            skip_preflight: false,
            commitment: "confirmed".to_string(),
        }
    }
}

impl JitoService {
    /// Create a new Jito service instance
    pub fn new(config: JitoConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()?;
        
        Ok(Self {
            client,
            base_url: config.base_url,
            auth_header: config.auth_header,
            timeout: config.timeout,
        })
    }
    
    /// Send transaction to Jito
    pub async fn send_transaction(
        &self,
        transaction: &Transaction,
        config: &JitoConfig,
    ) -> Result<JitoTransactionResponse> {
        let transaction_data = base64::encode(&bincode::serialize(transaction)?);
        
        let request = JitoTransactionRequest {
            transaction: transaction_data,
            commitment: config.commitment.clone(),
            skip_preflight: config.skip_preflight,
            max_retries: Some(config.max_retries),
            min_context_slot: None,
        };
        
        let url = format!("{}/v1/transactions", self.base_url);
        
        let mut request_builder = self.client.post(&url)
            .json(&request)
            .timeout(self.timeout);
        
        if !self.auth_header.is_empty() {
            request_builder = request_builder.header("Authorization", &self.auth_header);
        }
        
        let response = request_builder.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Jito API error: {} - {}", status, error_text);
        }
        
        let jito_response: JitoTransactionResponse = response.json().await?;
        Ok(jito_response)
    }
    
    /// Get available block builders
    pub async fn get_block_builders(&self) -> Result<Vec<JitoBlockBuilderInfo>> {
        let url = format!("{}/v1/block-builders", self.base_url);
        
        let mut request_builder = self.client.get(&url).timeout(self.timeout);
        
        if !self.auth_header.is_empty() {
            request_builder = request_builder.header("Authorization", &self.auth_header);
        }
        
        let response = request_builder.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Jito API error: {} - {}", status, error_text);
        }
        
        let builders: Vec<JitoBlockBuilderInfo> = response.json().await?;
        Ok(builders)
    }
    
    /// Get Jito network status
    pub async fn get_network_status(&self) -> Result<serde_json::Value> {
        let url = format!("{}/v1/status", self.base_url);
        
        let mut request_builder = self.client.get(&url).timeout(self.timeout);
        
        if !self.auth_header.is_empty() {
            request_builder = request_builder.header("Authorization", &self.auth_header);
        }
        
        let response = request_builder.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Jito API error: {} - {}", status, error_text);
        }
        
        let status: serde_json::Value = response.json().await?;
        Ok(status)
    }
    
    /// Check whether the transaction is accepted by Jito
    pub async fn check_transaction_status(
        &self,
        signature: &Signature,
    ) -> Result<Option<serde_json::Value>> {
        let url = format!("{}/v1/transactions/{}", self.base_url, signature);
        
        let mut request_builder = self.client.get(&url).timeout(self.timeout);
        
        if !self.auth_header.is_empty() {
            request_builder = request_builder.header("Authorization", &self.auth_header);
        }
        
        let response = request_builder.send().await?;
        
        if response.status().is_success() {
            let status: serde_json::Value = response.json().await?;
            Ok(Some(status))
        } else if response.status().as_u16() == 404 {
            Ok(None) // Transaction not found
        } else {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Jito API error: {} - {}", status, error_text);
        }
    }
    
    /// Get Jito fee info
    pub async fn get_fee_info(&self) -> Result<serde_json::Value> {
        let url = format!("{}/v1/fees", self.base_url);
        
        let mut request_builder = self.client.get(&url).timeout(self.timeout);
        
        if !self.auth_header.is_empty() {
            request_builder = request_builder.header("Authorization", &self.auth_header);
        }
        
        let response = request_builder.send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Jito API error: {} - {}", status, error_text);
        }
        
        let fees: serde_json::Value = response.json().await?;
        Ok(fees)
    }
    
    /// Validate Jito configuration
    pub async fn validate_config(&self) -> Result<bool> {
        match self.get_network_status().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Get service health status
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        
        let response = self.client.get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        
        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// Jito MEV protection manager
pub struct JitoMevProtection {
    jito_service: JitoService,
    config: JitoConfig,
    active_transactions: std::collections::HashMap<Signature, Transaction>,
}

impl JitoMevProtection {
    /// Create a new MEV protection manager
    pub fn new(config: JitoConfig) -> Result<Self> {
        let jito_service = JitoService::new(config.clone())?;
        
        Ok(Self {
            jito_service,
            config,
            active_transactions: std::collections::HashMap::new(),
        })
    }
    
    /// Protect transactions from MEV attacks
    pub async fn protect_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<Signature> {
        // Send transaction to Jito
        let jito_response = self.jito_service.send_transaction(&transaction, &self.config).await?;
        
        let signature = Signature::from_str(&jito_response.signature)?;
        
        // Store active transaction
        self.active_transactions.insert(signature, transaction);
        
        Ok(signature)
    }
    
    /// Check transaction status
    pub async fn check_transaction_status(
        &self,
        signature: &Signature,
    ) -> Result<Option<serde_json::Value>> {
        self.jito_service.check_transaction_status(signature).await
    }
    
    /// Get active transactions
    pub fn get_active_transactions(&self) -> &std::collections::HashMap<Signature, Transaction> {
        &self.active_transactions
    }
    
    /// Cleanup confirmed transactions
    pub async fn cleanup_confirmed_transactions(&mut self) -> Result<()> {
        let mut to_remove = Vec::new();
        
        for (signature, _) in &self.active_transactions {
            if let Ok(Some(status)) = self.check_transaction_status(signature).await {
                // Check whether the transaction is confirmed
                if let Some(confirmations) = status.get("confirmations") {
                    if confirmations.as_u64().unwrap_or(0) > 0 {
                        to_remove.push(*signature);
                    }
                }
            }
        }
        
        for signature in to_remove {
            self.active_transactions.remove(&signature);
        }
        
        Ok(())
    }
    
    /// Get MEV protection statistics
    pub fn get_protection_stats(&self) -> MevProtectionStats {
        MevProtectionStats {
            total_transactions: self.active_transactions.len(),
            active_transactions: self.active_transactions.len(),
            protected_transactions: self.active_transactions.len(),
        }
    }
    
    /// Update configuration
    pub fn update_config(&mut self, new_config: JitoConfig) -> Result<()> {
        self.config = new_config.clone();
        self.jito_service = JitoService::new(new_config)?;
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &JitoConfig {
        &self.config
    }
}

/// MEV protection statistics
#[derive(Debug, Clone)]
pub struct MevProtectionStats {
    pub total_transactions: usize,
    pub active_transactions: usize,
    pub protected_transactions: usize,
}

impl MevProtectionStats {
    /// Calculate protection success rate
    pub fn protection_success_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            self.protected_transactions as f64 / self.total_transactions as f64
        }
    }
    
    /// Calculate active transaction rate
    pub fn active_transaction_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            self.active_transactions as f64 / self.total_transactions as f64
        }
    }
}

/// Jito MEV protection strategy
#[derive(Debug, Clone)]
pub enum MevProtectionStrategy {
    /// Always use Jito protection
    Always,
    /// Decide based on transaction value
    ValueBased { min_value_sol: f64 },
    /// Decide based on network congestion
    CongestionBased { max_fee_multiplier: f64 },
    /// Hybrid strategy
    Hybrid { min_value_sol: f64, max_fee_multiplier: f64 },
}

impl MevProtectionStrategy {
    /// Determine whether to protect the transaction
    pub fn should_protect(
        &self,
        transaction_value_sol: f64,
        current_fee_multiplier: f64,
    ) -> bool {
        match self {
            MevProtectionStrategy::Always => true,
            MevProtectionStrategy::ValueBased { min_value_sol } => {
                transaction_value_sol >= *min_value_sol
            }
            MevProtectionStrategy::CongestionBased { max_fee_multiplier } => {
                current_fee_multiplier <= *max_fee_multiplier
            }
            MevProtectionStrategy::Hybrid { min_value_sol, max_fee_multiplier } => {
                transaction_value_sol >= *min_value_sol && current_fee_multiplier <= *max_fee_multiplier
            }
        }
    }
}

/// Jito MEV protection config builder
pub struct JitoConfigBuilder {
    config: JitoConfig,
}

impl JitoConfigBuilder {
    /// Create a new config builder
    pub fn new() -> Self {
        Self {
            config: JitoConfig::default(),
        }
    }
    
    /// Set base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.config.base_url = base_url;
        self
    }
    
    /// Set auth header
    pub fn with_auth_header(mut self, auth_header: String) -> Self {
        self.config.auth_header = auth_header;
        self
    }
    
    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }
    
    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }
    
    /// Set whether to skip preflight
    pub fn with_skip_preflight(mut self, skip_preflight: bool) -> Self {
        self.config.skip_preflight = skip_preflight;
        self
    }
    
    /// Set commitment level
    pub fn with_commitment(mut self, commitment: String) -> Self {
        self.config.commitment = commitment;
        self
    }
    
    /// Build configuration
    pub fn build(self) -> JitoConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mev_protection_strategy() {
        let strategy = MevProtectionStrategy::ValueBased { min_value_sol: 1.0 };
        assert!(strategy.should_protect(2.0, 1.5));
        assert!(!strategy.should_protect(0.5, 1.5));
    }
    
    #[test]
    fn test_jito_config_builder() {
        let config = JitoConfigBuilder::new()
            .with_base_url("https://test.com".to_string())
            .with_timeout(Duration::from_secs(60))
            .build();
        
        assert_eq!(config.base_url, "https://test.com");
        assert_eq!(config.timeout, Duration::from_secs(60));
    }
}
