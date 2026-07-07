use anyhow::Result;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_server::Display;
use std::sync::Arc;
use crate::state::AuroraState;

pub struct AuroraCompositor {
    pub display: Display<AuroraState>,
}

impl AuroraCompositor {
    pub fn new(_loop_handle: &LoopHandle<AuroraState>) -> Result<Self> {
        // Create Wayland display
        let display = Display::new()?;
        
        Ok(Self {
            display,
        })
    }
    
    pub fn flush_clients(&mut self) {
        self.display.flush_clients();
    }
}
