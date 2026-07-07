# AuroraWM

A fast, reliable, and beautiful Wayland compositor written in Rust.

## Features

- **High Performance**: Built with Rust and smithay for minimal latency
- **Multiple Layouts**: Dwindle, Master, Spiral, Columns, Grid, and Floating
- **Eye-Candy**: Smooth animations, blur effects, and shadows
- **Configurable**: TOML-based configuration with hot-reloading
- **Multi-Monitor**: Full support for multiple outputs
- **Input Handling**: Keyboard, mouse, touch, and tablet support via libinput
- **Window Rules**: Configure window behavior based on app-id and title

## Building

### Dependencies

- Rust 1.70 or later
- Wayland development libraries
- EGL and OpenGL development libraries
- libinput
- libseat
- GBM (for AMD/Intel GPUs)

On Arch Linux:
```bash
sudo pacman -S wayland libxkbcommon libinput libseat egl-wayland mesa
```

On Ubuntu/Debian:
```bash
sudo apt install libwayland-dev libxkbcommon-dev libinput-dev libseat-dev libegl1-mesa-dev
```

### Build from source

```bash
git clone https://github.com/yourusername/aurorawm.git
cd aurorawm
cargo build --release
```

The binary will be available at `target/release/aurorawm`.

## Installation

### System-wide installation

```bash
sudo cp target/release/aurorawm /usr/local/bin/
sudo chmod +x /usr/local/bin/aurorawm
```

### Create desktop entry

```bash
sudo cp aurorawm.desktop /usr/share/wayland-sessions/
```

## Configuration

Configuration is stored in `~/.config/aurorawm/config.toml`. A default configuration will be created on first run.

### Example Configuration

```toml
[general]
autostart = ["alacritty", "waybar"]
reload_config_on_change = true

[input.keyboard]
repeat_rate = 25
repeat_delay = 600
xkb_layout = "us"

[input.mouse]
accel_speed = 0.0
natural_scroll = false

[layout]
default = "dwindle"

[layout.gaps]
inner = 8
outer = 8

[layout.borders]
width = 2
focused = "#89b4fa"
unfocused = "#45475a"
urgent = "#f38ba8"

[appearance.opacity]
active = 1.0
inactive = 0.9

[appearance.blur]
enabled = true
size = 8
passes = 2

[appearance.animations]
enabled = true
duration_ms = 200
easing = "ease-out-cubic"

[keybindings]
modifier = "SUPER"

[[keybindings.bindings]]
keys = ["SUPER", "RETURN"]
command = "alacritty"
description = "Launch terminal"

[[keybindings.bindings]]
keys = ["SUPER", "Q"]
command = "close"
description = "Close window"

[[window_rules]]
app_id = "pavucontrol"
floating = true
```

## Keybindings

Default keybindings (configurable):

- `SUPER + RETURN` - Launch terminal
- `SUPER + Q` - Close focused window
- `SUPER + SHIFT + Q` - Quit compositor
- `SUPER + D` - Launch application launcher
- `SUPER + 1-9` - Switch to workspace
- `SUPER + SHIFT + 1-9` - Move window to workspace
- `SUPER + H/J/K/L` - Move focus (vim-style)
- `SUPER + SHIFT + H/J/K/L` - Move window
- `SUPER + F` - Toggle fullscreen
- `SUPER + SPACE` - Toggle floating
- `SUPER + R` - Cycle layouts

## Layouts

- **Dwindle**: Recursive split layout (default)
- **Master**: Master window on left, stack on right
- **Spiral**: Spiral layout similar to dwindle
- **Columns**: Column-based layout
- **Grid**: Equal grid layout
- **Floating**: Manual positioning

## Development

### Running in development mode

```bash
cargo run
```

### Running tests

```bash
cargo test
```

### Formatting code

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Built with [smithay](https://github.com/Smithay/smithay)
- Inspired by [Hyprland](https://github.com/hyprwm/Hyprland), [Sway](https://github.com/swaywm/sway), and [mangoHud](https://github.com/flightlessmango/MangoHud)
