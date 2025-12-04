use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use std::cmp::Ordering;

/// Math utility functions
pub struct MathUtils;

impl MathUtils {
    /// Simple power function implementation (using f64 conversion)
    fn decimal_pow(base: Decimal, exponent: Decimal) -> Option<Decimal> {
        let base_f64 = base.to_f64()?;
        let exp_f64 = exponent.to_f64()?;
        let result = base_f64.powf(exp_f64);
        Decimal::from_f64(result)
    }

    /// Simple square root implementation (using f64 conversion)
    fn decimal_sqrt(value: Decimal) -> Option<Decimal> {
        let value_f64 = value.to_f64()?;
        if value_f64 < 0.0 {
            return None;
        }
        let result = value_f64.sqrt();
        Decimal::from_f64(result)
    }
    /// Calculate percentage change
    pub fn calculate_percentage_change(old_value: Decimal, new_value: Decimal) -> Decimal {
        if old_value == Decimal::ZERO {
            return Decimal::ZERO;
        }
        
        (new_value - old_value) / old_value
    }
    
    /// Calculate APY (annual percentage yield)
    pub fn calculate_apy(
        initial_value: Decimal,
        final_value: Decimal,
        time_period_days: u64,
    ) -> Decimal {
        if initial_value == Decimal::ZERO || time_period_days == 0 {
            return Decimal::ZERO;
        }
        
        let total_return = (final_value - initial_value) / initial_value;
        let years = Decimal::from(time_period_days) / Decimal::from(365);
        
        if years == Decimal::ZERO {
            return Decimal::ZERO;
        }
        
        Self::decimal_pow(Decimal::ONE + total_return, Decimal::ONE / years).unwrap_or(Decimal::ZERO) - Decimal::ONE
    }
    
    /// Calculate compound interest
    pub fn calculate_compound_interest(
        principal: Decimal,
        rate: Decimal,
        periods: u32,
    ) -> Decimal {
        if rate == Decimal::ZERO || periods == 0 {
            return principal;
        }
        
        principal * Self::decimal_pow(Decimal::ONE + rate, Decimal::from(periods)).unwrap_or(Decimal::ONE)
    }
    
    /// Calculate geometric mean
    pub fn geometric_mean(values: &[Decimal]) -> Option<Decimal> {
        if values.is_empty() {
            return None;
        }
        
        let product: Decimal = values.iter().product();
        let count = Decimal::from(values.len());
        
        if product < Decimal::ZERO {
            return None; // Geometric mean undefined for negative product
        }
        
        Self::decimal_pow(product, Decimal::ONE / count)
    }
    
    /// Calculate weighted average
    pub fn weighted_average(values: &[Decimal], weights: &[Decimal]) -> Option<Decimal> {
        if values.len() != weights.len() || values.is_empty() {
            return None;
        }
        
        let weighted_sum: Decimal = values
            .iter()
            .zip(weights.iter())
            .map(|(value, weight)| value * weight)
            .sum();
        
        let total_weight: Decimal = weights.iter().sum();
        
        if total_weight == Decimal::ZERO {
            return None;
        }
        
        Some(weighted_sum / total_weight)
    }
    
    /// Calculate standard deviation
    pub fn standard_deviation(values: &[Decimal]) -> Option<Decimal> {
        if values.len() < 2 {
            return None;
        }
        
        let mean = values.iter().sum::<Decimal>() / Decimal::from(values.len());
        let variance: Decimal = values
            .iter()
            .map(|value| Self::decimal_pow(value - mean, Decimal::from(2)).unwrap_or(Decimal::ZERO))
            .sum::<Decimal>()
            / Decimal::from(values.len() - 1);
        
        Self::decimal_sqrt(variance)
    }
    
    /// Calculate Sharpe ratio
    pub fn sharpe_ratio(
        returns: &[Decimal],
        risk_free_rate: Decimal,
    ) -> Option<Decimal> {
        if returns.len() < 2 {
            return None;
        }
        
        let mean_return = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let excess_return = mean_return - risk_free_rate;
        
        if let Some(std_dev) = Self::standard_deviation(returns) {
            if std_dev == Decimal::ZERO {
                return None;
            }
            Some(excess_return / std_dev)
        } else {
            None
        }
    }
    
