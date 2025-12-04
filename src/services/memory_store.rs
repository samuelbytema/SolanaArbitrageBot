use anyhow::Result;
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::{
    ArbitrageOpportunity, ArbitrageStrategy, ArbitrageExecution,
    OpportunityStatus, ExecutionStatus, RiskScore
};
use crate::dex::DexType;

/// High-performance in-memory storage service optimized for high-frequency trading
pub struct MemoryStore {
    // Use RwLock to separate reads/writes and improve concurrency
    opportunities: Arc<RwLock<HashMap<String, ArbitrageOpportunity>>>,
    strategies: Arc<RwLock<HashMap<String, ArbitrageStrategy>>>,
    executions: Arc<RwLock<VecDeque<ArbitrageExecution>>>,
    
    // Use Mutex to protect metrics and configuration
    metrics: Arc<Mutex<StoreMetrics>>,
    
    // Configuration parameters
    max_opportunities: usize,
    max_executions: usize,
    cleanup_interval: std::time::Duration,
}

/// Storage metrics
#[derive(Debug, Clone)]
struct StoreMetrics {
    total_opportunities: u64,
    total_executions: u64,
    successful_executions: u64,
    total_profit: Decimal,
    total_fees: Decimal,
    last_cleanup: DateTime<Utc>,
}

impl Default for StoreMetrics {
    fn default() -> Self {
        Self {
            total_opportunities: 0,
            total_executions: 0,
            successful_executions: 0,
            total_profit: Decimal::ZERO,
            total_fees: Decimal::ZERO,
            last_cleanup: Utc::now(),
        }
    }
}

impl MemoryStore {
    /// Create a new memory store instance
    pub fn new(max_opportunities: usize, max_executions: usize) -> Self {
        let store = Self {
            opportunities: Arc::new(RwLock::new(HashMap::new())),
            strategies: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(VecDeque::new())),
            metrics: Arc::new(Mutex::new(StoreMetrics::default())),
            max_opportunities,
            max_executions,
            cleanup_interval: std::time::Duration::from_secs(300), // Clean every 5 minutes
        };

        // Start background cleanup task
        let store_clone = store.clone();
        tokio::spawn(async move {
            store_clone.background_cleanup().await;
        });

