use serde::{Deserialize, Serialize};
use std::time::Duration;
use anyhow::Result;
use config::{Config, Environment, File};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub memory_store: MemoryStoreConfig,
    pub solana: SolanaConfig,
    pub dex: DexConfig,
    pub arbitrage: ArbitrageConfig,
    pub logging: LoggingConfig,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreConfig {
    pub enabled: bool,
    pub max_opportunities: usize,
    pub max_executions: usize,
    pub cleanup_interval_seconds: u64,
    pub data_retention_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub commitment: String,
    pub jito_url: String,
    pub jito_auth_header: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    pub raydium: DexEndpointConfig,
    pub meteora: DexEndpointConfig,
    pub whirlpool: DexEndpointConfig,
    pub pump: DexEndpointConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexEndpointConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_seconds: u64,
    pub rate_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    pub min_profit_threshold: f64,
    pub max_slippage: f64,
    pub gas_price_multiplier: f64,
    pub max_concurrent_opportunities: usize,
    pub execution_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: String,
    pub max_file_size: u64,
    pub max_files: u32,
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::File::with_name("config/local").required(false))
            .add_source(config::Environment::with_prefix("ARBITRAGE_BOT"))
            .build()?;

        settings.try_deserialize()
    }

    pub fn validate(&self) -> Result<()> {
        // Validate required fields
        if self.solana.rpc_url.is_empty() {
            anyhow::bail!("Solana RPC URL is required");
        }
        if self.solana.jito_url.is_empty() {
            anyhow::bail!("Jito URL is required");
        }
        if self.arbitrage.min_profit_threshold <= 0.0 {
            anyhow::bail!("Min profit threshold must be positive");
        }
        Ok(())
    }

    pub fn get_memory_store_config(&self) -> MemoryStoreConfig {
        self.memory_store.clone()
    }

    pub fn is_memory_store_enabled(&self) -> bool {
        self.memory_store.enabled
    }
}

impl Default for MemoryStoreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_opportunities: 10000,
            max_executions: 50000,
            cleanup_interval_seconds: 300, // 5 minutes
            data_retention_days: 7,
        }
    }
}
