use anyhow::Result;
use smithay::reexports::calloop::LoopHandle;
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::display::Display;
use std::sync::Arc;

pub struct AuroraCompositor {
    pub display: Display<()>,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
}

impl AuroraCompositor {
    pub fn new(loop_handle: &LoopHandle<()>) -> Result<Self> {
        // Create Wayland display
        let display = Display::new()?;
        
        // Initialize compositor state
        let compositor_state = CompositorState::new::<()>(&display);
        
        // Initialize XDG shell
        let xdg_shell_state = XdgShellState::new::<()>(&display);
        
        // Initialize SHM
        let shm_state = ShmState::new::<()>(&display, vec![]);
        
        // Create Wayland socket listener
        let socket = smithay::wayland::socket::ListeningSocketSource::new("aurorawm-0")?;
        
        // Add socket source to event loop
        let display_clone = display.clone();
        loop_handle.insert_source(socket, move |stream, _, _| {
            // Client will be handled by the display
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
