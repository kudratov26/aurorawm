use std::time::Instant;

use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
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
        shm::{ShmHandler, ShmState},
    },
};

use crate::config::Config;
use crate::layout::LayoutEngine;

pub struct AuroraState {
    pub config: Config,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
    pub space: Space<Window>,
    pub popups: PopupManager,
    pub layout: LayoutEngine,
    pub output: Output,
    pub running: bool,
    pub start_time: Instant,
    pub last_cursor_pos: Point<f64, Logical>,
    pub pending_move: Option<Window>,
    pub pending_resize: Option<Window>,
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
        self.space.map_element(window.clone(), (0, 0), true);
        self.arrange_windows();

        window.toplevel().unwrap().send_configure();
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
    pub fn arrange_windows(&mut self) {
        let output_size = self
            .output
            .current_mode()
            .map(|m| m.size.to_logical(1))
            .unwrap_or((1280, 800).into());

        let output_area =
            smithay::utils::Rectangle::<i32, Logical>::new((0, 0).into(), output_size);

        let windows: Vec<Window> = self.space.elements().cloned().collect();
        let window_geos: Vec<_> = windows.iter().map(|w| w.geometry()).collect();

        if !window_geos.is_empty() {
            let positions = self.layout.arrange(&window_geos, output_area);

            for (window, geo) in windows.iter().zip(positions.iter()) {
                self.space.map_element(window.clone(), geo.loc, false);
                window.toplevel().unwrap().send_configure();
            }
        }
    }

    pub fn close_focused(&mut self) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        if let Some(active) = windows.last() {
            if active.toplevel().is_some() {
                self.space.unmap_elem(active);
            }
        }
        self.arrange_windows();
    }
}

delegate_compositor!(AuroraState);
delegate_xdg_shell!(AuroraState);
delegate_shm!(AuroraState);
delegate_seat!(AuroraState);
delegate_output!(AuroraState);
delegate_data_device!(AuroraState);

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
