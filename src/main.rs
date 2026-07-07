mod compositor;
mod config;
mod input;
mod layout;
mod output;
mod render;
mod shell;
mod state;
mod utils;

use anyhow::Result;
use env_logger::Env;
use log::{info};
use state::AuroraState;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    info!("Starting AuroraWM - A fast, reliable Wayland compositor");
    info!("This requires Linux with DRM/KMS support");
    
    // Load configuration
    let config = config::load_config()?;
    info!("Configuration loaded successfully");
    
    // Initialize the compositor state
    let mut state = AuroraState::new(config)?;
    
    // Run the event loop
    info!("Starting event loop");
    state.run()?;
    
    Ok(())
}
