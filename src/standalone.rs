use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
use smithay::backend::allocator::{Format, Fourcc};
use smithay::backend::renderer::element::texture::{TextureBuffer, TextureRenderElement};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::ImportMem;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmEvent, GbmBufferedSurface};
use smithay::backend::egl::{EGLContext, EGLDisplay};
use smithay::backend::input::KeyState;
use smithay::backend::libinput::LibinputSessionInterface;
use smithay::backend::renderer::element::surface::{
    render_elements_from_surface_tree, WaylandSurfaceRenderElement,
};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::draw_render_elements;
use smithay::backend::renderer::{Bind, Color32F, Frame, Renderer};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SessionEvent, Session};

use smithay::desktop::layer_map_for_output;
use smithay::input::keyboard::FilterResult;
use smithay::output::{Output, PhysicalProperties, Scale, Subpixel};
use smithay::reexports::calloop;
use smithay::reexports::drm::control::{connector, crtc, Device as ControlDevice, Mode as DrmMode};
use smithay::reexports::input as libinput_rs;
use smithay::reexports::input::event::keyboard::KeyboardEventTrait;
use smithay::reexports::rustix::fs::OFlags;
use smithay::utils::{Point, Rectangle, Size, Transform};
use smithay::wayland::compositor::{with_surface_tree_downward, SurfaceAttributes, TraversalAction};
use smithay::wayland::compositor::CompositorState;
use smithay::wayland::shell::wlr_layer::Layer as WlrLayer;
use tracing::{info, trace, warn};

use crate::compositor::AuroraCompositor;
use crate::config::Config;
use crate::input::InputManager;
use crate::state::AuroraState;

