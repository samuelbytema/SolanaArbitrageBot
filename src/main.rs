use clap::Parser;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use offchain_bot::{
    config::AppConfig,
    services::database::DatabaseService,
    dex::DexFactory,
    DexType,
    arbitrage::ArbitrageEngine,
};

#[derive(Parser)]
#[command(name = "offchain-bot")]
#[command(about = "Solana DEX arbitrage bot with offchain execution and Jito MEV protection")]
#[command(version = "0.1.0")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config/default.toml")]
    config: String,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,
    
    /// Dry run mode (don't execute trades)
    #[arg(long)]
    dry_run: bool,
    
    /// Force use memory store only
    #[arg(long)]
    memory_only: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(&cli.log_level, cli.debug)?;
    
    info!("Starting Solana DEX Arbitrage Bot...");
    info!("Version: 0.1.0");
    info!("Configuration: {}", cli.config);
    info!("Log level: {}", cli.log_level);
    info!("Debug mode: {}", cli.debug);
    info!("Dry run mode: {}", cli.dry_run);
    info!("Memory only mode: {}", cli.memory_only);
    
    // Load configuration
    let config = load_config(&cli.config)?;
    info!("Configuration loaded successfully");
    
    // Initialize storage services based on configuration
    let database = if cli.memory_only || !config.is_memory_store_enabled() {
        None
    } else {
        match DatabaseService::new(&config.database.url).await {
            Ok(db) => {
                info!("Database service initialized successfully");
                Some(std::sync::Arc::new(db))
            }
            Err(e) => {
                warn!("Failed to initialize database service: {}, falling back to memory store only", e);
                None
            }
        }
    };
    
    if database.is_none() {
        info!("Using memory store only for high-frequency trading");
    }
    
    // Create DEX instances
    let dex_instances = create_dex_instances(&config).await?;
    info!("DEX instances created: {:?}", dex_instances.keys().collect::<Vec<_>>());
    
    // Create arbitrage engine
    let mut arbitrage_engine = ArbitrageEngine::new(
        config.clone(),
        database,
        dex_instances,
    );
    
    // Start arbitrage engine
    info!("Starting arbitrage engine...");
    if let Err(e) = arbitrage_engine.start().await {
        error!("Failed to start arbitrage engine: {}", e);
        return Err(e);
    }
    
    info!("Arbitrage bot started successfully");
    
    // Wait for interrupt signal
    tokio::signal::ctrl_c().await?;
    info!("Received interrupt signal, shutting down...");
    
    Ok(())
}

/// Initialize logging system
fn init_logging(log_level: &str, debug: bool) -> anyhow::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            if debug {
                "offchain_bot=debug,tower=debug,hyper=debug".into()
            } else {
                format!("offchain_bot={}", log_level.to_lowercase()).into()
            }
        });
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

/// Load configuration from file
fn load_config(_config_path: &str) -> anyhow::Result<AppConfig> {
    let config = AppConfig::load()?;
    Ok(config)
}

/// Create DEX instances based on configuration
async fn create_dex_instances(config: &AppConfig) -> anyhow::Result<std::collections::HashMap<DexType, Box<dyn offchain_bot::dex::DexInterface>>> {
    let dex_instances = DexFactory::create_all_dexes(config).await?;
    if dex_instances.is_empty() {
        return Err(anyhow::anyhow!("No DEX instances could be created"));
    }
    Ok(dex_instances)
}
