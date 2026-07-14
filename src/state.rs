use std::time::{Duration, Instant};

use smithay::{
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_output, delegate_seat,
    delegate_shm, delegate_xdg_shell,
    desktop::{PopupKind, PopupManager, Space, Window},
    input::{Seat, SeatHandler, SeatState},
    output::Output,
    reexports::wayland_server::{
        backend::{ClientData, ClientId, DisconnectReason},
        protocol::wl_output::WlOutput,
        protocol::wl_surface::WlSurface,
        Client,
    },
    utils::{Logical, Point},
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        output::OutputHandler,
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shell::wlr_layer::{WlrLayerShellHandler, WlrLayerShellState},
        shm::{ShmHandler, ShmState},
    },
};

use crate::config::Config;
use crate::layout::LayoutEngine;
use crate::render::WallpaperData;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    None,
    Moving,
    Resizing,
}

pub(crate) struct AnimEntry {
    window: Window,
    old_pos: Point<i32, Logical>,
    new_pos: Point<i32, Logical>,
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn apply_easing(t: f64, easing: &str) -> f64 {
    match easing {
        "linear" => t,
        "ease-out-cubic" => ease_out_cubic(t),
        "ease-in-out-cubic" => ease_in_out_cubic(t),
        _ => ease_out_cubic(t),
    }
}

pub struct AuroraState {
    pub config: Config,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
    pub workspaces: Vec<Space<Window>>,
    pub current_workspace: usize,
    pub popups: PopupManager,
    pub layout: LayoutEngine,
    pub output: Output,
    pub layer_shell_state: WlrLayerShellState,
    pub running: bool,
    pub start_time: Instant,
    pub last_cursor_pos: Point<f64, Logical>,
    pub mouse_mode: MouseMode,
    pub drag_window: Option<Window>,
    pub drag_offset: Point<i32, Logical>,
    pub wallpaper: Option<WallpaperData>,
    pub animating: bool,
    pub anim_start: Instant,
    pub anim_duration: Duration,
    pub anim_easing: String,
    pub(crate) anim_windows: Vec<AnimEntry>,
}

impl BufferHandler for AuroraState {
    fn buffer_destroyed(&mut self, _buffer: &wayland_server::protocol::wl_buffer::WlBuffer) {}
}

impl OutputHandler for AuroraState {
    fn output_bound(&mut self, _output: Output, _wl_output: WlOutput) {}
}

impl XdgShellHandler for AuroraState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface);
        self.space_mut().map_element(window.clone(), (0, 0), true);
        self.space_mut().raise_element(&window, true);
        if let Some(toplevel) = window.toplevel() {
            let s = toplevel.wl_surface().clone();
            if let Some(kb) = self.seat.get_keyboard() {
                kb.set_focus(self, Some(s), 0.into());
            }
        }
        self.arrange_windows();
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        let _ = self.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn grab(
        &mut self,
        _surface: PopupSurface,
        _seat: wayland_server::protocol::wl_seat::WlSeat,
        _serial: smithay::utils::Serial,
    ) {
    }

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }
}

impl WlrLayerShellHandler for AuroraState {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        output: Option<WlOutput>,
        _layer: smithay::wayland::shell::wlr_layer::Layer,
        namespace: String,
    ) {
        let desktop_surface = smithay::desktop::LayerSurface::new(surface, namespace);

        let target_output = output
            .as_ref()
            .and_then(|wl| Output::from_resource(wl))
            .unwrap_or_else(|| self.output.clone());

        let output_size = target_output
            .current_mode()
            .map(|m| m.size.to_logical(1))
            .unwrap_or((1280, 800).into());

        desktop_surface.layer_surface().with_pending_state(|state| {
            state.size = Some(output_size);
        });
        desktop_surface.layer_surface().send_configure();

        let mut layer_map =
            smithay::desktop::layer_map_for_output(&target_output);
        let _ = layer_map.map_layer(&desktop_surface);
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {
        let mut layer_map =
            smithay::desktop::layer_map_for_output(&self.output);
        let to_remove: Vec<smithay::desktop::LayerSurface> = layer_map
            .layers()
            .filter(|ls| ls.layer_surface().wl_surface() == surface.wl_surface())
            .cloned()
            .collect();
        for ds in to_remove {
            layer_map.unmap_layer(&ds);
        }
    }
}

impl SelectionHandler for AuroraState {
    type SelectionUserData = ();
}

impl DataDeviceHandler for AuroraState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for AuroraState {}
impl ServerDndGrabHandler for AuroraState {
    fn send(&mut self, _mime_type: String, _fd: std::os::unix::io::OwnedFd, _seat: Seat<Self>) {}
}

