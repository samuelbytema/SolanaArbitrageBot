use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error};

use crate::{
    config::AppConfig,
    dex::{DexInterface, DexType},
    models::{
        ArbitrageOpportunity, ArbitrageStrategy, ArbitrageExecution, 
        ArbitrageMetrics, Token, Pool, RiskScore, ExecutionStatus
    },
    services::{database::DatabaseService, memory_store::{MemoryStore, StorageUsage}},
    arbitrage::{scanner::OpportunityScanner, executor::ArbitrageExecutor},
};

pub struct ArbitrageEngine {
    config: AppConfig,
    database: Option<Arc<DatabaseService>>,
    memory_store: Arc<MemoryStore>,
    strategies: Arc<RwLock<HashMap<String, ArbitrageStrategy>>>,
    active_opportunities: Arc<RwLock<HashMap<String, ArbitrageOpportunity>>>,
    executions: Arc<RwLock<Vec<ArbitrageExecution>>>,
    dex_instances: Arc<HashMap<DexType, Box<dyn DexInterface>>>,
    opportunity_sender: mpsc::Sender<ArbitrageOpportunity>,
    opportunity_receiver: mpsc::Receiver<ArbitrageOpportunity>,
    execution_sender: mpsc::Sender<ArbitrageExecution>,
    execution_receiver: mpsc::Receiver<ArbitrageExecution>,
}

impl ArbitrageEngine {
    pub fn new(
        config: AppConfig,
        database: Option<Arc<DatabaseService>>,
        dex_instances: HashMap<DexType, Box<dyn DexInterface>>,
    ) -> Self {
        let (opportunity_sender, opportunity_receiver) = mpsc::channel(10000); // Increase buffer size
        let (execution_sender, execution_receiver) = mpsc::channel(10000);
        
        // Create memory store instance
        let memory_config = config.get_memory_store_config();
        let memory_store = Arc::new(MemoryStore::new(
            memory_config.max_opportunities,
            memory_config.max_executions,
        ));

        Self {
            config,
            database,
            memory_store,
            strategies: Arc::new(RwLock::new(HashMap::new())),
            active_opportunities: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            dex_instances: Arc::new(dex_instances),
            opportunity_sender,
            opportunity_receiver,
            execution_sender,
            execution_receiver,
        }
    }

    /// Start the arbitrage engine
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting arbitrage engine with memory store...");
        
        // Load strategies
        self.load_strategies().await?;
        
        // Start the opportunity scanner
        self.start_opportunity_scanner().await?;
        
        // Start the executor
        self.start_executor().await?;
        
        // Start the main loop
        self.main_loop().await?;
        
