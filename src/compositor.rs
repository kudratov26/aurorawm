use anyhow::Result;
use smithay::reexports::calloop::LoopHandle;
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::reexports::wayland_server::Display;
use std::sync::Arc;
use crate::state::AuroraState;

pub struct AuroraCompositor {
    pub display: Display<AuroraState>,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
}

impl AuroraCompositor {
    pub fn new(loop_handle: &LoopHandle<AuroraState>) -> Result<Self> {
        // Create Wayland display
        let display = Display::new()?;
        
        // Initialize compositor state
        let compositor_state = CompositorState::new::<AuroraState>(&display.handle());
        
        // Initialize XDG shell
        let xdg_shell_state = XdgShellState::new::<AuroraState>(&display.handle());
        
        // Initialize SHM
        let shm_state = ShmState::new::<AuroraState>(&display.handle(), vec![]);
        
        // Create Wayland socket listener
        let socket = smithay::wayland::socket::ListeningSocketSource::with_name("aurorawm-0")?;
        
        // Add socket source to event loop
        let display_clone = display.clone();
        loop_handle.insert_source(socket, move |stream, _, _| {
            let _ = display_clone.insert_client(stream, Arc::new(()));
        })?;
        
        Ok(Self {
            display,
            compositor_state,
            xdg_shell_state,
            shm_state,
        })
    }
    
    pub fn flush_clients(&mut self) {
        self.display.flush_clients();
    }
}
