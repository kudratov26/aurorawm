mod compositor;
mod config;
mod input;
mod layout;
mod output;
mod render;
mod shell;
mod state;
mod standalone;

use std::time::{Duration, Instant};

use smithay::{
    backend::{
        allocator::Fourcc,
        input::{
            AbsolutePositionEvent, ButtonState, InputEvent, KeyboardKeyEvent, PointerButtonEvent,
        },
        renderer::{
            element::{
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                texture::{TextureBuffer, TextureRenderElement},
                Kind,
            },
            gles::GlesRenderer,
            utils::draw_render_elements,
            Color32F, Frame, ImportMem, Renderer,
        },
        winit::{self, WinitEvent},
    },
    desktop::layer_map_for_output,
    input::keyboard::FilterResult,
    output::{Mode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::wayland_server::protocol::wl_surface,
    utils::{Point, Rectangle, Transform},
    wayland::{
        compositor::{with_surface_tree_downward, SurfaceAttributes, TraversalAction},
        shell::wlr_layer::Layer as WlrLayer,
    },
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

    let layer_shell_state =
        smithay::wayland::shell::wlr_layer::WlrLayerShellState::new::<AuroraState>(&dh);

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
        workspaces: vec![smithay::desktop::Space::default()],
        current_workspace: 0,
        popups: smithay::desktop::PopupManager::default(),
        layout,
        layer_shell_state,
        output,
        running: true,
        start_time: Instant::now(),
        last_cursor_pos: Point::from((0.0, 0.0)),
        mouse_mode: crate::state::MouseMode::None,
        drag_window: None,
        drag_offset: (0, 0).into(),
        wallpaper: None,
        animating: false,
        anim_start: Instant::now(),
        anim_duration: Duration::from_millis(200),
        anim_easing: String::new(),
        anim_windows: Vec::new(),
    };

    state.wallpaper = crate::render::load_wallpaper(
        state.config.appearance.wallpaper.as_deref(),
    );

    for ws in &mut state.workspaces {
        ws.map_output(&state.output, (0, 0));
    }

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

                    if state.mouse_mode == crate::state::MouseMode::Moving {
                        if let Some(window) = state.drag_window.clone() {
                            let new_pos = state.last_cursor_pos.to_i32_round()
                                - state.drag_offset;
                            state.space_mut().map_element(window, new_pos, false);
                        }
                    }
                }
                InputEvent::PointerButton { event } => {
                    let btn_state = event.state();

                    if btn_state == ButtonState::Pressed {
                        let is_super_held = input_manager.pressed_keys.contains(&64);

                        let clicked = state
                            .space()
                            .element_under(state.last_cursor_pos)
                            .map(|(w, _)| w.clone());

                        if let Some(window) = clicked {
                            state.space_mut().raise_element(&window, true);
                            if let Some(toplevel) = window.toplevel() {
                                let surface = toplevel.wl_surface().clone();
                                if let Some(keyboard) = state.seat.get_keyboard() {
                                    keyboard.set_focus(&mut state, Some(surface), 0.into());
                                }
                            }

                            if is_super_held {
                                let cursor_geo = state.last_cursor_pos.to_i32_round();
                                let win_loc = state
                                    .space()
                                    .element_location(&window)
                                    .unwrap_or((0, 0).into());
                                state.mouse_mode = crate::state::MouseMode::Moving;
                                state.drag_window = Some(window);
                                state.drag_offset = cursor_geo - win_loc;
                            }
                        } else if let Some(keyboard) = state.seat.get_keyboard() {
                            keyboard.set_focus(&mut state, None, 0.into());
                        }
                    } else {
                        if state.mouse_mode != crate::state::MouseMode::None {
                            state.mouse_mode = crate::state::MouseMode::None;
                            state.drag_window = None;
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

        let prev_count = state.space().elements().len();
        state.space_mut().refresh();
        if state.space().elements().len() != prev_count {
            state.arrange_windows();
        }

        state.update_animation();

        let size = backend.window_size();
        let damage = Rectangle::from_size(size);

        {
            let (renderer, mut framebuffer) = backend.bind().unwrap();

            let output = &state.output;
            let output_geo = state.space().output_geometry(output).unwrap_or_else(|| {
                Rectangle::from_size(
                    output
                        .current_mode()
                        .map(|m| m.size.to_logical(1))
                        .unwrap_or((1280, 800).into()),
                )
            });

            let output_size = smithay::utils::Size::<i32, smithay::utils::Logical>::from((
                size.w,
                size.h,
            ));

            let mut layer_map = layer_map_for_output(output);
            layer_map.arrange();
            layer_map.cleanup();
            let scale_f64 = output.current_scale().fractional_scale();
            let scale = smithay::utils::Scale::from(scale_f64);

            let collect_layer_elems = |layer: WlrLayer,
                                       renderer: &mut GlesRenderer,
                                       layer_map: &mut smithay::desktop::LayerMap|
             -> Vec<WaylandSurfaceRenderElement<GlesRenderer>> {
                layer_map
                    .layers_on(layer)
                    .filter_map(|ls| {
                        let geo = layer_map.layer_geometry(ls)?;
                        let loc = (geo.loc - output_geo.loc)
                            .to_physical_precise_round(scale_f64);
                        Some(render_elements_from_surface_tree(
                            renderer,
                            ls.wl_surface(),
                            loc,
                            scale,
                            1.0,
                            Kind::Unspecified,
                        ))
                    })
                    .flatten()
                    .collect()
            };

            let mut pre_layer_elems: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
                collect_layer_elems(WlrLayer::Background, renderer, &mut layer_map);
            pre_layer_elems.extend(collect_layer_elems(
                WlrLayer::Bottom,
                renderer,
                &mut layer_map,
            ));

            let wallpaper_elem = state
                .wallpaper
                .as_ref()
                .and_then(|wp| {
                    TextureBuffer::from_memory(
                        renderer,
                        &wp.rgba,
                        Fourcc::Abgr8888,
                        (wp.width as i32, wp.height as i32),
                        false,
                        wp.width as i32,
                        Transform::Normal,
                        None,
                    )
                    .ok()
                    .map(|tex_buf| {
                        TextureRenderElement::from_texture_buffer(
                            (0.0, 0.0),
                            &tex_buf,
                            Some(1.0),
                            None,
                            Some(output_size),
                            Kind::Unspecified,
                        )
                    })
                });

            let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = {
                let space = state.space();
                space
                    .elements()
                    .flat_map(|window| {
                        let location = space
                            .element_location(window)
                            .unwrap_or((0, 0).into());
                        let loc = (location - output_geo.loc).to_physical_precise_round(scale_f64);
                        render_elements_from_surface_tree(
                            renderer,
                            window.toplevel().unwrap().wl_surface(),
                            loc,
                            scale,
                            1.0,
                            Kind::Unspecified,
                        )
                    })
                    .collect()
            };

            let mut post_layer_elems: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
                collect_layer_elems(WlrLayer::Top, renderer, &mut layer_map);
            post_layer_elems.extend(collect_layer_elems(
                WlrLayer::Overlay,
                renderer,
                &mut layer_map,
            ));

            drop(layer_map);

            let mut frame = renderer
                .render(&mut framebuffer, size, Transform::Flipped180)
                .unwrap();

            frame
                .clear(Color32F::new(0.1, 0.1, 0.15, 1.0), &[damage])
                .unwrap();

            draw_render_elements(&mut frame, 1.0, &pre_layer_elems, &[damage]).unwrap();

            if let Some(tex_elem) = wallpaper_elem {
                draw_render_elements::<GlesRenderer, _, TextureRenderElement<smithay::backend::renderer::gles::GlesTexture>>(
                    &mut frame,
                    1.0,
                    &[tex_elem],
                    &[damage],
                )
                .unwrap();
            }

            draw_render_elements(&mut frame, 1.0, &elements, &[damage]).unwrap();
            draw_render_elements(&mut frame, 1.0, &post_layer_elems, &[damage]).unwrap();
            let _ = frame.finish().unwrap();

            let start_time = state.start_time;
            for window in state.space().elements() {
                if let Some(surface) = window.toplevel().map(|t| t.wl_surface()) {
                    send_frames_surface_tree(surface, start_time.elapsed().as_millis() as u32);
                }
            }

            compositor.accept_clients()?;
            compositor.dispatch_and_flush(&mut state)?;
        }

        backend.submit(Some(&[damage])).unwrap();
    }

    Ok(())
}