        Ok(())
    }

    /// Load arbitrage strategies
    async fn load_strategies(&self) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        
        // Create default strategy
        let default_strategy = ArbitrageStrategy::new(
            "default".to_string(),
            "Default arbitrage strategy".to_string(),
            Decimal::from_f64(self.config.arbitrage.min_profit_threshold).unwrap_or(Decimal::from(1) / Decimal::from(100)), // Default 1%
            Decimal::from_f64(self.config.arbitrage.max_slippage).unwrap_or(Decimal::from(1) / Decimal::from(100)), // Default 1%
            Decimal::from(5) / Decimal::from(1000), // 0.5% max price impact
            Decimal::from(1000), // Minimum liquidity 1000
            vec![DexType::Raydium, DexType::Meteora, DexType::Whirlpool, DexType::Pump],
            RiskScore::Medium,
        );
        
        strategies.insert(default_strategy.id.clone(), default_strategy);
        
        // Load strategies from memory store
        let memory_strategies = self.memory_store.get_strategies().await;
        for strategy in memory_strategies {
            strategies.insert(strategy.id.clone(), strategy);
        }
        
        // If database is available, load from database as well
        if let Some(ref db) = self.database {
            if let Ok(db_strategies) = db.get_strategies().await {
                for strategy in db_strategies {
                    strategies.insert(strategy.id.clone(), strategy);
                }
            }
        }
        
        info!("Loaded {} strategies", strategies.len());
        Ok(())
    }

    /// Start the opportunity scanner
    async fn start_opportunity_scanner(&self) -> Result<()> {
        let scanner = OpportunityScanner::new(
            self.dex_instances.clone(),
            self.opportunity_sender.clone(),
            self.config.clone(),
        );
        
        tokio::spawn(async move {
            if let Err(e) = scanner.start().await {
                error!("Opportunity scanner failed: {}", e);
            }
        });
        
        Ok(())
    }

    /// Start the executor
    async fn start_executor(&self) -> Result<()> {
        let executor = ArbitrageExecutor::new(
            self.dex_instances.clone(),
            self.execution_sender.clone(),
            self.config.clone(),
        );
        
        tokio::spawn(async move {
            if let Err(e) = executor.start().await {
                error!("Arbitrage executor failed: {}", e);
            }
        });
        
        Ok(())
    }

    /// Main loop
    async fn main_loop(&mut self) -> Result<()> {
        info!("Arbitrage engine main loop started");
        
        loop {
            tokio::select! {
                // Handle new arbitrage opportunities
                opportunity = self.opportunity_receiver.recv() => {
                    if let Some(opportunity) = opportunity {
                        self.process_opportunity(opportunity).await?;
                    }
                }
                
                // Handle execution results
                execution = self.execution_receiver.recv() => {
                    if let Some(execution) = execution {
                        self.process_execution(execution).await?;
                    }
                }
                
                // Periodically cleanup expired opportunities
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => { // Reduce cleanup interval
                    self.cleanup_expired_opportunities().await?;
                }
            }
        }
    }

    /// Process a new arbitrage opportunity
    async fn process_opportunity(&self, opportunity: ArbitrageOpportunity) -> Result<()> {
        // Check whether the opportunity is still valid
        if opportunity.is_expired() {
            return Ok(());
        }

        // Apply strategy filters
        let strategies = self.strategies.read().await;
        let suitable_strategy = strategies.values().find(|s| s.is_opportunity_suitable(&opportunity));
        
        if suitable_strategy.is_none() {
            return Ok(());
        }

        // Check if the same opportunity already exists
        let mut active_opportunities = self.active_opportunities.write().await;
        if active_opportunities.contains_key(&opportunity.id) {
            return Ok(());
        }

        // Validate profitability
        if !opportunity.is_profitable(Decimal::from_f64(self.config.arbitrage.min_profit_threshold).unwrap_or(Decimal::from(1) / Decimal::from(100))) {
            return Ok(());
        }

        // Add to active opportunities
        active_opportunities.insert(opportunity.id.clone(), opportunity.clone());
        
        // Save to memory store (primary storage)
        if let Err(e) = self.memory_store.save_opportunity(&opportunity).await {
            warn!("Failed to save opportunity to memory store: {}", e);
        }
        
        // If database is available, also save to database (backup)
        if let Some(ref db) = self.database {
            if let Err(e) = db.save_opportunity(&opportunity).await {
                warn!("Failed to save opportunity to database: {}", e);
            }
        }

        info!("New arbitrage opportunity: {}", opportunity);
        
        // Send to executor
        if let Err(e) = self.execution_sender.send(ArbitrageExecution::new(opportunity)).await {
            error!("Failed to send opportunity to executor: {}", e);
        }

        Ok(())
    }

    /// Process an execution result
    async fn process_execution(&self, execution: ArbitrageExecution) -> Result<()> {
        // Update active opportunity status
        let mut active_opportunities = self.active_opportunities.write().await;
        if let Some(opportunity) = active_opportunities.get_mut(&execution.opportunity.id) {
            // Update opportunity status based on execution status
            let new_status = match execution.execution_status {
                ExecutionStatus::Confirmed => crate::models::OpportunityStatus::Completed,
                ExecutionStatus::Failed => crate::models::OpportunityStatus::Failed,
                ExecutionStatus::Cancelled => crate::models::OpportunityStatus::Expired,
                _ => crate::models::OpportunityStatus::Pending,
            };
            opportunity.update_status(new_status);
        }

        // Save execution result to memory store (primary storage)
        if let Err(e) = self.memory_store.save_execution(&execution).await {
            warn!("Failed to save execution to memory store: {}", e);
        }
        
        // If database is available, also save to database (backup)
        if let Some(ref db) = self.database {
            if let Err(e) = db.save_execution(&execution).await {
                warn!("Failed to save execution to database: {}", e);
            }
        }

        // Add to execution history
        let mut executions = self.executions.write().await;
        executions.push(execution.clone());

        info!("Execution completed: {} - {:?}", execution.id, execution.execution_status);

        Ok(())
    }

    /// Cleanup expired arbitrage opportunities
    async fn cleanup_expired_opportunities(&self) -> Result<()> {
        let mut active_opportunities = self.active_opportunities.write().await;
        let expired_ids: Vec<String> = active_opportunities
            .iter()
            .filter(|(_, opportunity)| opportunity.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        let expired_count = expired_ids.len();
        for id in &expired_ids {
            if let Some(mut opportunity) = active_opportunities.remove(id) {
                opportunity.update_status(crate::models::OpportunityStatus::Expired);
                
                // Update memory store
                if let Err(e) = self.memory_store.update_opportunity_status(&id, crate::models::OpportunityStatus::Expired).await {
                    warn!("Failed to update expired opportunity status in memory store: {}", e);
                }
                
                // If database is available, update the database as well
                if let Some(ref db) = self.database {
                    if let Err(e) = db.update_opportunity_status(&opportunity).await {
                        warn!("Failed to update expired opportunity status in database: {}", e);
                    }
                }
            }
        }

        if expired_count > 0 {
            info!("Cleaned up {} expired opportunities", expired_count);
        }

        Ok(())
    }

    /// Get arbitrage metrics
    pub async fn get_metrics(&self) -> Result<ArbitrageMetrics> {
        let active_opportunities = self.active_opportunities.read().await;
        let executions = self.executions.read().await;
        
        let total_opportunities = active_opportunities.len() as u64;
        let executed_opportunities = executions.len() as u64;
        let successful_executions = executions
            .iter()
            .filter(|e| e.execution_status == crate::models::ExecutionStatus::Confirmed)
            .count() as u64;
        
        let total_profit: Decimal = executions
            .iter()
            .filter_map(|e| e.actual_profit)
            .sum();
        
        let total_fees: Decimal = executions
            .iter()
            .filter_map(|e| e.total_cost)
            .sum();
        
        let net_profit = total_profit - total_fees;
        let success_rate = if executed_opportunities > 0 {
            Decimal::from(successful_executions) / Decimal::from(executed_opportunities)
        } else {
            Decimal::ZERO
        };

        Ok(ArbitrageMetrics {
            total_opportunities,
            executed_opportunities,
            successful_executions,
            total_profit,
            total_fees,
            net_profit,
            success_rate,
            average_execution_time: None, // Would need to calculate from execution data
            timestamp: chrono::Utc::now(),
        })
    }

    /// Add a new arbitrage strategy
    pub async fn add_strategy(&self, strategy: ArbitrageStrategy) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy.id.clone(), strategy.clone());
        
        // Save to memory store
        if let Err(e) = self.memory_store.save_strategy(&strategy).await {
            warn!("Failed to save strategy to memory store: {}", e);
        }
        
        // If database is available, also save to database
        if let Some(ref db) = self.database {
            if let Err(e) = db.save_strategy(&strategy).await {
                warn!("Failed to save strategy to database: {}", e);
            }
        }
        
        info!("Added new strategy: {}", strategy.name);
        Ok(())
    }

    /// Update a strategy
    pub async fn update_strategy(&self, strategy: ArbitrageStrategy) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy.id.clone(), strategy.clone());
        
        // Update memory store
        if let Err(e) = self.memory_store.update_strategy(&strategy).await {
            warn!("Failed to update strategy in memory store: {}", e);
        }
        
        // If database is available, update database as well
        if let Some(ref db) = self.database {
            if let Err(e) = db.update_strategy(&strategy).await {
                warn!("Failed to update strategy in database: {}", e);
            }
        }
        
        info!("Updated strategy: {}", strategy.name);
        Ok(())
    }

    /// Remove a strategy
    pub async fn remove_strategy(&self, strategy_id: &str) -> Result<()> {
        let mut strategies = self.strategies.write().await;
        if let Some(strategy) = strategies.remove(strategy_id) {
            // Delete from memory store
            if let Err(e) = self.memory_store.delete_strategy(strategy_id).await {
                warn!("Failed to delete strategy from memory store: {}", e);
            }
            
            // If database is available, delete from database as well
            if let Some(ref db) = self.database {
                if let Err(e) = db.delete_strategy(strategy_id).await {
                    warn!("Failed to delete strategy from database: {}", e);
                }
            }
            
            info!("Removed strategy: {}", strategy.name);
        }
        
        Ok(())
    }

    /// Get all active arbitrage opportunities
    pub async fn get_active_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let active_opportunities = self.active_opportunities.read().await;
        active_opportunities.values().cloned().collect()
    }

    /// Get execution history
    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<ArbitrageExecution> {
        let executions = self.executions.read().await;
        let mut result: Vec<ArbitrageExecution> = executions.iter().cloned().collect();
        
        // Sort by time descending
        result.sort_by(|a, b| b.execution_time.cmp(&a.execution_time));
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        result
    }

    /// Get memory store usage
    pub async fn get_storage_usage(&self) -> crate::services::StorageUsage {
        self.memory_store.get_storage_usage().await
    }

    /// Search arbitrage opportunities (using memory store fast search)
    pub async fn search_opportunities(
        &self,
        min_profit: Option<Decimal>,
        max_risk: Option<RiskScore>,
        dex_types: Option<Vec<DexType>>,
    ) -> Vec<ArbitrageOpportunity> {
        self.memory_store.search_opportunities(min_profit, max_risk, dex_types).await
    }
}

impl ArbitrageExecution {
    fn new(opportunity: ArbitrageOpportunity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            opportunity: opportunity.clone(),
            route: crate::models::ArbitrageRoute::new(
                vec![],
                opportunity.base_token.clone(),
                opportunity.quote_token.clone(),
                Decimal::ZERO,
            ),
            transaction_signature: None,
            execution_status: crate::models::ExecutionStatus::Pending,
            gas_used: None,
            gas_price: None,
            total_cost: None,
            actual_profit: None,
            execution_time: chrono::Utc::now(),
            error_message: None,
        }
    }
}
