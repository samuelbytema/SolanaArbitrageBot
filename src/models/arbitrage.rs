use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use rust_decimal::Decimal;
use crate::models::{Token, Pool};
use crate::dex::DexType;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub base_token: Token,
    pub quote_token: Token,
    pub buy_pool: Pool,
    pub sell_pool: Pool,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub price_difference: Decimal,
    pub profit_percentage: Decimal,
    pub estimated_profit: Decimal,
    pub estimated_fees: Decimal,
    pub net_profit: Decimal,
    pub risk_score: RiskScore,
    pub timestamp: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
    pub status: OpportunityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageRoute {
    pub id: String,
    pub pools: Vec<Pool>,
    pub input_token: Token,
    pub output_token: Token,
    pub input_amount: Decimal,
    pub expected_output: Decimal,
    pub actual_output: Decimal,
    pub fees: Vec<Decimal>,
    pub total_fees: Decimal,
    pub price_impact: Decimal,
    pub execution_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageExecution {
    pub id: String,
    pub opportunity: ArbitrageOpportunity,
    pub route: ArbitrageRoute,
    pub transaction_signature: Option<String>,
    pub execution_status: ExecutionStatus,
    pub gas_used: Option<u64>,
    pub gas_price: Option<u64>,
    pub total_cost: Option<Decimal>,
    pub actual_profit: Option<Decimal>,
    pub execution_time: DateTime<Utc>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskScore {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OpportunityStatus {
    Pending,
    Executing,
    Completed,
    Failed,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Pending,
    Executing,
    Submitted,
    Confirmed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageStrategy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub min_profit_threshold: Decimal,
    pub max_slippage: Decimal,
    pub max_price_impact: Decimal,
    pub min_liquidity: Decimal,
    pub supported_dexes: Vec<DexType>,
    pub risk_tolerance: RiskScore,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageMetrics {
    pub total_opportunities: u64,
    pub executed_opportunities: u64,
    pub successful_executions: u64,
    pub total_profit: Decimal,
    pub total_fees: Decimal,
    pub net_profit: Decimal,
    pub success_rate: Decimal,
    pub average_execution_time: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

impl ArbitrageOpportunity {
    pub fn new(
        base_token: Token,
        quote_token: Token,
        buy_pool: Pool,
        sell_pool: Pool,
    ) -> Self {
        let buy_price = buy_pool.get_price(&base_token).unwrap_or(Decimal::ZERO);
        let sell_price = sell_pool.get_price(&base_token).unwrap_or(Decimal::ZERO);
        
        let price_difference = if sell_price > buy_price {
            sell_price - buy_price
        } else {
            Decimal::ZERO
        };
        
        let profit_percentage = if buy_price > Decimal::ZERO {
            price_difference / buy_price
        } else {
            Decimal::ZERO
        };

        let estimated_profit = Decimal::ZERO; // Will be calculated based on amount
        let estimated_fees = Decimal::ZERO; // Will be calculated
        let net_profit = estimated_profit - estimated_fees;
        
        let risk_score = Self::calculate_risk_score(&buy_pool, &sell_pool, profit_percentage);
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            base_token,
            quote_token,
            buy_pool,
            sell_pool,
            buy_price,
            sell_price,
            price_difference,
            profit_percentage,
            estimated_profit,
            estimated_fees,
            net_profit,
            risk_score,
            timestamp: Utc::now(),
            expiry: Utc::now() + chrono::Duration::seconds(30), // 30 seconds expiry
            status: OpportunityStatus::Pending,
        }
    }

    pub fn calculate_risk_score(buy_pool: &Pool, sell_pool: &Pool, profit_percentage: Decimal) -> RiskScore {
        let mut risk_score = 0u8;
        
        // Check liquidity
        if buy_pool.reserve_a < Decimal::from(1000) || buy_pool.reserve_b < Decimal::from(1000) {
            risk_score += 2;
        }
        if sell_pool.reserve_a < Decimal::from(1000) || sell_pool.reserve_b < Decimal::from(1000) {
            risk_score += 2;
        }
        
        // Check profit percentage
        if profit_percentage < Decimal::from(5) / Decimal::from(1000) { // < 0.5%
            risk_score += 1;
        } else if profit_percentage > Decimal::from(5) / Decimal::from(100) { // > 5%
            risk_score += 3; // High profit might indicate high risk
        }
        
        // Check pool age/activity
        let pool_age = Utc::now().signed_duration_since(buy_pool.last_updated);
        if pool_age.num_hours() > 24 {
            risk_score += 1;
        }
        
        match risk_score {
            0..=2 => RiskScore::Low,
            3..=4 => RiskScore::Medium,
            5..=6 => RiskScore::High,
            _ => RiskScore::Critical,
        }
    }

    pub fn is_profitable(&self, min_profit_threshold: Decimal) -> bool {
        self.net_profit > min_profit_threshold
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expiry
    }

    pub fn update_status(&mut self, status: OpportunityStatus) {
        self.status = status;
    }
}

impl ArbitrageRoute {
    pub fn new(pools: Vec<Pool>, input_token: Token, output_token: Token, input_amount: Decimal) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pools,
            input_token,
            output_token,
            input_amount,
            expected_output: Decimal::ZERO,
            actual_output: Decimal::ZERO,
            fees: Vec::new(),
            total_fees: Decimal::ZERO,
            price_impact: Decimal::ZERO,
            execution_time: None,
        }
    }

    pub fn calculate_expected_output(&mut self) -> Option<Decimal> {
        if self.pools.is_empty() {
            return None;
        }

        let mut current_amount = self.input_amount;
        let mut current_token = &self.input_token;

        for pool in &self.pools {
            let output_amount = pool.calculate_output_amount(current_amount, current_token)?;
            current_amount = output_amount;
            current_token = if current_token.mint == pool.token_a.mint {
                &pool.token_b
            } else {
                &pool.token_a
            };
        }

        self.expected_output = current_amount;
        Some(current_amount)
    }

    pub fn calculate_total_fees(&mut self) -> Decimal {
        self.total_fees = self.fees.iter().sum();
        self.total_fees
    }
}

impl ArbitrageStrategy {
    pub fn new(
        name: String,
        description: String,
        min_profit_threshold: Decimal,
        max_slippage: Decimal,
        max_price_impact: Decimal,
        min_liquidity: Decimal,
        supported_dexes: Vec<DexType>,
        risk_tolerance: RiskScore,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            min_profit_threshold,
            max_slippage,
            max_price_impact,
            min_liquidity,
            supported_dexes,
            risk_tolerance,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn is_opportunity_suitable(&self, opportunity: &ArbitrageOpportunity) -> bool {
        opportunity.profit_percentage >= self.min_profit_threshold
            && opportunity.risk_score <= self.risk_tolerance
            && self.supported_dexes.contains(&opportunity.buy_pool.dex_type)
            && self.supported_dexes.contains(&opportunity.sell_pool.dex_type)
    }
}

impl std::fmt::Display for ArbitrageOpportunity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} -> {} (Profit: {:.4}%, Risk: {:?})",
            self.id,
            self.buy_pool.dex_type,
            self.sell_pool.dex_type,
            self.profit_percentage * Decimal::from(100),
            self.risk_score
        )
    }
}
