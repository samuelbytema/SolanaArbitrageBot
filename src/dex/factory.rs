use std::collections::HashMap;
use anyhow::Result;
use crate::dex::{DexInterface, DexConnectionConfig, DexType};

pub struct DexFactory {
    dex_instances: HashMap<DexType, Box<dyn DexInterface>>,
}

impl DexFactory {
    pub fn new() -> Self {
        Self {
            dex_instances: HashMap::new(),
        }
    }

    /// Create all DEX instances
    pub async fn create_all_dexes(config: &crate::config::AppConfig) -> Result<HashMap<DexType, Box<dyn DexInterface>>> {
        let mut factory = Self::new();
        
        // Create Raydium DEX
        if let Ok(raydium) = factory.create_raydium_dex(&config.dex.raydium).await {
            factory.dex_instances.insert(DexType::Raydium, raydium);
        }
        
        // Create Meteora DEX
        if let Ok(meteora) = factory.create_meteora_dex(&config.dex.meteora).await {
            factory.dex_instances.insert(DexType::Meteora, meteora);
        }
        
        // Create Whirlpool DEX
        if let Ok(whirlpool) = factory.create_whirlpool_dex(&config.dex.whirlpool).await {
            factory.dex_instances.insert(DexType::Whirlpool, whirlpool);
        }
        
        // Create Pump DEX
        if let Ok(pump) = factory.create_pump_dex(&config.dex.pump).await {
            factory.dex_instances.insert(DexType::Pump, pump);
        }
        
        Ok(factory.dex_instances)
    }

    /// Create Raydium DEX instance
    async fn create_raydium_dex(&self, config: &crate::config::DexEndpointConfig) -> Result<Box<dyn DexInterface>> {
        let dex_config = DexConnectionConfig {
            base_url: config.base_url.clone(),
            api_key: Some(config.api_key.clone()),
            timeout_seconds: config.timeout_seconds,
            max_retries: 3,
            rate_limit: config.rate_limit,
        };
        
        let raydium_dex = crate::dex::raydium::RaydiumDex::new(dex_config)?;
        Ok(Box::new(raydium_dex))
    }

    /// Create Meteora DEX instance
    async fn create_meteora_dex(&self, config: &crate::config::DexEndpointConfig) -> Result<Box<dyn DexInterface>> {
        let dex_config = DexConnectionConfig {
            base_url: config.base_url.clone(),
            api_key: Some(config.api_key.clone()),
            timeout_seconds: config.timeout_seconds,
            max_retries: 3,
            rate_limit: config.rate_limit,
        };
        
        let meteora_dex = crate::dex::meteora::MeteoraDex::new(dex_config)?;
        Ok(Box::new(meteora_dex))
    }

    /// Create Whirlpool DEX instance
    async fn create_whirlpool_dex(&self, config: &crate::config::DexEndpointConfig) -> Result<Box<dyn DexInterface>> {
        let dex_config = DexConnectionConfig {
            base_url: config.base_url.clone(),
            api_key: Some(config.api_key.clone()),
            timeout_seconds: config.timeout_seconds,
            max_retries: 3,
            rate_limit: config.rate_limit,
        };
        
        let whirlpool_dex = crate::dex::whirlpool::WhirlpoolDex::new(dex_config)?;
        Ok(Box::new(whirlpool_dex))
    }

    /// Create Pump DEX instance
    async fn create_pump_dex(&self, config: &crate::config::DexEndpointConfig) -> Result<Box<dyn DexInterface>> {
        let dex_config = DexConnectionConfig {
            base_url: config.base_url.clone(),
            api_key: Some(config.api_key.clone()),
            timeout_seconds: config.timeout_seconds,
            max_retries: 3,
            rate_limit: config.rate_limit,
        };
        
        let pump_dex = crate::dex::pump::PumpDex::new(dex_config)?;
        Ok(Box::new(pump_dex))
    }

    /// Get DEX instance of a specific type
    pub fn get_dex(&self, dex_type: &DexType) -> Option<&Box<dyn DexInterface>> {
        self.dex_instances.get(dex_type)
    }

    /// Get all DEX instances
    pub fn get_all_dexes(&self) -> &HashMap<DexType, Box<dyn DexInterface>> {
        &self.dex_instances
    }

    /// Check DEX health status
    pub async fn check_dex_health(&self) -> HashMap<DexType, bool> {
        let mut health_status = HashMap::new();
        
        for (dex_type, dex_instance) in &self.dex_instances {
            let is_healthy = dex_instance.is_connected().await.unwrap_or(false);
            health_status.insert(dex_type.clone(), is_healthy);
        }
        
        health_status
    }
}
