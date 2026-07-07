use anyhow::Result;
use smithay::reexports::calloop::LoopHandle;
use smithay::wayland::display::Display;
use std::sync::Arc;
use crate::state::AuroraState;

pub struct AuroraCompositor {
    pub display: Display<AuroraState>,
}

impl AuroraCompositor {
    pub fn new(loop_handle: &LoopHandle<AuroraState>) -> Result<Self> {
        // Create Wayland display
        let display = Display::new()?;
        
        // Create Wayland socket listener
        let socket = smithay::wayland::socket::ListeningSocketSource::with_name("aurorawm-0")?;
        
        // Add socket source to event loop
        let display_clone = display.clone();
        loop_handle.insert_source(socket, move |stream, _, _| {
            let _ = display_clone.insert_client(stream, Arc::new(()));
        })?;
        
        Ok(Self {
            display,
        })
    }
    
    pub fn flush_clients(&mut self) {
        self.display.flush_clients();
    }
}
