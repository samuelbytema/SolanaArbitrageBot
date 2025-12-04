use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{ArbitrageOpportunity, RiskScore};
use crate::dex::DexType;

/// Arbitrage strategy interface
pub trait Strategy: Send + Sync {
    /// Strategy name
    fn name(&self) -> &str;
    
    /// Strategy description
    fn description(&self) -> &str;
    
    /// Whether the arbitrage should be executed
    fn should_execute(&self, opportunity: &ArbitrageOpportunity) -> bool;
    
    /// Calculate optimal trade amount
    fn calculate_optimal_amount(&self, opportunity: &ArbitrageOpportunity) -> Option<Decimal>;
    
    /// Get strategy parameters
    fn get_parameters(&self) -> StrategyParameters;
    
    /// Validate strategy configuration
    fn validate(&self) -> Result<()>;
}

/// Basic arbitrage strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseArbitrageStrategy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub parameters: StrategyParameters,
    pub is_active: bool,
}

impl Strategy for BaseArbitrageStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn should_execute(&self, opportunity: &ArbitrageOpportunity) -> bool {
        if !self.is_active {
            return false;
        }
        
        // Check profit threshold
        if opportunity.profit_percentage < self.parameters.min_profit_threshold {
            return false;
        }
        
        // Check risk score
        if opportunity.risk_score > self.parameters.max_risk_score {
            return false;
        }
        
        // Check DEX support
        if !self.parameters.supported_dexes.contains(&opportunity.buy_pool.dex_type)
            || !self.parameters.supported_dexes.contains(&opportunity.sell_pool.dex_type)
        {
            return false;
        }
        
        // Check liquidity requirements
        let min_liquidity = self.parameters.min_liquidity;
        if opportunity.buy_pool.reserve_a < min_liquidity
            || opportunity.buy_pool.reserve_b < min_liquidity
            || opportunity.sell_pool.reserve_a < min_liquidity
            || opportunity.sell_pool.reserve_b < min_liquidity
        {
            return false;
        }
        
        true
    }
    
    fn calculate_optimal_amount(&self, opportunity: &ArbitrageOpportunity) -> Option<Decimal> {
        let buy_pool = &opportunity.buy_pool;
        let sell_pool = &opportunity.sell_pool;
        
        // Calculate maximum tradable amount
        let max_buy_amount = std::cmp::min(
            buy_pool.reserve_a,
            buy_pool.reserve_b,
        );
        
        let max_sell_amount = std::cmp::min(
            sell_pool.reserve_a,
            sell_pool.reserve_b,
        );
        
        let max_amount = std::cmp::min(max_buy_amount, max_sell_amount);
        
        // Apply strategy constraints
        let strategy_amount = max_amount * self.parameters.position_size_multiplier;
        
        // Cap the maximum trade amount
        let max_trade_amount = self.parameters.max_trade_amount;
        
        Some(std::cmp::min(strategy_amount, max_trade_amount))
    }
    
    fn get_parameters(&self) -> StrategyParameters {
        self.parameters.clone()
    }
    
    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("Strategy name cannot be empty");
        }
        
        if self.parameters.min_profit_threshold <= Decimal::ZERO {
            anyhow::bail!("Min profit threshold must be positive");
        }
        
        if self.parameters.max_slippage <= Decimal::ZERO {
            anyhow::bail!("Max slippage must be positive");
        }
        
        if self.parameters.min_liquidity <= Decimal::ZERO {
            anyhow::bail!("Min liquidity must be positive");
        }
        
        if self.parameters.max_trade_amount <= Decimal::ZERO {
            anyhow::bail!("Max trade amount must be positive");
        }
        
        if self.parameters.position_size_multiplier <= Decimal::ZERO {
            anyhow::bail!("Position size multiplier must be positive");
        }
        
        if self.parameters.supported_dexes.is_empty() {
            anyhow::bail!("At least one DEX must be supported");
        }
        
        Ok(())
    }
}

/// Strategy parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParameters {
    /// Minimum profit threshold
    pub min_profit_threshold: Decimal,
    /// Maximum slippage
    pub max_slippage: Decimal,
    /// Maximum price impact
    pub max_price_impact: Decimal,
    /// Minimum liquidity requirement
    pub min_liquidity: Decimal,
    /// Maximum trade amount
    pub max_trade_amount: Decimal,
    /// Position size multiplier
    pub position_size_multiplier: Decimal,
    /// Supported DEX types
    pub supported_dexes: Vec<DexType>,
    /// Maximum risk score
    pub max_risk_score: RiskScore,
    /// Execution delay (seconds)
    pub execution_delay_seconds: u64,
    /// Maximum retries
    pub max_retries: u32,
    /// Retry delay (seconds)
    pub retry_delay_seconds: u64,
}

impl Default for StrategyParameters {
    fn default() -> Self {
        Self {
            min_profit_threshold: Decimal::from(5) / Decimal::from(1000), // 0.5%
            max_slippage: Decimal::from(1) / Decimal::from(100), // 1%
            max_price_impact: Decimal::from(5) / Decimal::from(1000), // 0.5%
            min_liquidity: Decimal::from(1000),
            max_trade_amount: Decimal::from(10000),
            position_size_multiplier: Decimal::from(1),
            supported_dexes: vec![DexType::Raydium, DexType::Meteora, DexType::Whirlpool, DexType::Pump],
            max_risk_score: RiskScore::Medium,
            execution_delay_seconds: 0,
            max_retries: 3,
            retry_delay_seconds: 5,
        }
    }
}

