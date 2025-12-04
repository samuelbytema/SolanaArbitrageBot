use async_trait::async_trait;
use anyhow::Result;
use rust_decimal::Decimal;
use solana_program::pubkey::Pubkey;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};

use crate::{
    dex::{DexInterface, DexError, DexMetrics, DexConnectionConfig, PoolUpdateStream, PoolUpdate, DexType},
    models::{Token, Pool, PoolQuote, PoolState, PoolMetrics},
};

pub struct WhirlpoolDex {
    config: DexConnectionConfig,
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct WhirlpoolPool {
    id: String,
    base_mint: String,
    quote_mint: String,
    base_decimals: u8,
    quote_decimals: u8,
    base_reserve: String,
    quote_reserve: String,
    fee_rate: String,
    pool_address: String,
    authority: String,
    program_id: String,
}

impl WhirlpoolDex {
    pub fn new(config: DexConnectionConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        let base_url = config.base_url.clone();
        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    async fn make_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.client.get(&url).send()
        ).await??;

        if !response.status().is_success() {
            let status = response.status();
            return Err(DexError::InvalidResponse(
                format!("HTTP {}: {}", status, response.text().await?)
            ).into());
        }

        let data: T = response.json().await?;
        Ok(data)
    }

    fn parse_pool(&self, whirlpool_pool: &WhirlpoolPool) -> Result<Pool> {
        let base_mint = whirlpool_pool.base_mint.parse::<Pubkey>()?;
        let quote_mint = whirlpool_pool.quote_mint.parse::<Pubkey>()?;
        let pool_address = whirlpool_pool.pool_address.parse::<Pubkey>()?;
        let authority = whirlpool_pool.authority.parse::<Pubkey>()?;
        let program_id = whirlpool_pool.program_id.parse::<Pubkey>()?;

        let base_token = Token::new(
            base_mint,
            "BASE".to_string(),
            "Base Token".to_string(),
            whirlpool_pool.base_decimals,
        );

        let quote_token = Token::new(
            quote_mint,
            "QUOTE".to_string(),
            "Quote Token".to_string(),
            whirlpool_pool.quote_decimals,
        );

        let reserve_a = whirlpool_pool.base_reserve.parse::<Decimal>()?;
        let reserve_b = whirlpool_pool.quote_reserve.parse::<Decimal>()?;
        let fee_rate = whirlpool_pool.fee_rate.parse::<Decimal>()?;

        Ok(Pool::new(
            whirlpool_pool.id.clone(),
            DexType::Whirlpool,
            base_token,
            quote_token,
            pool_address,
            authority,
            program_id,
        ).update_reserves(reserve_a, reserve_b).with_fee_rate(fee_rate))
    }
}

#[async_trait]
impl DexInterface for WhirlpoolDex {
    fn get_dex_type(&self) -> DexType {
        DexType::Whirlpool
    }

    fn get_name(&self) -> &str {
        "Whirlpool"
    }

    fn get_version(&self) -> &str {
        "1.0.0"
    }

