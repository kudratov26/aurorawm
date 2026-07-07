use anyhow::Result;
use smithay::backend::allocator::dmabuf::DmabufAllocator;
use smithay::backend::allocator::gbm::GbmAllocator;
use smithay::backend::drm::{DrmDevice, DrmNode};
use smithay::backend::egl::{EGLDevice, EGLDisplay};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::reexports::calloop::LoopHandle;
use smithay::utils::{Physical, Size};
use std::collections::HashMap;
use std::sync::Arc;
use crate::state::AuroraState;

pub struct OutputManager {
    pub session: Arc<LibSeatSession>,
    pub gpus: HashMap<DrmNode, Gpu>,
    pub outputs: HashMap<String, Output>,
}

pub struct Gpu {
    pub device: DrmDevice,
    pub renderer: Option<smithay::backend::renderer::gles2::Gles2Renderer>,
    pub egl_display: Option<EGLDisplay>,
}

pub struct Output {
    pub name: String,
    pub mode: Mode,
    pub scale: u32,
    pub position: (i32, i32),
    pub size: Size<u32, Physical>,
}

#[derive(Debug, Clone)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

impl OutputManager {
    pub fn new(_loop_handle: &LoopHandle<AuroraState>) -> Result<Self> {
        // Create session
        let (session, _notifier) = LibSeatSession::new()?;
        
        // Find DRM devices
        let mut gpus = HashMap::new();
        
        // Try to find available DRM nodes
        let drm_nodes = DrmNode::from_path("/dev/dri/card*").unwrap_or_default();
        
        for node in drm_nodes {
            if let Ok(device) = DrmDevice::new(node, None) {
                // Create EGL display
                if let Ok(egl_device) = EGLDevice::from_node(node) {
                    if let Ok(egl_display) = EGLDisplay::new(egl_device) {
                        // Create renderer
                        let gbm_device = device.gbm_device();
                        let gbm = GbmAllocator::new(gbm_device, smithay::backend::allocator::gbm::GbmAllocatorFlags::RENDERING);
                        let dmabuf = DmabufAllocator::new(gbm.clone());
                        
                        let renderer = unsafe {
                            smithay::backend::renderer::gles2::Gles2Renderer::new(egl_display, gbm, dmabuf)
                        };
                        
                        gpus.insert(node, Gpu {
                            device,
                            renderer: renderer.ok(),
                            egl_display: Some(egl_display),
                        });
                    }
                }
            }
        }
        
        Ok(Self {
            session: Arc::new(session),
            gpus,
            outputs: HashMap::new(),
        })
    }
    
    pub fn add_output(&mut self, name: String, mode: Mode, scale: u32) {
        let output = Output {
            name: name.clone(),
            mode,
            scale,
            position: (0, 0),
            size: Size::from((mode.width, mode.height)),
        };
        self.outputs.insert(name, output);
    }
    
    pub fn remove_output(&mut self, name: &str) {
        self.outputs.remove(name);
    }
    
    pub fn get_primary_output(&self) -> Option<&Output> {
        self.outputs.values().next()
    }
    
    pub fn get_renderer(&self) -> Option<&smithay::backend::renderer::gles2::Gles2Renderer> {
        self.gpus.values().next().and_then(|gpu| gpu.renderer.as_ref())
    }
}
