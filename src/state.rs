use crate::compositor::AuroraCompositor;
use crate::config::Config;
use crate::input::InputManager;
use crate::layout::LayoutEngine;
use crate::output::OutputManager;
use crate::render::Renderer;
use anyhow::Result;
use smithay::reexports::calloop::{EventLoop, LoopHandle};
use smithay::wayland::compositor::{CompositorHandler, CompositorState};
use smithay::wayland::shell::xdg::{XdgShellHandler, XdgShellState};
use smithay::wayland::compositor::Surface;
use std::sync::Arc;

pub struct AuroraState {
    pub config: Config,
    pub compositor: AuroraCompositor,
    pub input_manager: InputManager,
    pub layout_engine: LayoutEngine,
    pub output_manager: OutputManager,
    pub renderer: Renderer,
    pub event_loop: EventLoop<()>,
    pub loop_handle: LoopHandle<()>,
}

impl CompositorHandler for AuroraState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor.compositor_state
    }
    
    fn commit(&mut self, surface: &Surface) {
        // Handle surface commit
        // This is called when a client commits changes to a surface
    }
}

impl XdgShellHandler for AuroraState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.compositor.xdg_shell_state
    }
}

impl AuroraState {
    pub fn new(config: Config) -> Result<Self> {
        // Create event loop
        let mut event_loop = EventLoop::<()>::try_new()?;
        let loop_handle = event_loop.handle();
        
        // Initialize compositor
        let compositor = AuroraCompositor::new(&loop_handle)?;
        
        // Initialize output manager
        let output_manager = OutputManager::new(&loop_handle)?;
        
        // Initialize renderer
        let renderer = Renderer::new(&output_manager)?;
        
        // Initialize input manager
        let input_manager = InputManager::new(&loop_handle, &config)?;
        
        // Initialize layout engine
        let layout_engine = LayoutEngine::new(&config);
        
        Ok(Self {
            config,
            compositor,
            input_manager,
            layout_engine,
            output_manager,
            renderer,
            event_loop,
            loop_handle,
        })
    }
    
    pub fn run(&mut self) -> Result<()> {
        // Run the main event loop
        self.event_loop.run(None, &mut (), |_| {
            // Dispatch Wayland events
            self.compositor.flush_clients();
        })?;
        Ok(())
    }
}