    async fn is_connected(&self) -> Result<bool> {
        match self.make_request::<serde_json::Value>("/health").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_pools(&self) -> Result<Vec<Pool>> {
        let pools: Vec<WhirlpoolPool> = self.make_request("/pools").await?;
        let mut result = Vec::new();
        
        for pool in pools {
            match self.parse_pool(&pool) {
                Ok(parsed_pool) => result.push(parsed_pool),
                Err(e) => tracing::warn!("Failed to parse Whirlpool pool: {}", e),
            }
        }
        
        Ok(result)
    }

    async fn get_pools_by_tokens(&self, token_a: &Token, token_b: &Token) -> Result<Vec<Pool>> {
        let all_pools = self.get_pools().await?;
        let filtered_pools: Vec<Pool> = all_pools
            .into_iter()
            .filter(|pool| {
                (pool.token_a.mint == token_a.mint && pool.token_b.mint == token_b.mint)
                    || (pool.token_a.mint == token_b.mint && pool.token_b.mint == token_a.mint)
            })
            .collect();
        
        Ok(filtered_pools)
    }

    async fn get_pool_state(&self, pool_address: &Pubkey) -> Result<PoolState> {
        let endpoint = format!("/pool/{}", pool_address);
        let pool_data: WhirlpoolPool = self.make_request(&endpoint).await?;
        let pool = self.parse_pool(&pool_data)?;
        
        let current_price = pool.get_price(&pool.token_a).unwrap_or(Decimal::ZERO);
        let price_impact = Decimal::ZERO;
        
        let tvl = pool.reserve_a + pool.reserve_b;
        Ok(PoolState {
            pool,
            current_price,
            price_impact,
            volume_24h: Decimal::ZERO,
            tvl,
            apy: None,
        })
    }

    async fn get_token_price(&self, token: &Token, quote_token: &Token) -> Result<Decimal> {
        let pools = self.get_pools_by_tokens(token, quote_token).await?;
        if pools.is_empty() {
            return Err(DexError::PoolNotFound("No pools found for token pair".to_string()).into());
        }
        
        let pool = &pools[0];
        pool.get_price(token).ok_or_else(|| {
            DexError::InsufficientLiquidity("Cannot calculate price from pool".to_string()).into()
        })
    }

    async fn get_quote(
        &self,
        input_token: &Token,
        output_token: &Token,
        input_amount: Decimal,
        pool_address: Option<&Pubkey>,
    ) -> Result<PoolQuote> {
        let pools = if let Some(addr) = pool_address {
            vec![self.get_pool_state(addr).await?.pool]
        } else {
            self.get_pools_by_tokens(input_token, output_token).await?
        };

        if pools.is_empty() {
            return Err(DexError::PoolNotFound("No pools found for token pair".to_string()).into());
        }

        let pool = &pools[0];
        let output_amount = pool.calculate_output_amount(input_amount, input_token)
            .ok_or_else(|| DexError::InsufficientLiquidity("Cannot calculate output amount".to_string()))?;
        
        let fee_amount = input_amount * pool.fee_rate;
        let price_impact = pool.calculate_price_impact(input_amount, input_token)
            .unwrap_or(Decimal::ZERO);
        
        let minimum_output = output_amount * (Decimal::ONE - Decimal::from(5) / Decimal::from(1000));

        Ok(PoolQuote {
            pool: pool.clone(),
            input_token: input_token.clone(),
            output_token: output_token.clone(),
            input_amount,
            output_amount,
            price_impact,
            fee_amount,
            minimum_output,
            route: pools,
        })
    }

    async fn execute_swap(
        &self,
        quote: &PoolQuote,
        wallet: &Pubkey,
        slippage_tolerance: Decimal,
    ) -> Result<String> {
        tracing::info!("Executing Whirlpool swap for wallet: {}", wallet);
        Ok("mock_transaction_signature".to_string())
    }

    async fn get_pool_metrics(&self, pool_address: &Pubkey) -> Result<PoolMetrics> {
        let pool_state = self.get_pool_state(pool_address).await?;
        
        Ok(PoolMetrics {
            pool_id: pool_state.pool.id.clone(),
            dex_type: DexType::Whirlpool,
            volume_24h: pool_state.volume_24h,
            volume_7d: Decimal::ZERO,
            tvl: pool_state.tvl,
            fee_revenue_24h: Decimal::ZERO,
            unique_traders_24h: 0,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn get_dex_metrics(&self) -> Result<DexMetrics> {
        let pools = self.get_pools().await?;
        let total_tvl: Decimal = pools.iter().map(|p| p.reserve_a + p.reserve_b).sum();
        
        Ok(DexMetrics {
            total_volume_24h: Decimal::ZERO,
            total_tvl,
            total_pools: pools.len() as u64,
            active_pools: pools.iter().filter(|p| p.is_active).count() as u64,
            total_trades_24h: 0,
            average_gas_price: Decimal::ZERO,
        })
    }

    async fn subscribe_pool_updates(&self, pool_address: &Pubkey) -> Result<PoolUpdateStream> {
        let (_, receiver) = tokio::sync::mpsc::channel(100);
        
        Ok(PoolUpdateStream {
            pool_address: *pool_address,
            update_receiver: receiver,
        })
    }

    async fn get_supported_tokens(&self) -> Result<Vec<Token>> {
        Ok(Vec::new())
    }

    async fn validate_transaction(&self, transaction_data: &[u8]) -> Result<bool> {
        Ok(true)
    }
}
