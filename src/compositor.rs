use std::sync::Arc;

use smithay::reexports::wayland_server::{
    Client,
    Display,
    ListeningSocket,
};

use crate::state::{AuroraState, ClientState};

pub struct AuroraCompositor {
    pub display: Display<AuroraState>,
    pub listener: ListeningSocket,
}

impl AuroraCompositor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let display = Display::new()?;
        let listener = ListeningSocket::bind("aurorawm-0")?;

        Ok(Self {
            display,
            listener,
        })
    }

    pub fn accept_clients(&mut self) -> Result<Vec<Client>, Box<dyn std::error::Error>> {
        let mut clients = Vec::new();
        let mut dh = self.display.handle();
        while let Some(stream) = self.listener.accept()? {
            let client = dh.insert_client(stream, Arc::new(ClientState::default()))?;
            clients.push(client);
        }
        Ok(clients)
    }

    pub fn dispatch_and_flush(&mut self, state: &mut AuroraState) -> Result<(), Box<dyn std::error::Error>> {
        self.display.dispatch_clients(state)?;
        self.display.flush_clients()?;
        Ok(())
    }
}