/// Strategy factory
pub struct StrategyFactory;

impl StrategyFactory {
    /// Create a basic strategy
    pub fn create_base_strategy(
        name: String,
        description: String,
        parameters: StrategyParameters,
    ) -> Result<BaseArbitrageStrategy> {
        let strategy = BaseArbitrageStrategy {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            parameters,
            is_active: true,
        };
        
        strategy.validate()?;
        Ok(strategy)
    }
    
    /// Create a conservative strategy
    pub fn create_conservative_strategy() -> BaseArbitrageStrategy {
        let mut parameters = StrategyParameters::default();
        parameters.min_profit_threshold = Decimal::from(1) / Decimal::from(100); // 1%
        parameters.max_risk_score = RiskScore::Low;
        parameters.max_trade_amount = Decimal::from(5000);
        parameters.position_size_multiplier = Decimal::from(5) / Decimal::from(10); // 0.5
        
        BaseArbitrageStrategy {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Conservative".to_string(),
            description: "Conservative arbitrage strategy with low risk tolerance".to_string(),
            parameters,
            is_active: true,
        }
    }
    
    /// Create an aggressive strategy
    pub fn create_aggressive_strategy() -> BaseArbitrageStrategy {
        let mut parameters = StrategyParameters::default();
        parameters.min_profit_threshold = Decimal::from(2) / Decimal::from(1000); // 0.2%
        parameters.max_risk_score = RiskScore::High;
        parameters.max_trade_amount = Decimal::from(50000);
        parameters.position_size_multiplier = Decimal::from(2);
        
        BaseArbitrageStrategy {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Aggressive".to_string(),
            description: "Aggressive arbitrage strategy with high risk tolerance".to_string(),
            parameters,
            is_active: true,
        }
    }
    
    /// Create a triangular arbitrage strategy
    pub fn create_triangular_strategy() -> BaseArbitrageStrategy {
        let mut parameters = StrategyParameters::default();
        parameters.min_profit_threshold = Decimal::from(3) / Decimal::from(1000); // 0.3%
        parameters.max_risk_score = RiskScore::Medium;
        parameters.max_trade_amount = Decimal::from(20000);
        parameters.position_size_multiplier = Decimal::from(15) / Decimal::from(10); // 1.5
        
        BaseArbitrageStrategy {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Triangular".to_string(),
            description: "Triangular arbitrage strategy for three-token cycles".to_string(),
            parameters,
            is_active: true,
        }
    }
}

/// Strategy manager
pub struct StrategyManager {
    strategies: HashMap<String, Box<dyn Strategy>>,
}

impl StrategyManager {
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }
    
    /// Add a strategy
    pub fn add_strategy(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.insert(strategy.name().to_string(), strategy);
    }
    
    /// Remove a strategy
    pub fn remove_strategy(&mut self, name: &str) -> Option<Box<dyn Strategy>> {
        self.strategies.remove(name)
    }
    
    /// Get a strategy
    pub fn get_strategy(&self, name: &str) -> Option<&Box<dyn Strategy>> {
        self.strategies.get(name)
    }
    
    /// Get all strategies
    pub fn get_all_strategies(&self) -> &HashMap<String, Box<dyn Strategy>> {
        &self.strategies
    }
    
    /// Evaluate an arbitrage opportunity
    pub fn evaluate_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Vec<StrategyEvaluation> {
        let mut evaluations = Vec::new();
        
        for strategy in self.strategies.values() {
            let should_execute = strategy.should_execute(opportunity);
            let optimal_amount = strategy.calculate_optimal_amount(opportunity);
            let parameters = strategy.get_parameters();
            
            let evaluation = StrategyEvaluation {
                strategy_name: strategy.name().to_string(),
                should_execute,
                optimal_amount,
                parameters,
                score: self.calculate_strategy_score(opportunity, strategy),
            };
            
            evaluations.push(evaluation);
        }
        
        // Sort by score
        evaluations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        evaluations
    }
    
    /// Calculate strategy score
    fn calculate_strategy_score(
        &self,
        opportunity: &ArbitrageOpportunity,
        strategy: &Box<dyn Strategy>,
    ) -> f64 {
        let mut score = 0.0;
        
        // Profit score
        let profit_score = (opportunity.profit_percentage / strategy.get_parameters().min_profit_threshold)
            .to_f64()
            .unwrap_or(0.0);
        score += profit_score * 0.4;
        
        // Risk score
        let risk_score = match opportunity.risk_score {
            RiskScore::Low => 1.0,
            RiskScore::Medium => 0.7,
            RiskScore::High => 0.4,
            RiskScore::Critical => 0.0,
        };
        score += risk_score * 0.3;
        
        // Liquidity score
        let liquidity_score = std::cmp::min(
            opportunity.buy_pool.reserve_a,
            opportunity.buy_pool.reserve_b,
        ).to_f64().unwrap_or(0.0) / 10000.0;
        score += liquidity_score.min(1.0) * 0.3;
        
        score
    }
}

/// Strategy evaluation result
#[derive(Debug, Clone)]
pub struct StrategyEvaluation {
    pub strategy_name: String,
    pub should_execute: bool,
    pub optimal_amount: Option<Decimal>,
    pub parameters: StrategyParameters,
    pub score: f64,
}