impl CompositorHandler for AuroraState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a Client,
    ) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        self.popups.commit(surface);
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
    }
}

impl ShmHandler for AuroraState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for AuroraState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}

impl AuroraState {
    pub fn space(&self) -> &Space<Window> {
        &self.workspaces[self.current_workspace]
    }

    pub fn space_mut(&mut self) -> &mut Space<Window> {
        &mut self.workspaces[self.current_workspace]
    }

    pub fn switch_workspace(&mut self, idx: usize) {
        if idx < self.workspaces.len() && idx != self.current_workspace {
            self.workspaces.swap(self.current_workspace, idx);
            self.current_workspace = idx;
            self.arrange_windows();
        }
    }

    pub fn arrange_windows(&mut self) {
        let output_size = self
            .output
            .current_mode()
            .map(|m| m.size.to_logical(1))
            .unwrap_or((1280, 800).into());

        let output_area =
            smithay::utils::Rectangle::<i32, Logical>::new((0, 0).into(), output_size);

        let windows: Vec<Window> = self.space().elements().cloned().collect();
        let window_geos: Vec<_> = windows.iter().map(|w| w.geometry()).collect();

        if window_geos.is_empty() {
            return;
        }

        let old_positions: Vec<(Window, Point<i32, Logical>)> = windows
            .iter()
            .map(|w| {
                (
                    w.clone(),
                    self.space().element_location(w).unwrap_or((0, 0).into()),
                )
            })
            .collect();

        let positions = self.layout.arrange(&window_geos, output_area);

        let anim_dur = Duration::from_millis(
            self.config.appearance.animations.duration_ms as u64,
        );
        let anim_enabled = self.config.appearance.animations.enabled;

        for (window, geo) in windows.iter().zip(positions.iter()) {
            self.space_mut().map_element(window.clone(), geo.loc, false);
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| {
                    state.size = Some(smithay::utils::Size::from((geo.size.w.max(1), geo.size.h.max(1))));
                });
                toplevel.send_configure();
            }
        }

        if anim_enabled {
            let new_positions: Vec<(Window, Point<i32, Logical>)> = windows
                .iter()
                .map(|w| {
                    (
                        w.clone(),
                        self.space().element_location(w).unwrap_or((0, 0).into()),
                    )
                })
                .collect();

            self.anim_windows = old_positions
                .into_iter()
                .zip(new_positions.into_iter())
                .map(|((w, old), (_, new))| AnimEntry {
                    window: w,
                    old_pos: old,
                    new_pos: new,
                })
                .filter(|e| e.old_pos != e.new_pos)
                .collect();

            if !self.anim_windows.is_empty() {
                self.animating = true;
                self.anim_start = Instant::now();
                self.anim_duration = anim_dur;
                self.anim_easing = self.config.appearance.animations.easing.clone();
                let first = self.anim_windows[0].window.clone();
                let first_pos = self.anim_windows[0].old_pos;
                self.space_mut().map_element(first, first_pos, false);
            }
        }
    }

    pub fn update_animation(&mut self) {
        if !self.animating {
            return;
        }

        let elapsed = self.anim_start.elapsed();
        let t = (elapsed.as_secs_f64() / self.anim_duration.as_secs_f64()).min(1.0);

        if t >= 1.0 {
            self.animating = false;
            let entries: Vec<_> = self.anim_windows.drain(..).collect();
            for entry in &entries {
                self.space_mut()
                    .map_element(entry.window.clone(), entry.new_pos, false);
            }
            return;
        }

        let eased = apply_easing(t, &self.anim_easing);

        let entries: Vec<_> = self.anim_windows.iter().map(|e| {
            let x = e.old_pos.x as f64 + (e.new_pos.x - e.old_pos.x) as f64 * eased;
            let y = e.old_pos.y as f64 + (e.new_pos.y - e.old_pos.y) as f64 * eased;
            (e.window.clone(), Point::from((x.round() as i32, y.round() as i32)))
        }).collect();

        for (window, pos) in &entries {
            self.space_mut().map_element(window.clone(), *pos, false);
        }
    }

    pub fn close_focused(&mut self) {
        let windows: Vec<Window> = self.space().elements().cloned().collect();
        if let Some(active) = windows.last() {
            if let Some(toplevel) = active.toplevel() {
                toplevel.send_close();
            }
        }
    }
}

delegate_compositor!(AuroraState);
delegate_xdg_shell!(AuroraState);
delegate_shm!(AuroraState);
delegate_seat!(AuroraState);
delegate_output!(AuroraState);
delegate_data_device!(AuroraState);
delegate_layer_shell!(AuroraState);

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
