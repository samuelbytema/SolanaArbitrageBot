use anyhow::Result;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

use crate::{
    config::AppConfig,
    dex::{DexInterface, DexType},
    models::{ArbitrageOpportunity, Token, Pool, RiskScore},
};

pub struct OpportunityScanner {
    dex_instances: Arc<HashMap<DexType, Box<dyn DexInterface>>>,
    opportunity_sender: mpsc::Sender<ArbitrageOpportunity>,
    config: AppConfig,
    scan_interval: Duration,
}

impl OpportunityScanner {
    pub fn new(
        dex_instances: Arc<HashMap<DexType, Box<dyn DexInterface>>>,
        opportunity_sender: mpsc::Sender<ArbitrageOpportunity>,
        config: AppConfig,
    ) -> Self {
        Self {
            dex_instances,
            opportunity_sender,
            config,
            scan_interval: Duration::from_secs(5), // Scan every 5 seconds
        }
    }

    /// Start the scanner
    pub async fn start(mut self) -> Result<()> {
        info!("Starting opportunity scanner...");
        
        loop {
            if let Err(e) = self.scan_opportunities().await {
                error!("Error scanning opportunities: {}", e);
            }
            
            sleep(self.scan_interval).await;
        }
    }

    /// Scan for arbitrage opportunities
    async fn scan_opportunities(&mut self) -> Result<()> {
        let mut all_pools = HashMap::new();
        
        // Fetch pools from all DEXes
        for (dex_type, dex_instance) in self.dex_instances.iter() {
            match dex_instance.get_pools().await {
                Ok(pools) => {
                    all_pools.insert(dex_type.clone(), pools.clone());
                    info!("Retrieved {} pools from {}", pools.len(), dex_instance.get_name());
                }
                Err(e) => {
                    warn!("Failed to get pools from {}: {}", dex_instance.get_name(), e);
                }
            }
        }
        
        // Find arbitrage opportunities
        let opportunities = self.find_arbitrage_opportunities(&all_pools).await?;
        
        // Send arbitrage opportunities
        for opportunity in opportunities {
            if let Err(e) = self.opportunity_sender.send(opportunity).await {
                error!("Failed to send opportunity: {}", e);
            }
        }
        
        Ok(())
    }

    /// Find arbitrage opportunities
    async fn find_arbitrage_opportunities(
        &self,
        all_pools: &HashMap<DexType, Vec<Pool>>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get all token pairs
        let token_pairs = self.get_token_pairs(all_pools);
        
        for (token_a, token_b) in token_pairs {
            let pools_for_pair = self.get_pools_for_token_pair(all_pools, &token_a, &token_b);
            
            if pools_for_pair.len() < 2 {
                continue; // Need at least two pools for arbitrage
            }
            
            // Calculate price differences
            let price_differences = self.calculate_price_differences(&pools_for_pair, &token_a, &token_b);
            
            // Filter profitable opportunities
            for (buy_pool, sell_pool, _price_diff, profit_percentage) in price_differences {
                if profit_percentage >= Decimal::try_from(self.config.arbitrage.min_profit_threshold).unwrap_or(Decimal::ZERO) {
                    let opportunity = ArbitrageOpportunity::new(
                        token_a.clone(),
                        token_b.clone(),
                        buy_pool.clone(),
                        sell_pool.clone(),
                    );
                    
                    opportunities.push(opportunity);
                }
            }
        }
        
        info!("Found {} arbitrage opportunities", opportunities.len());
        Ok(opportunities)
    }

    /// Get all token pairs
    fn get_token_pairs(&self, all_pools: &HashMap<DexType, Vec<Pool>>) -> Vec<(Token, Token)> {
        let mut token_pairs = std::collections::HashSet::new();
        
        for pools in all_pools.values() {
            for pool in pools {
                let pair = if pool.token_a.mint < pool.token_b.mint {
                    (pool.token_a.clone(), pool.token_b.clone())
                } else {
                    (pool.token_b.clone(), pool.token_a.clone())
                };
                token_pairs.insert(pair);
            }
        }
        
        token_pairs.into_iter().collect()
    }

