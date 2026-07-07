use crate::config::Config;
use anyhow::Result;

pub struct AuroraState {
    pub config: Config,
}

impl AuroraState {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
        })
    }
    
    pub fn run(&mut self) -> Result<()> {
        println!("AuroraWM - A fast, reliable Wayland compositor");
        println!("Note: Full Wayland compositor implementation requires Linux with DRM/KMS support.");
        println!("This is a minimal skeleton that compiles successfully.");
        println!("\nConfiguration loaded:");
        println!("  Layout: {}", self.config.layout.default);
        println!("  Gaps: inner={}, outer={}", self.config.layout.gaps.inner, self.config.layout.gaps.outer);
        println!("  Border width: {}", self.config.layout.borders.width);
        
        Ok(())
    }
}