    /// Calculate max drawdown
    pub fn max_drawdown(values: &[Decimal]) -> Option<Decimal> {
        if values.len() < 2 {
            return None;
        }
        
        let mut peak = values[0];
        let mut max_dd = Decimal::ZERO;
        
        for &value in values {
            if value > peak {
                peak = value;
            }
            
            let drawdown = (peak - value) / peak;
            if drawdown > max_dd {
                max_dd = drawdown;
            }
        }
        
        Some(max_dd)
    }
    
    /// Calculate correlation coefficient
    pub fn correlation(x_values: &[Decimal], y_values: &[Decimal]) -> Option<Decimal> {
        if x_values.len() != y_values.len() || x_values.len() < 2 {
            return None;
        }
        
        let n = Decimal::from(x_values.len());
        let x_mean = x_values.iter().sum::<Decimal>() / n;
        let y_mean = y_values.iter().sum::<Decimal>() / n;
        
        let numerator: Decimal = x_values
            .iter()
            .zip(y_values.iter())
            .map(|(x, y)| (x - x_mean) * (y - y_mean))
            .sum();
        
        let x_variance: Decimal = x_values
            .iter()
            .map(|x| Self::decimal_pow(x - x_mean, Decimal::from(2)).unwrap_or(Decimal::ZERO))
            .sum();
        
        let y_variance: Decimal = y_values
            .iter()
            .map(|y| Self::decimal_pow(y - y_mean, Decimal::from(2)).unwrap_or(Decimal::ZERO))
            .sum();
        
        let denominator = Self::decimal_sqrt(x_variance * y_variance).unwrap_or(Decimal::ZERO);
        
        if denominator == Decimal::ZERO {
            return None;
        }
        
        Some(numerator / denominator)
    }
    
    /// Calculate moving average
    pub fn moving_average(values: &[Decimal], window: usize) -> Vec<Decimal> {
        if values.len() < window || window == 0 {
            return Vec::new();
        }
        
        let mut result = Vec::new();
        
        for i in window - 1..values.len() {
            let start = i + 1 - window;
            let sum: Decimal = values[start..=i].iter().sum();
            result.push(sum / Decimal::from(window));
        }
        
        result
    }
    
    /// Calculate exponential moving average
    pub fn exponential_moving_average(values: &[Decimal], alpha: Decimal) -> Vec<Decimal> {
        if values.is_empty() || alpha <= Decimal::ZERO || alpha >= Decimal::ONE {
            return Vec::new();
        }
        
        let mut result = Vec::new();
        let mut ema = values[0];
        result.push(ema);
        
        for &value in values.iter().skip(1) {
            ema = alpha * value + (Decimal::ONE - alpha) * ema;
            result.push(ema);
        }
        
        result
    }
    
    /// Calculate Relative Strength Index (RSI)
    pub fn rsi(values: &[Decimal], period: usize) -> Option<Vec<Decimal>> {
        if values.len() < period + 1 || period == 0 {
            return None;
        }
        
        let mut gains = Vec::new();
        let mut losses = Vec::new();
        
        for i in 1..values.len() {
            let change = values[i] - values[i - 1];
            if change > Decimal::ZERO {
                gains.push(change);
                losses.push(Decimal::ZERO);
            } else {
                gains.push(Decimal::ZERO);
                losses.push(-change);
            }
        }
        
        let avg_gain = Self::exponential_moving_average(&gains, Decimal::from(1) / Decimal::from(period));
        let avg_loss = Self::exponential_moving_average(&losses, Decimal::from(1) / Decimal::from(period));
        
        let mut rsi_values = Vec::new();
        
        for i in 0..avg_gain.len() {
            if avg_loss[i] == Decimal::ZERO {
                rsi_values.push(Decimal::from(100));
            } else {
                let rs = avg_gain[i] / avg_loss[i];
                let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs));
                rsi_values.push(rsi);
            }
        }
        
        Some(rsi_values)
    }
    
    /// Calculate Bollinger Bands
    pub fn bollinger_bands(
        values: &[Decimal],
        period: usize,
        std_dev_multiplier: Decimal,
    ) -> Option<(Vec<Decimal>, Vec<Decimal>, Vec<Decimal>)> {
        if values.len() < period || period == 0 {
            return None;
        }
        
        let sma = Self::moving_average(values, period);
        let mut upper_band = Vec::new();
        let mut lower_band = Vec::new();
        
        for i in period - 1..values.len() {
            let window = &values[i - period + 1..=i];
            if let Some(std_dev) = Self::standard_deviation(window) {
                let middle = sma[i - period + 1];
                upper_band.push(middle + std_dev_multiplier * std_dev);
                lower_band.push(middle - std_dev_multiplier * std_dev);
            } else {
                upper_band.push(Decimal::ZERO);
                lower_band.push(Decimal::ZERO);
            }
        }
        
        Some((upper_band, sma, lower_band))
    }
}

