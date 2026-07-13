use std::collections::HashSet;
use std::process::Command;

use smithay::backend::input::{KeyState, Keycode};

use crate::state::AuroraState;

pub struct InputManager {
    pub pressed_keys: HashSet<u32>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    pub fn handle_keyboard_event(
        &mut self,
        state: &mut AuroraState,
        keycode: Keycode,
        key_state: KeyState,
    ) {
        let key: u32 = keycode.raw();

        match key_state {
            KeyState::Pressed => {
                self.pressed_keys.insert(key);

                if key == 1 {
                    state.running = false;
                    return;
                }

                self.check_keybindings(state);
            }
            KeyState::Released => {
                self.pressed_keys.remove(&key);
            }
        }
    }

    fn check_keybindings(&self, state: &mut AuroraState) {
        let pressed: HashSet<String> = self
            .pressed_keys
            .iter()
            .map(|k| keycode_to_name(*k))
            .collect();

        let commands: Vec<String> = state
            .config
            .keybindings
            .bindings
            .iter()
            .filter(|binding| {
                let required: HashSet<String> = binding
                    .keys
                    .iter()
                    .map(|k| k.to_lowercase())
                    .collect();
                required.iter().all(|k| pressed.contains(k))
            })
            .map(|b| b.command.clone())
            .collect();

        for cmd in commands {
            execute_command(&cmd, state);
        }
    }
}

fn execute_command(command: &str, state: &mut AuroraState) {
    match command {
        "close" => {
            state.close_focused();
        }
        _ => {
            let _ = Command::new("sh").arg("-c").arg(command).spawn();
        }
    }
}

fn keycode_to_name(key: u32) -> String {
    match key {
        64 => "super".to_string(),
        36 => "return".to_string(),
        24 => "q".to_string(),
        38 => "a".to_string(),
        39 => "s".to_string(),
        40 => "d".to_string(),
        37 => "w".to_string(),
        _ => format!("key{}", key),
    }
}
