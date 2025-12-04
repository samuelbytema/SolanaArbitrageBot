use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use rust_decimal::Decimal;
use crate::models::token::Token;
use crate::dex::DexType;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub id: String,
    pub dex_type: DexType,
    pub token_a: Token,
    pub token_b: Token,
    pub reserve_a: Decimal,
    pub reserve_b: Decimal,
    pub fee_rate: Decimal,
    pub pool_address: Pubkey,
    pub authority: Pubkey,
    pub program_id: Pubkey,
    pub version: String,
    pub is_active: bool,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub pool: Pool,
    pub current_price: Decimal,
    pub price_impact: Decimal,
    pub volume_24h: Decimal,
    pub tvl: Decimal,
    pub apy: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolQuote {
    pub pool: Pool,
    pub input_token: Token,
    pub output_token: Token,
    pub input_amount: Decimal,
    pub output_amount: Decimal,
    pub price_impact: Decimal,
    pub fee_amount: Decimal,
    pub minimum_output: Decimal,
    pub route: Vec<Pool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetrics {
    pub pool_id: String,
    pub dex_type: DexType,
    pub volume_24h: Decimal,
    pub volume_7d: Decimal,
    pub tvl: Decimal,
    pub fee_revenue_24h: Decimal,
    pub unique_traders_24h: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub dex_type: DexType,
    pub program_id: Pubkey,
    pub authority: Pubkey,
    pub fee_rate: Decimal,
    pub min_liquidity: Decimal,
    pub max_slippage: Decimal,
}

impl Pool {
    pub fn new(
        id: String,
        dex_type: DexType,
        token_a: Token,
        token_b: Token,
        pool_address: Pubkey,
        authority: Pubkey,
        program_id: Pubkey,
    ) -> Self {
        Self {
            id,
            dex_type,
            token_a,
            token_b,
            reserve_a: Decimal::ZERO,
            reserve_b: Decimal::ZERO,
            fee_rate: Decimal::ZERO,
            pool_address,
            authority,
            program_id,
            version: "1.0".to_string(),
            is_active: true,
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn update_reserves(mut self, reserve_a: Decimal, reserve_b: Decimal) -> Self {
        self.reserve_a = reserve_a;
        self.reserve_b = reserve_b;
        self.last_updated = chrono::Utc::now();
        self
    }

    pub fn with_fee_rate(mut self, fee_rate: Decimal) -> Self {
        self.fee_rate = fee_rate;
        self
    }

    pub fn get_price(&self, base_token: &Token) -> Option<Decimal> {
        if base_token.mint == self.token_a.mint {
            if self.reserve_b > Decimal::ZERO {
                Some(self.reserve_a / self.reserve_b)
            } else {
                None
            }
        } else if base_token.mint == self.token_b.mint {
            if self.reserve_a > Decimal::ZERO {
                Some(self.reserve_b / self.reserve_a)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn calculate_output_amount(
        &self,
        input_amount: Decimal,
        input_token: &Token,
    ) -> Option<Decimal> {
        let (input_reserve, output_reserve) = if input_token.mint == self.token_a.mint {
            (self.reserve_a, self.reserve_b)
        } else if input_token.mint == self.token_b.mint {
            (self.reserve_b, self.reserve_a)
        } else {
            return None;
        };

        if input_reserve <= Decimal::ZERO || output_reserve <= Decimal::ZERO {
            return None;
        }

        let fee_multiplier = Decimal::ONE - self.fee_rate;
        let input_with_fee = input_amount * fee_multiplier;
        let numerator = input_with_fee * output_reserve;
        let denominator = input_reserve + input_with_fee;

        if denominator > Decimal::ZERO {
            Some(numerator / denominator)
        } else {
            None
        }
    }

    pub fn calculate_price_impact(&self, input_amount: Decimal, input_token: &Token) -> Option<Decimal> {
        let price_before = self.get_price(input_token)?;
        let output_amount = self.calculate_output_amount(input_amount, input_token)?;
        
        // Calculate new reserves after swap
        let (input_reserve, output_reserve) = if input_token.mint == self.token_a.mint {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        };

        let new_input_reserve = input_reserve + input_amount;
        let new_output_reserve = output_reserve - output_amount;
        
        if new_output_reserve <= Decimal::ZERO {
            return None;
        }

        let price_after = new_input_reserve / new_output_reserve;
        let price_change = (price_after - price_before) / price_before;
        
        Some(price_change.abs())
    }
}

impl std::fmt::Display for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}/{} on {}",
            self.id, self.token_a.symbol, self.token_b.symbol, self.dex_type
        )
    }
}
