pub mod interface;
pub mod raydium;
pub mod meteora;
pub mod whirlpool;
pub mod pump;
pub mod factory;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DexType {
    Raydium,
    Meteora,
    Whirlpool,
    Pump,
}

impl std::fmt::Display for DexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DexType::Raydium => write!(f, "Raydium"),
            DexType::Meteora => write!(f, "Meteora"),
            DexType::Whirlpool => write!(f, "Whirlpool"),
            DexType::Pump => write!(f, "Pump"),
        }
    }
}

pub use interface::*;
pub use factory::*;