        store
    }

    /// Save an arbitrage opportunity
    pub async fn save_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        let mut opportunities = self.opportunities.write().await;
        
        // If capacity is reached, remove the oldest opportunity
        if opportunities.len() >= self.max_opportunities {
            let oldest_key = opportunities
                .iter()
                .min_by_key(|(_, opp)| opp.timestamp)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = oldest_key {
                opportunities.remove(&key);
            }
        }
        
        opportunities.insert(opportunity.id.clone(), opportunity.clone());
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.total_opportunities += 1;
        
        Ok(())
    }

    /// Update an arbitrage opportunity status
    pub async fn update_opportunity_status(&self, opportunity_id: &str, status: OpportunityStatus) -> Result<()> {
        let mut opportunities = self.opportunities.write().await;
        
        if let Some(opportunity) = opportunities.get_mut(opportunity_id) {
            opportunity.status = status;
        }
        
        Ok(())
    }

    /// Get an arbitrage opportunity
    pub async fn get_opportunity(&self, opportunity_id: &str) -> Option<ArbitrageOpportunity> {
        let opportunities = self.opportunities.read().await;
        opportunities.get(opportunity_id).cloned()
    }

    /// Get all active opportunities
    pub async fn get_active_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let opportunities = self.opportunities.read().await;
        opportunities
            .values()
            .filter(|opp| opp.status == OpportunityStatus::Pending)
            .cloned()
            .collect()
    }

    /// Get opportunities by status
    pub async fn get_opportunities_by_status(&self, status: OpportunityStatus) -> Vec<ArbitrageOpportunity> {
        let opportunities = self.opportunities.read().await;
        opportunities
            .values()
            .filter(|opp| opp.status == status)
            .cloned()
            .collect()
    }

    /// Save an arbitrage strategy
    pub async fn save_strategy(&self, strategy: &ArbitrageStrategy) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy.id.clone(), strategy.clone());
        Ok(())
    }

    /// Update a strategy
    pub async fn update_strategy(&self, strategy: &ArbitrageStrategy) -> Result<()> {
        self.save_strategy(strategy).await
    }

    /// Delete a strategy
    pub async fn delete_strategy(&self, strategy_id: &str) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        strategies.remove(strategy_id);
        Ok(())
    }

    /// Get all strategies
    pub async fn get_strategies(&self) -> Vec<ArbitrageStrategy> {
        let strategies = self.strategies.read().await;
        strategies.values().cloned().collect()
    }

    /// Save an execution result
    pub async fn save_execution(&self, execution: &ArbitrageExecution) -> Result<()> {
        let mut executions = self.executions.write().await;
        
        // If capacity is reached, remove the oldest execution record
        if executions.len() >= self.max_executions {
            executions.pop_front();
        }
        
        executions.push_back(execution.clone());
        
        // Update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.total_executions += 1;
        
        if execution.execution_status == ExecutionStatus::Confirmed {
            metrics.successful_executions += 1;
        }
        
        if let Some(profit) = execution.actual_profit {
            metrics.total_profit += profit;
        }
        
        if let Some(fees) = execution.total_cost {
            metrics.total_fees += fees;
        }
        
        Ok(())
    }

    /// Get executions by status
    pub async fn get_executions_by_status(&self, status: ExecutionStatus) -> Vec<ArbitrageExecution> {
        let executions = self.executions.read().await;
        executions
            .iter()
            .filter(|exec| exec.execution_status == status)
            .cloned()
            .collect()
    }

    /// Get execution statistics
    pub async fn get_execution_stats(&self, days: i64) -> Result<(u64, Decimal, Decimal)> {
        let since = Utc::now() - chrono::Duration::days(days);
        let executions = self.executions.read().await;
        
        let filtered_executions: Vec<&ArbitrageExecution> = executions
            .iter()
            .filter(|exec| exec.execution_time >= since)
            .collect();
        
        let total_executions = filtered_executions.len() as u64;
        let total_profit: Decimal = filtered_executions
            .iter()
            .filter_map(|exec| exec.actual_profit)
            .sum();
        let total_fees: Decimal = filtered_executions
            .iter()
            .filter_map(|exec| exec.total_cost)
            .sum();
        
        Ok((total_executions, total_profit, total_fees))
    }

    /// Get storage metrics
    pub async fn get_metrics(&self) -> StoreMetrics {
        let metrics = self.metrics.lock().await;
        metrics.clone()
    }

    /// Cleanup expired data
    async fn cleanup_expired_data(&self) -> Result<()> {
        let now = Utc::now();
        
        // Cleanup expired opportunities
        let mut opportunities = self.opportunities.write().await;
        let expired_opportunities: Vec<String> = opportunities
            .iter()
            .filter(|(_, opp)| opp.expiry < now)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired_opportunities {
            if let Some(mut opportunity) = opportunities.remove(&id) {
                opportunity.status = OpportunityStatus::Expired;
                opportunities.insert(id, opportunity);
            }
        }
        
        // Cleanup expired executions (keep last 7 days)
        let cutoff = now - chrono::Duration::days(7);
        let mut executions = self.executions.write().await;
        executions.retain(|exec| exec.execution_time >= cutoff);
        
        // Update cleanup time
        let mut metrics = self.metrics.lock().await;
        metrics.last_cleanup = now;
        
        Ok(())
    }

    /// Background cleanup task
    async fn background_cleanup(&self) {
        let mut interval = tokio::time::interval(self.cleanup_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.cleanup_expired_data().await {
                tracing::warn!("Background cleanup failed: {}", e);
            }
        }
    }

    /// Get storage usage
    pub async fn get_storage_usage(&self) -> StorageUsage {
        let opportunities = self.opportunities.read().await;
        let strategies = self.strategies.read().await;
        let executions = self.executions.read().await;
        
        StorageUsage {
            opportunities_count: opportunities.len(),
            strategies_count: strategies.len(),
            executions_count: executions.len(),
            max_opportunities: self.max_opportunities,
            max_executions: self.max_executions,
        }
    }

    /// Batch save opportunities (optimized for bulk operations)
    pub async fn batch_save_opportunities(&self, opportunities: Vec<ArbitrageOpportunity>) -> Result<()> {
        let mut opps = self.opportunities.write().await;
        let opportunities_len = opportunities.len();
        
        for opportunity in opportunities {
            // If capacity is reached, remove the oldest opportunity
            if opps.len() >= self.max_opportunities {
                let oldest_key = opps
                    .iter()
                    .min_by_key(|(_, opp)| opp.timestamp)
                    .map(|(k, _)| k.clone());
                
                if let Some(key) = oldest_key {
                    opps.remove(&key);
                }
            }
            
            opps.insert(opportunity.id.clone(), opportunity);
        }
        
        // Bulk update metrics
        let mut metrics = self.metrics.lock().await;
        metrics.total_opportunities += opportunities_len as u64;
        
        Ok(())
    }

    /// Search opportunities (supports fast filtering)
    pub async fn search_opportunities(
        &self,
        min_profit: Option<Decimal>,
        max_risk: Option<RiskScore>,
        dex_types: Option<Vec<DexType>>,
    ) -> Vec<ArbitrageOpportunity> {
        let opportunities = self.opportunities.read().await;
        
        opportunities
            .values()
            .filter(|opp| {
                // Profit filter
                if let Some(min_profit_threshold) = min_profit {
                    if opp.net_profit < min_profit_threshold {
                        return false;
                    }
                }
                
                // Risk filter
                if let Some(max_risk_threshold) = &max_risk {
                    if opp.risk_score > *max_risk_threshold {
                        return false;
                    }
                }
                
                // DEX type filter
                if let Some(allowed_dexes) = &dex_types {
                    if !allowed_dexes.contains(&opp.buy_pool.dex_type) || 
                       !allowed_dexes.contains(&opp.sell_pool.dex_type) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect()
    }
}

/// Storage usage
#[derive(Debug, Clone)]
pub struct StorageUsage {
    pub opportunities_count: usize,
    pub strategies_count: usize,
    pub executions_count: usize,
    pub max_opportunities: usize,
    pub max_executions: usize,
}

impl Clone for MemoryStore {
    fn clone(&self) -> Self {
        Self {
            opportunities: Arc::clone(&self.opportunities),
            strategies: Arc::clone(&self.strategies),
            executions: Arc::clone(&self.executions),
            metrics: Arc::clone(&self.metrics),
            max_opportunities: self.max_opportunities,
            max_executions: self.max_executions,
            cleanup_interval: self.cleanup_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Token, Pool};
    use crate::dex::DexType;

    #[tokio::test]
    async fn test_memory_store_basic_operations() {
        let store = MemoryStore::new(100, 1000);
        
        // Test saving and retrieving opportunity
        let opportunity = create_test_opportunity();
        store.save_opportunity(&opportunity).await.unwrap();
        
        let retrieved = store.get_opportunity(&opportunity.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, opportunity.id);
    }

    #[tokio::test]
    async fn test_memory_store_capacity_limits() {
        let store = MemoryStore::new(2, 3);
        
        // Create 3 opportunities; only 2 should be kept
        for i in 0..3 {
            let mut opp = create_test_opportunity();
            opp.id = format!("opp_{}", i);
            store.save_opportunity(&opp).await.unwrap();
        }
        
        let opportunities = store.get_active_opportunities().await;
        assert_eq!(opportunities.len(), 2);
    }

    fn create_test_opportunity() -> ArbitrageOpportunity {
        // Create a mock Pubkey
        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        let pubkey = solana_program::pubkey::Pubkey::new_from_array(bytes);
        
        ArbitrageOpportunity {
            id: "test_opp".to_string(),
            base_token: Token {
                mint: pubkey,
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
                decimals: 9,
                logo_uri: None,
                coingecko_id: None,
            },
            quote_token: Token {
                mint: pubkey,
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                decimals: 6,
                logo_uri: None,
                coingecko_id: None,
            },
            buy_pool: Pool {
                id: "pool1".to_string(),
                dex_type: DexType::Raydium,
                token_a: Token {
                    mint: pubkey,
                    symbol: "SOL".to_string(),
                    name: "Solana".to_string(),
                    decimals: 9,
                    logo_uri: None,
                    coingecko_id: None,
                },
                token_b: Token {
                    mint: pubkey,
                    symbol: "USDC".to_string(),
                    name: "USD Coin".to_string(),
                    decimals: 6,
                    logo_uri: None,
                    coingecko_id: None,
                },
                reserve_a: Decimal::from(1000000),
                reserve_b: Decimal::from(1000000),
                fee_rate: Decimal::from(25) / Decimal::from(10000),
                pool_address: pubkey,
                authority: pubkey,
                program_id: pubkey,
                version: "1.0".to_string(),
                is_active: true,
                last_updated: Utc::now(),
            },
            sell_pool: Pool {
                id: "pool2".to_string(),
                dex_type: DexType::Meteora,
                token_a: Token {
                    mint: pubkey,
                    symbol: "SOL".to_string(),
                    name: "Solana".to_string(),
                    decimals: 9,
                    logo_uri: None,
                    coingecko_id: None,
                },
                token_b: Token {
                    mint: pubkey,
                    symbol: "USDC".to_string(),
                    name: "USD Coin".to_string(),
                    decimals: 6,
                    logo_uri: None,
                    coingecko_id: None,
                },
                reserve_a: Decimal::from(1000000),
                reserve_b: Decimal::from(1000000),
                fee_rate: Decimal::from(25) / Decimal::from(10000),
                pool_address: pubkey,
                authority: pubkey,
                program_id: pubkey,
                version: "1.0".to_string(),
                is_active: true,
                last_updated: Utc::now(),
            },
            buy_price: Decimal::from(100),
            sell_price: Decimal::from(101),
            price_difference: Decimal::from(1),
            profit_percentage: Decimal::from(1) / Decimal::from(100),
            estimated_profit: Decimal::from(10),
            estimated_fees: Decimal::from(1),
            net_profit: Decimal::from(9),
            risk_score: RiskScore::Low,
            timestamp: Utc::now(),
            expiry: Utc::now() + chrono::Duration::minutes(5),
            status: OpportunityStatus::Pending,
        }
    }
}
