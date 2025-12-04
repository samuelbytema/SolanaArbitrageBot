use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Token {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_uri: Option<String>,
    pub coingecko_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub token: Token,
    pub price_usd: Decimal,
    pub price_sol: Decimal,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PriceSource {
    Coingecko,
    Jupiter,
    Pyth,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub token: Token,
    pub balance: Decimal,
    pub owner: Pubkey,
    pub associated_token_account: Pubkey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub mint: Pubkey,
    pub metadata: HashMap<String, serde_json::Value>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Token {
    pub fn new(mint: Pubkey, symbol: String, name: String, decimals: u8) -> Self {
        Self {
            mint,
            symbol,
            name,
            decimals,
            logo_uri: None,
            coingecko_id: None,
        }
    }

    pub fn with_logo(mut self, logo_uri: String) -> Self {
        self.logo_uri = Some(logo_uri);
        self
    }

    pub fn with_coingecko_id(mut self, coingecko_id: String) -> Self {
        self.coingecko_id = Some(coingecko_id);
        self
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.symbol, self.mint)
    }
}
