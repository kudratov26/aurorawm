use std::collections::HashMap;

use smithay::utils::{Physical, Size};

pub struct OutputManager {
    pub outputs: HashMap<String, Output>,
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
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    pub fn add_output(&mut self, name: String, mode: Mode, scale: u32) {
        let size = Size::from((mode.width, mode.height));
        let output = Output {
            name: name.clone(),
            mode,
            scale,
            position: (0, 0),
            size,
        };
        self.outputs.insert(name, output);
    }

    pub fn remove_output(&mut self, name: &str) {
        self.outputs.remove(name);
    }

    pub fn get_primary_output(&self) -> Option<&Output> {
        self.outputs.values().next()
    }
}