/// Financial calculation utilities
pub struct FinancialUtils;

impl FinancialUtils {
    /// Simple power function implementation (using f64 conversion)
    fn decimal_pow(base: Decimal, exponent: Decimal) -> Option<Decimal> {
        let base_f64 = base.to_f64()?;
        let exp_f64 = exponent.to_f64()?;
        let result = base_f64.powf(exp_f64);
        Decimal::from_f64(result)
    }
}

impl FinancialUtils {
    /// Calculate present value
    pub fn present_value(
        future_value: Decimal,
        rate: Decimal,
        periods: u32,
    ) -> Decimal {
        if rate == Decimal::ZERO {
            return future_value;
        }
        
        future_value / Self::decimal_pow(Decimal::ONE + rate, Decimal::from(periods)).unwrap_or(Decimal::ONE)
    }
    
    /// Calculate future value
    pub fn future_value(
        present_value: Decimal,
        rate: Decimal,
        periods: u32,
    ) -> Decimal {
        if rate == Decimal::ZERO {
            return present_value;
        }
        
        present_value * Self::decimal_pow(Decimal::ONE + rate, Decimal::from(periods)).unwrap_or(Decimal::ONE)
    }
    
    /// Calculate present value of an annuity
    pub fn present_value_annuity(
        payment: Decimal,
        rate: Decimal,
        periods: u32,
    ) -> Decimal {
        if rate == Decimal::ZERO {
            return payment * Decimal::from(periods);
        }
        
        payment * (Decimal::ONE - Self::decimal_pow(Decimal::ONE + rate, -Decimal::from(periods)).unwrap_or(Decimal::ONE)) / rate
    }
    
    /// Calculate future value of an annuity
    pub fn future_value_annuity(
        payment: Decimal,
        rate: Decimal,
        periods: u32,
    ) -> Decimal {
        if rate == Decimal::ZERO {
            return payment * Decimal::from(periods);
        }
        
        payment * (Self::decimal_pow(Decimal::ONE + rate, Decimal::from(periods)).unwrap_or(Decimal::ONE) - Decimal::ONE) / rate
    }
    
    /// Calculate Internal Rate of Return (IRR)
    pub fn internal_rate_of_return(cash_flows: &[Decimal]) -> Option<Decimal> {
        if cash_flows.len() < 2 {
            return None;
        }
        
        // Simplified IRR calculation; real applications may need more robust numerical methods
        let mut rate = Decimal::from(1) / Decimal::from(100); // 1%
        let mut prev_npv = Decimal::ZERO;
        
        for _ in 0..100 {
            let mut npv = Decimal::ZERO;
            
            for (i, &cf) in cash_flows.iter().enumerate() {
                npv += cf / Self::decimal_pow(Decimal::ONE + rate, Decimal::from(i)).unwrap_or(Decimal::ONE);
            }
            
            if npv.abs() < Decimal::from(1) / Decimal::from(10000) {
                return Some(rate);
            }
            
            if prev_npv != Decimal::ZERO {
                let derivative = (npv - prev_npv) / (Decimal::from(1) / Decimal::from(100));
                if derivative.abs() < Decimal::from(1) / Decimal::from(10000) {
                    break;
                }
                rate -= npv / derivative;
            }
            
            prev_npv = npv;
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate_percentage_change() {
        let result = MathUtils::calculate_percentage_change(
            Decimal::from(100),
            Decimal::from(110),
        );
        assert_eq!(result, Decimal::from(1) / Decimal::from(10)); // 10%
    }
    
    #[test]
    fn test_geometric_mean() {
        let values = vec![
            Decimal::from(2),
            Decimal::from(8),
            Decimal::from(32),
        ];
        let result = MathUtils::geometric_mean(&values).unwrap();
        assert!((result - Decimal::from(8)).abs() < Decimal::from(1) / Decimal::from(100));
    }
    
    #[test]
    fn test_moving_average() {
        let values = vec![
            Decimal::from(1),
            Decimal::from(2),
            Decimal::from(3),
            Decimal::from(4),
            Decimal::from(5),
        ];
        let result = MathUtils::moving_average(&values, 3);
        assert_eq!(result, vec![
            Decimal::from(2),
            Decimal::from(3),
            Decimal::from(4),
        ]);
    }
}