fn send_frames_surface_tree(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    time: u32,
) {
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

fn find_first_connector(
    drm: &DrmDevice,
) -> Option<(connector::Handle, crtc::Handle, DrmMode)> {
    let resources = drm.resource_handles().ok()?;
    let crtcs = resources.crtcs();
    for conn_handle in resources.connectors() {
        let conn_info = drm.get_connector(*conn_handle, false).ok()?;
        if conn_info.state() != connector::State::Connected {
            continue;
        }
        let drm_mode = conn_info.modes().first()?.clone();
        let crtc_handle = *crtcs.first()?;
        return Some((*conn_handle, crtc_handle, drm_mode));
    }
    None
}

pub fn run_standalone(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting AuroraWM standalone (DRM/KMS)");

    let mut compositor = AuroraCompositor::new()?;
    let dh = compositor.display.handle();

    let compositor_state = CompositorState::new::<AuroraState>(&dh);
    let shm_state = smithay::wayland::shm::ShmState::new::<AuroraState>(&dh, vec![]);
    let mut seat_state = smithay::input::SeatState::new();
    let seat = seat_state.new_wl_seat(&dh, "seat-0");
    let layout = crate::layout::LayoutEngine::new(&config);

    let layer_shell_state =
        smithay::wayland::shell::wlr_layer::WlrLayerShellState::new::<AuroraState>(&dh);

    let (mut session, session_notifier) = LibSeatSession::new()?;
    let seat_name = session.seat();
    info!("Session: seat={}", seat_name);

    let gpu_path = smithay::backend::udev::primary_gpu(&seat_name)?
        .ok_or("No primary GPU found")?;
    info!("GPU: {:?}", gpu_path);

    let drm_fd = session.open(&gpu_path, OFlags::RDWR | OFlags::CLOEXEC)?;
    let (mut drm_device, drm_notifier) = DrmDevice::new(DrmDeviceFd::new(drm_fd.into()), false)?;

    let (conn_handle, crtc_handle, drm_mode) =
        find_first_connector(&drm_device).ok_or("No connected connector found")?;
    let output_mode: smithay::output::Mode = drm_mode.into();
    let output_size = (output_mode.size.w, output_mode.size.h);
    info!("Output: {}x{}", output_size.0, output_size.1);

    let drm_surface = drm_device.create_surface(crtc_handle, drm_mode, &[conn_handle])?;

    let gbm_fd = session.open(&gpu_path, OFlags::RDWR | OFlags::CLOEXEC)?;
    let gbm_dev = GbmDevice::new(gbm_fd)?;

    let allocator = GbmAllocator::new(
        gbm_dev,
        GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
    );

    let color_formats = [Fourcc::Argb8888, Fourcc::Xrgb8888];
    let renderer_formats = vec![Format {
        code: Fourcc::Argb8888,
        modifier: smithay::backend::allocator::Modifier::Invalid,
    }];

    let gbm_surface = GbmBufferedSurface::new(
        drm_surface,
        allocator,
        &color_formats,
        renderer_formats,
    )
    .map_err(|e| format!("GBM surface: {:?}", e))?;
    let gbm_surface = Rc::new(RefCell::new(gbm_surface));

    let egl_gbm = GbmDevice::new(session.open(&gpu_path, OFlags::RDWR | OFlags::CLOEXEC)?)?;
    let egl_display = unsafe { EGLDisplay::new(egl_gbm)? };
    let egl_context = EGLContext::new(&egl_display)?;
    let renderer = RefCell::new(unsafe { GlesRenderer::new(egl_context)? });

    let output = Output::new(
        "aurorawm-0".into(),
        PhysicalProperties {
            size: (output_size.0 as i32, output_size.1 as i32).into(),
            subpixel: Subpixel::Unknown,
            make: "AuroraWM".into(),
            model: "DRM Output".into(),
        },
    );
    output.create_global::<AuroraState>(&dh);
    output.change_current_state(
        Some(output_mode),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );

    let wallpaper = crate::render::load_wallpaper(
        config.appearance.wallpaper.as_deref(),
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
        output: output.clone(),
        layer_shell_state,
        running: true,
        start_time: Instant::now(),
        last_cursor_pos: Point::from((0.0f64, 0.0f64)),
        mouse_mode: crate::state::MouseMode::None,
        drag_window: None,
        drag_offset: (0, 0).into(),
        wallpaper,
        animating: false,
        anim_start: Instant::now(),
        anim_duration: Duration::from_millis(200),
        anim_easing: String::new(),
        anim_windows: Vec::new(),
    };

    for ws in &mut state.workspaces {
        ws.map_output(&state.output, (0, 0));
    }

    let keyboard = state.seat.add_keyboard(Default::default(), 200, 200)?;
    let mut input_manager = InputManager::new();

    std::env::set_var("WAYLAND_DISPLAY", "aurorawm-0");

    for cmd in &state.config.general.autostart {
        let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
    }

    let start_time = Instant::now();

    let input_session = session.clone();
    let interface = LibinputSessionInterface::from(input_session);
    let mut lib_ctx = libinput_rs::Libinput::new_with_udev(interface);
    lib_ctx
        .udev_assign_seat(&seat_name)
        .map_err(|e| format!("libinput seat: {:?}", e))?;
    drop(session);

    let mut event_loop: calloop::EventLoop<'_, ()> = calloop::EventLoop::try_new()?;
    let loop_handle = event_loop.handle();

    let session_active = Rc::new(std::cell::Cell::new(true));

    {
        let active = session_active.clone();
        loop_handle.insert_source(session_notifier, move |event, _, _| match event {
            SessionEvent::PauseSession => {
                info!("Session paused");
                active.set(false);
            }
            SessionEvent::ActivateSession => {
                info!("Session activated");
                active.set(true);
            }
        })?;
    }

    {
        let gs = gbm_surface.clone();
        loop_handle.insert_source(drm_notifier, move |event: DrmEvent, _, _| {
            if let DrmEvent::VBlank(_) = event {
                let _ = gs.borrow_mut().frame_submitted();
            }
        })?;
    }

    info!("Entering main loop");
    let mut running = true;
    while running {
        let _ = event_loop.dispatch(Duration::ZERO, &mut ());

        if lib_ctx.dispatch().is_ok() {
            for event in &mut lib_ctx {
                match event {
                    libinput_rs::Event::Keyboard(kb_event) => {
                        let keycode: smithay::backend::input::Keycode =
                            (kb_event.key() + 8).into();
                        let keystate = match kb_event.key_state() {
                            libinput_rs::event::keyboard::KeyState::Pressed => {
                                KeyState::Pressed
                            }
                            libinput_rs::event::keyboard::KeyState::Released => {
                                KeyState::Released
                            }
                        };
                        input_manager.handle_keyboard_event(&mut state, keycode, keystate);
                        keyboard.input::<(), _>(
                            &mut state,
                            keycode,
                            keystate,
                            0.into(),
                            0,
                            |_, _, _| FilterResult::Forward,
                        );
                    }
                    libinput_rs::Event::Pointer(ptr_event) => match ptr_event {
                        libinput_rs::event::PointerEvent::Motion(motion) => {
                            let dx = motion.dx();
                            let dy = motion.dy();
                            state.last_cursor_pos = Point::from((
                                state.last_cursor_pos.x + dx,
                                state.last_cursor_pos.y + dy,
                            ));

                            if state.mouse_mode == crate::state::MouseMode::Moving {
                                if let Some(window) = state.drag_window.clone() {
                                    let new_pos = state.last_cursor_pos.to_i32_round()
                                        - state.drag_offset;
                                    state.space_mut().map_element(window, new_pos, false);
                                }
                            }
                        }
                        libinput_rs::event::PointerEvent::MotionAbsolute(abs_event) => {
                            let abs_x = abs_event.absolute_x_transformed(
                                state
                                    .output
                                    .current_mode()
                                    .map(|m| m.size.w as u32)
                                    .unwrap_or(1920),
                            );
                            let abs_y = abs_event.absolute_y_transformed(
                                state
                                    .output
                                    .current_mode()
                                    .map(|m| m.size.h as u32)
                                    .unwrap_or(1080),
                            );
                            state.last_cursor_pos = Point::from((abs_x, abs_y));

                            if state.mouse_mode == crate::state::MouseMode::Moving {
                                if let Some(window) = state.drag_window.clone() {
                                    let new_pos = state.last_cursor_pos.to_i32_round()
                                        - state.drag_offset;
                                    state.space_mut().map_element(window, new_pos, false);
                                }
                            }
                        }
                        libinput_rs::event::PointerEvent::Button(btn_event) => {
                            let is_pressed = btn_event.button_state()
                                == libinput_rs::event::pointer::ButtonState::Pressed;
                            let is_super_held =
                                input_manager.pressed_keys.contains(&64);

                            if is_pressed {
                                let clicked = state
                                    .space()
                                    .element_under(state.last_cursor_pos)
                                    .map(|(w, _)| w.clone());

                                if let Some(window) = clicked {
                                    state.space_mut().raise_element(&window, true);
                                    if let Some(toplevel) = window.toplevel() {
                                        let surface = toplevel.wl_surface().clone();
                                        if let Some(keyboard) = state.seat.get_keyboard() {
                                            keyboard.set_focus(
                                                &mut state,
                                                Some(surface),
                                                0.into(),
                                            );
                                        }
                                    }

                                    if is_super_held {
                                        let cursor_geo =
                                            state.last_cursor_pos.to_i32_round();
                                        let win_loc = state
                                            .space()
                                            .element_location(&window)
                                            .unwrap_or((0, 0).into());
                                        state.mouse_mode =
                                            crate::state::MouseMode::Moving;
                                        state.drag_window = Some(window);
                                        state.drag_offset = cursor_geo - win_loc;
                                    }
                                } else if let Some(keyboard) = state.seat.get_keyboard() {
                                    keyboard.set_focus(&mut state, None, 0.into());
                                }
                            } else {
                                if state.mouse_mode
                                    != crate::state::MouseMode::None
                                {
                                    state.mouse_mode =
                                        crate::state::MouseMode::None;
                                    state.drag_window = None;
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        if !session_active.get() {
            std::thread::sleep(Duration::from_millis(10));
            let _ = compositor.dispatch_and_flush(&mut state);
            continue;
        }

        let prev_count = state.space().elements().len();
        state.space_mut().refresh();
        if state.space().elements().len() != prev_count {
            state.arrange_windows();
        }

        state.update_animation();

        let size = Size::from((output_mode.size.w, output_mode.size.h));
        let damage = Rectangle::from_size(size);
        let full_damage = vec![damage];

        let (mut dmabuf, _age) = match gbm_surface.borrow_mut().next_buffer() {
            Ok(r) => r,
            Err(e) => {
                trace!("next_buffer: {:?}", e);
                std::thread::sleep(Duration::from_millis(1));
                let _ = compositor.accept_clients();
                let _ = compositor.dispatch_and_flush(&mut state);
                continue;
            }
        };

        let output = state.output.clone();
        let output_geo = state
            .space()
            .output_geometry(&output)
            .unwrap_or_else(|| Rectangle::from_size(output_mode.size.to_logical(1)));

        let output_size = smithay::utils::Size::<i32, smithay::utils::Logical>::from((
            output_mode.size.w,
            output_mode.size.h,
        ));

        let mut layer_map = layer_map_for_output(&output);
        layer_map.arrange();
        layer_map.cleanup();
        let scale_f64 = output.current_scale().fractional_scale();
        let scale = smithay::utils::Scale::from(scale_f64);

        let collect_layer = |layer: WlrLayer,
                             renderer: &std::cell::RefCell<GlesRenderer>,
                             layer_map: &smithay::desktop::LayerMap|
         -> Vec<WaylandSurfaceRenderElement<GlesRenderer>> {
            let mut r = renderer.borrow_mut();
            layer_map
                .layers_on(layer)
                .filter_map(|ls| {
                    let geo = layer_map.layer_geometry(ls)?;
                    let loc = (geo.loc - output_geo.loc)
                        .to_physical_precise_round(scale_f64);
                    Some(render_elements_from_surface_tree(
                        &mut *r,
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
            collect_layer(WlrLayer::Background, &renderer, &layer_map);
        pre_layer_elems.extend(collect_layer(
            WlrLayer::Bottom,
            &renderer,
            &layer_map,
        ));

        let wallpaper_elem = state.wallpaper.as_ref().and_then(|wp| {
            let mut r = renderer.borrow_mut();
            TextureBuffer::from_memory(
                &mut *r,
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
            let mut r = renderer.borrow_mut();
            let space = state.space();
            space
                .elements()
                .flat_map(|window| {
                    let location = space
                        .element_location(window)
                        .unwrap_or((0, 0).into());
                    let loc = (location - output_geo.loc).to_physical_precise_round(
                        scale_f64,
                    );
                    render_elements_from_surface_tree(
                        &mut *r,
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
            collect_layer(WlrLayer::Top, &renderer, &layer_map);
        post_layer_elems.extend(collect_layer(
            WlrLayer::Overlay,
            &renderer,
            &layer_map,
        ));

        drop(layer_map);

        let mut r = renderer.borrow_mut();
        let mut target = match r.bind(&mut dmabuf) {
            Ok(t) => t,
            Err(e) => {
                warn!("bind dmabuf: {:?}", e);
                continue;
            }
        };

        let mut frame = match r.render(&mut target, size, Transform::Normal) {
            Ok(f) => f,
            Err(e) => {
                warn!("render: {:?}", e);
                continue;
            }
        };

        let _ = frame.clear(Color32F::new(0.1, 0.1, 0.15, 1.0), &full_damage);

        let _ = draw_render_elements(&mut frame, 1.0, &pre_layer_elems, &full_damage);

        if let Some(tex_elem) = wallpaper_elem {
            let _ = draw_render_elements::<GlesRenderer, _, TextureRenderElement<smithay::backend::renderer::gles::GlesTexture>>(
                &mut frame,
                1.0,
                &[tex_elem],
                &full_damage,
            );
        }

        let _ = draw_render_elements::<GlesRenderer, _, WaylandSurfaceRenderElement<GlesRenderer>>(
            &mut frame,
            1.0,
            &elements,
            &full_damage,
        );

        let _ = draw_render_elements(&mut frame, 1.0, &post_layer_elems, &full_damage);

        if let Err(e) = frame.finish() {
            warn!("finish: {:?}", e);
            continue;
        }
        drop(target);
        drop(r);
        drop(dmabuf);

        if let Err(e) = gbm_surface
            .borrow_mut()
            .queue_buffer(None, Some(full_damage), ())
        {
            warn!("queue_buffer: {:?}", e);
        }

        for window in state.space().elements() {
            if let Some(surface) = window.toplevel().map(|t| t.wl_surface()) {
                send_frames_surface_tree(surface, start_time.elapsed().as_millis() as u32);
            }
        }

        let _ = compositor.accept_clients();
        let _ = compositor.dispatch_and_flush(&mut state);

        if !state.running {
            running = false;
        }
    }

    info!("AuroraWM exiting");
    Ok(())
}
