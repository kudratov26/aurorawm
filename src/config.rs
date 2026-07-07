use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub input: InputConfig,
    pub output: OutputConfig,
    pub layout: LayoutConfig,
    pub appearance: AppearanceConfig,
    pub keybindings: KeybindingsConfig,
    pub window_rules: Vec<WindowRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub autostart: Vec<String>,
    pub env: Vec<(String, String)>,
    pub reload_config_on_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub keyboard: KeyboardConfig,
    pub mouse: MouseConfig,
    pub touch: TouchConfig,
    pub tablet: TabletConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardConfig {
    pub repeat_rate: i32,
    pub repeat_delay: i32,
    pub xkb_layout: String,
    pub xkb_options: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseConfig {
    pub accel_speed: f64,
    pub accel_profile: String,
    pub natural_scroll: bool,
    pub left_handed: bool,
    pub middle_emulation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchConfig {
    pub accel_speed: f64,
    pub accel_profile: String,
    pub tap_to_click: bool,
    pub drag_lock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabletConfig {
    pub relative_motion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub scale: u32,
    pub adaptive_sync: bool,
    pub mode: OutputMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub default: String,
    pub gaps: GapsConfig,
    pub borders: BorderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapsConfig {
    pub inner: u32,
    pub outer: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderConfig {
    pub width: u32,
    pub focused: String,
    pub unfocused: String,
    pub urgent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub opacity: OpacityConfig,
    pub blur: BlurConfig,
    pub shadows: ShadowConfig,
    pub animations: AnimationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpacityConfig {
    pub active: f64,
    pub inactive: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlurConfig {
    pub enabled: bool,
    pub size: u32,
    pub passes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowConfig {
    pub enabled: bool,
    pub blur_size: u32,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationConfig {
    pub enabled: bool,
    pub duration_ms: u32,
    pub easing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    pub modifier: String,
    pub bindings: Vec<KeyBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub keys: Vec<String>,
    pub command: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRule {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub floating: Option<bool>,
    pub fullscreen: Option<bool>,
    pub workspace: Option<u32>,
    pub size: Option<(u32, u32)>,
    pub position: Option<(i32, i32)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                autostart: vec![],
                env: vec![],
                reload_config_on_change: false,
            },
            input: InputConfig {
                keyboard: KeyboardConfig {
                    repeat_rate: 25,
                    repeat_delay: 600,
                    xkb_layout: "us".to_string(),
                    xkb_options: "".to_string(),
                },
                mouse: MouseConfig {
                    accel_speed: 0.0,
                    accel_profile: "adaptive".to_string(),
                    natural_scroll: false,
                    left_handed: false,
                    middle_emulation: false,
                },
                touch: TouchConfig {
                    accel_speed: 0.5,
                    accel_profile: "adaptive".to_string(),
                    tap_to_click: true,
                    drag_lock: false,
                },
                tablet: TabletConfig {
                    relative_motion: false,
                },
            },
            output: OutputConfig {
                scale: 1,
                adaptive_sync: true,
                mode: OutputMode {
                    width: 1920,
                    height: 1080,
                    refresh_rate: 60,
                },
            },
            layout: LayoutConfig {
                default: "dwindle".to_string(),
                gaps: GapsConfig {
                    inner: 8,
                    outer: 8,
                },
                borders: BorderConfig {
                    width: 2,
                    focused: "#89b4fa".to_string(),
                    unfocused: "#45475a".to_string(),
                    urgent: "#f38ba8".to_string(),
                },
            },
            appearance: AppearanceConfig {
                opacity: OpacityConfig {
                    active: 1.0,
                    inactive: 0.9,
                },
                blur: BlurConfig {
                    enabled: true,
                    size: 8,
                    passes: 2,
                },
                shadows: ShadowConfig {
                    enabled: true,
                    blur_size: 16,
                    color: "#000000".to_string(),
                },
                animations: AnimationConfig {
                    enabled: true,
                    duration_ms: 200,
                    easing: "ease-out-cubic".to_string(),
                },
            },
            keybindings: KeybindingsConfig {
                modifier: "SUPER".to_string(),
                bindings: vec![
                    KeyBinding {
                        keys: vec!["SUPER".to_string(), "RETURN".to_string()],
                        command: "alacritty".to_string(),
                        description: Some("Launch terminal".to_string()),
                    },
                    KeyBinding {
                        keys: vec!["SUPER".to_string(), "Q".to_string()],
                        command: "close".to_string(),
                        description: Some("Close window".to_string()),
                    },
                ],
            },
            window_rules: vec![],
        }
    }
}

pub fn load_config() -> Result<Config> {
    let config_path = get_config_path();
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;
        
        Ok(config)
    } else {
        // Create default config
        let default_config = Config::default();
        save_config(&default_config)?;
        Ok(default_config)
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_path();
    
    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let content = toml::to_string_pretty(config)?;
    fs::write(&config_path, content)?;
    
    Ok(())
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("aurorawm")
        .join("config.toml")
}