    /// Get pools for a specific token pair
    fn get_pools_for_token_pair(
        &self,
        all_pools: &HashMap<DexType, Vec<Pool>>,
        token_a: &Token,
        _token_b: &Token,
    ) -> Vec<Pool> {
        let mut pools_for_pair = Vec::new();
        
        for pools in all_pools.values() {
            for pool in pools {
                if (pool.token_a.mint == token_a.mint && pool.token_b.mint == _token_b.mint)
                    || (pool.token_a.mint == _token_b.mint && pool.token_b.mint == token_a.mint)
                {
                    pools_for_pair.push(pool.clone());
                }
            }
        }
        
        pools_for_pair
    }

    /// Calculate price differences
    fn calculate_price_differences(
        &self,
        pools: &[Pool],
        token_a: &Token,
        token_b: &Token,
    ) -> Vec<(Pool, Pool, Decimal, Decimal)> {
        let mut price_differences = Vec::new();
        
        for i in 0..pools.len() {
            for j in i + 1..pools.len() {
                let pool_a = &pools[i];
                let pool_b = &pools[j];
                
                let price_a = pool_a.get_price(token_a);
                let price_b = pool_b.get_price(token_a);
                
                if let (Some(price_a), Some(price_b)) = (price_a, price_b) {
                    let price_diff = if price_a > price_b {
                        price_a - price_b
                    } else {
                        price_b - price_a
                    };
                    
                    let profit_percentage = if price_a > price_b {
                        price_diff / price_b
                    } else {
                        price_diff / price_a
                    };
                    
                    let (buy_pool, sell_pool) = if price_a < price_b {
                        (pool_a.clone(), pool_b.clone())
                    } else {
                        (pool_b.clone(), pool_a.clone())
                    };
                    
                    price_differences.push((buy_pool, sell_pool, price_diff, profit_percentage));
                }
            }
        }
        
        // Sort by profit percentage
        price_differences.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        
        price_differences
    }

    /// Validate whether an arbitrage opportunity is feasible
    fn validate_opportunity(&self, opportunity: &ArbitrageOpportunity) -> bool {
        // Check minimum profit threshold
        if opportunity.profit_percentage < Decimal::try_from(self.config.arbitrage.min_profit_threshold).unwrap_or(Decimal::ZERO) {
            return false;
        }
        
        // Check liquidity
        let min_liquidity = Decimal::from(1000); // Minimum liquidity requirement
        if opportunity.buy_pool.reserve_a < min_liquidity
            || opportunity.buy_pool.reserve_b < min_liquidity
            || opportunity.sell_pool.reserve_a < min_liquidity
            || opportunity.sell_pool.reserve_b < min_liquidity
        {
            return false;
        }
        
        // Check risk score
        if opportunity.risk_score == RiskScore::Critical {
            return false;
        }
        
        true
    }

    /// Calculate optimal trade amount
    fn calculate_optimal_amount(
        &self,
        buy_pool: &Pool,
        sell_pool: &Pool,
        token_a: &Token,
    ) -> Option<Decimal> {
        let buy_price = buy_pool.get_price(token_a)?;
        let sell_price = sell_pool.get_price(token_a)?;
        
        if sell_price <= buy_price {
            return None; // No arbitrage opportunity
        }
        
        // Simple arbitrage amount calculation
        // Real implementations need to consider additional factors
        let max_amount = std::cmp::min(
            buy_pool.reserve_a,
            sell_pool.reserve_b,
        );
        
        // Cap the maximum trade amount
        let max_trade_amount = Decimal::from(10000); // Maximum trade amount
        
        Some(std::cmp::min(max_amount, max_trade_amount))
    }
}
