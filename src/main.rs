mod compositor;
mod config;
mod input;
mod layout;
mod output;
mod shell;
mod state;
mod standalone;

use std::time::Instant;

use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, InputEvent, KeyboardKeyEvent,
        },
        renderer::{
            element::{
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                Kind,
            },
            gles::GlesRenderer,
            utils::draw_render_elements,
            Color32F, Frame, Renderer,
        },
        winit::{self, WinitEvent},
    },
    input::keyboard::FilterResult,
    output::{Mode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::wayland_server::protocol::wl_surface,
    utils::{Point, Rectangle, Transform},
    wayland::compositor::{with_surface_tree_downward, SurfaceAttributes, TraversalAction},
};

use crate::compositor::AuroraCompositor;
use crate::config::Config;
use crate::input::InputManager;
use crate::layout::LayoutEngine;
use crate::state::AuroraState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    let config = config::load_config()?;

    let backend = std::env::var("AURORAWM_BACKEND").unwrap_or_default();

    if backend == "standalone" || backend == "drm" {
        standalone::run_standalone(config)
    } else if backend == "winit" || backend == "nested" {
        run_winit(config)
    } else {
        let has_display =
            std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();
        if has_display {
            run_winit(config)
        } else {
            standalone::run_standalone(config)
        }
    }
}

fn send_frames_surface_tree(surface: &wl_surface::WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}

pub fn run_winit(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut compositor = AuroraCompositor::new()?;
    let dh = compositor.display.handle();

    let compositor_state = smithay::wayland::compositor::CompositorState::new::<AuroraState>(&dh);
    let shm_state = smithay::wayland::shm::ShmState::new::<AuroraState>(&dh, vec![]);
    let mut seat_state = smithay::input::SeatState::new();
    let seat = seat_state.new_wl_seat(&dh, "seat-0");

    let layout = LayoutEngine::new(&config);

    let output = Output::new(
        "aurorawm-0".into(),
        PhysicalProperties {
            size: (480, 300).into(),
            subpixel: Subpixel::Unknown,
            make: "AuroraWM".into(),
            model: "Virtual Output".into(),
        },
    );
    output.create_global::<AuroraState>(&dh);

    let default_mode = Mode {
        size: (1280, 800).into(),
        refresh: 60000,
    };
    output.change_current_state(
        Some(default_mode),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );

    let mut state = AuroraState {
        config,
        compositor_state,
        xdg_shell_state: smithay::wayland::shell::xdg::XdgShellState::new::<AuroraState>(&dh),
        shm_state,
        seat_state,
        data_device_state: smithay::wayland::selection::data_device::DataDeviceState::new::<
            AuroraState,
        >(&dh),
        seat,
        space: smithay::desktop::Space::default(),
        popups: smithay::desktop::PopupManager::default(),
        layout,
        output,
        running: true,
        start_time: Instant::now(),
        last_cursor_pos: Point::from((0.0, 0.0)),
        pending_move: None,
        pending_resize: None,
    };

    state.space.map_output(&state.output, (0, 0));

    let (mut backend, mut winit_event_loop) = winit::init::<GlesRenderer>()?;

    std::env::set_var("WAYLAND_DISPLAY", "aurorawm-0");

    for cmd in &state.config.general.autostart {
        let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
    }

    let keyboard = state
        .seat
        .add_keyboard(Default::default(), 200, 200)
        .unwrap();

    let mut input_manager = InputManager::new();

    while state.running {
        let _status = winit_event_loop.dispatch_new_events(|event| match event {
            WinitEvent::Input(event) => match event {
                InputEvent::Keyboard { event } => {
                    input_manager.handle_keyboard_event(
                        &mut state,
                        event.key_code(),
                        event.state(),
                    );
                    keyboard.input::<(), _>(
                        &mut state,
                        event.key_code(),
                        event.state(),
                        0.into(),
                        0,
                        |_, _, _| FilterResult::Forward,
                    );
                }
                InputEvent::PointerMotionAbsolute { event } => {
                    let size = state
                        .output
                        .current_mode()
                        .map(|m| m.size.to_logical(1))
                        .unwrap_or((1280, 800).into());
                    let pos = event.position_transformed(size);
                    state.last_cursor_pos = pos;
                }
                InputEvent::PointerButton { event: _event } => {
                    let windows: Vec<_> = state.space.elements().cloned().collect();
                    if let Some(window) = windows.last() {
                        state.space.raise_element(window, true);
                        if let Some(toplevel) = window.toplevel() {
                            let surface = toplevel.wl_surface().clone();
                            if let Some(keyboard) = state.seat.get_keyboard() {
                                keyboard.set_focus(&mut state, Some(surface), 0.into());
                            }
                        }
                    }
                }
                _ => {}
            },
            WinitEvent::CloseRequested => state.running = false,
            WinitEvent::Resized { size, .. } => {
                state.output.change_current_state(
                    Some(Mode {
                        size,
                        refresh: 60000,
                    }),
                    Some(Transform::Normal),
                    Some(Scale::Integer(1)),
                    Some((0, 0).into()),
                );
            }
            _ => {}
        });

        state.space.refresh();

        let size = backend.window_size();
        let damage = Rectangle::from_size(size);

        {
            let (renderer, mut framebuffer) = backend.bind().unwrap();

            let output = &state.output;
            let output_geo = state.space.output_geometry(output).unwrap_or_else(|| {
                Rectangle::from_size(
                    output
                        .current_mode()
                        .map(|m| m.size.to_logical(1))
                        .unwrap_or((1280, 800).into()),
                )
            });

            let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = state
                .space
                .elements()
                .flat_map(|window| {
                    let location = state
                        .space
                        .element_location(window)
                        .unwrap_or((0, 0).into());
                    let loc = (location - output_geo.loc).to_physical_precise_round(
                        output.current_scale().fractional_scale(),
                    );
                    let scale = smithay::utils::Scale::from(output.current_scale().fractional_scale());
                    render_elements_from_surface_tree(
                        renderer,
                        window.toplevel().unwrap().wl_surface(),
                        loc,
                        scale,
                        1.0,
                        Kind::Unspecified,
                    )
                })
                .collect();

            let mut frame = renderer
                .render(&mut framebuffer, size, Transform::Flipped180)
                .unwrap();
            frame
                .clear(Color32F::new(0.1, 0.1, 0.15, 1.0), &[damage])
                .unwrap();
            draw_render_elements(&mut frame, 1.0, &elements, &[damage]).unwrap();
            let _ = frame.finish().unwrap();

            for window in state.space.elements() {
                if let Some(surface) = window.toplevel().map(|t| t.wl_surface()) {
                    send_frames_surface_tree(surface, state.start_time.elapsed().as_millis() as u32);
                }
            }

            compositor.accept_clients()?;
            compositor.dispatch_and_flush(&mut state)?;
        }

        backend.submit(Some(&[damage])).unwrap();
    }

    Ok(())
}
