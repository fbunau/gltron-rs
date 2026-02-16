#![allow(unused_variables, unused_imports, dead_code)]

mod renderer;
mod types;
mod assets;
mod physics;
mod game;
mod ai;
mod camera;
mod input;
mod render;
mod world;
mod trail;
mod effects;
mod menu;
mod gamepad;
#[cfg(target_arch = "wasm32")]
mod embedded_assets;

use std::num::NonZeroU32;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};
#[cfg(not(target_arch = "wasm32"))]
use glutin::config::{ConfigTemplateBuilder, GlConfig};
#[cfg(not(target_arch = "wasm32"))]
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version};
#[cfg(not(target_arch = "wasm32"))]
use glutin::display::{Display, GetGlDisplay, GlDisplay};
#[cfg(not(target_arch = "wasm32"))]
use glutin::surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
#[cfg(not(target_arch = "wasm32"))]
use glutin_winit::DisplayBuilder;
#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::HasWindowHandle;
use renderer::{Renderer, Vertex, fan_verts, line_loop_verts};
use types::*;

#[cfg(not(target_arch = "wasm32"))]
fn save_screenshot(r: &mut Renderer, w: u32, h: u32) {
    use std::path::Path;
    let dir = "screenshots";
    if !Path::new(dir).exists() {
        std::fs::create_dir_all(dir).ok();
    }

    let mut pixels = vec![0u8; (w * h * 3) as usize];
    r.pixel_store_i(renderer::PACK_ALIGNMENT, 1);
    r.read_pixels(0, 0, w as i32, h as i32, renderer::RGB, renderer::UNSIGNED_BYTE, &mut pixels);

    // Flip vertically (GL reads bottom-up)
    let row_bytes = (w * 3) as usize;
    for y in 0..h as usize / 2 {
        let top = y * row_bytes;
        let bot = (h as usize - 1 - y) * row_bytes;
        for x in 0..row_bytes {
            pixels.swap(top + x, bot + x);
        }
    }

    // Timestamp filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = format!("{dir}/screenshot_{timestamp}.png");

    if let Some(img) = image::RgbImage::from_raw(w, h, pixels) {
        if img.save(&path).is_ok() {
            eprintln!("Screenshot saved: {path}");
        }
    }
}

/// Draw a pill (rounded rect) background + border for pause overlays.
fn draw_pill(r: &mut Renderer, px: f32, py: f32, pw: f32, ph: f32, radius: f32) {
    let segs = 8;
    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    let rad = radius.min(ph / 2.0).min(pw / 2.0);
    let corners = [
        (px + rad, py + rad, std::f32::consts::PI, std::f32::consts::FRAC_PI_2 * 3.0),
        (px + pw - rad, py + rad, std::f32::consts::FRAC_PI_2 * 3.0, std::f32::consts::TAU),
        (px + pw - rad, py + ph - rad, 0.0_f32, std::f32::consts::FRAC_PI_2),
        (px + rad, py + ph - rad, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),
    ];

    // Build perimeter points for the rounded rect
    let fill_color = [0.02, 0.03, 0.08, 0.88];
    let mut perimeter = Vec::new();
    for &(cx, cy, a0, a1) in &corners {
        for s in 0..=segs {
            let a = a0 + (a1 - a0) * s as f32 / segs as f32;
            perimeter.push(Vertex::pos_color_2d(cx + rad * a.cos(), cy + rad * a.sin(), fill_color));
        }
    }

    // Fill using triangle fan
    let center = Vertex::pos_color_2d(px + pw / 2.0, py + ph / 2.0, fill_color);
    // Close the fan by adding the first perimeter point at the end
    perimeter.push(perimeter[0]);
    let fill_tris = fan_verts(center, &perimeter);
    r.draw_triangles(&fill_tris);

    // Border
    r.set_line_smooth(true);
    r.line_width(1.5);
    let border_color = [0.3, 0.4, 0.8, 0.6];
    // Rebuild perimeter with border color (exclude the closing duplicate)
    let border_pts: Vec<Vertex> = corners.iter().flat_map(|&(cx, cy, a0, a1)| {
        (0..=segs).map(move |s| {
            let a = a0 + (a1 - a0) * s as f32 / segs as f32;
            Vertex::pos_color_2d(cx + rad * a.cos(), cy + rad * a.sin(), border_color)
        })
    }).collect();
    r.draw_lines(&line_loop_verts(&border_pts));
    r.set_line_smooth(false);
}

const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;
const WINDOW_TITLE: &str = "GLTron";

/// Signal app state to the web page so touch overlay buttons can adapt.
#[cfg(target_arch = "wasm32")]
fn set_web_app_state(state: &str) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(el) = doc.document_element() {
            let _ = el.set_attribute("data-state", state);
        }
    }
}

/// Open a URL in the user's default browser.
fn open_url(url: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd").args(["/C", "start", url]).spawn();
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            let _ = w.open_with_url_and_target(url, "_blank");
        }
    }
}

struct App {
    // GL state (initialized in resumed)
    window: Option<Window>,
    #[cfg(not(target_arch = "wasm32"))]
    gl_surface: Option<Surface<WindowSurface>>,
    #[cfg(not(target_arch = "wasm32"))]
    gl_context: Option<PossiblyCurrentContext>,
    #[cfg(not(target_arch = "wasm32"))]
    gl_display: Option<Display>,
    r: Option<Renderer>,

    // Resources (loaded after GL init)
    textures: Option<Vec<u32>>,
    models: Option<assets::Models>,
    audio: Option<assets::Audio>,
    music: Option<assets::Music>,
    font: Option<assets::BitmapFont>,

    // Game state
    settings: Settings,
    rules: GameRules,
    app_state: AppState,
    menu_state: Option<menu::MenuState>,
    state: Option<GameState>,

    // Timing
    start_time: Instant,
    last_ticks: u32,
    physics_accumulator: u32,
    finished_at: Option<u32>,

    // Input tracking
    ctrl_held: bool,
    mouse_left: bool,
    mouse_right: bool,
    cursor_pos: (f64, f64),

    // Key bindings (rebindable)
    key_bindings: input::KeyBindings,

    // Gamepad
    gamepad: Option<gamepad::GamepadManager>,
    prev_gp_states: Vec<gamepad::GamepadState>,
    pending_quit: bool,

    // WASM: defer audio init until first user gesture
    #[cfg(target_arch = "wasm32")]
    audio_needs_init: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            #[cfg(not(target_arch = "wasm32"))]
            gl_surface: None,
            #[cfg(not(target_arch = "wasm32"))]
            gl_context: None,
            #[cfg(not(target_arch = "wasm32"))]
            gl_display: None,
            r: None,
            textures: None,
            models: None,
            audio: None,
            music: None,
            font: None,
            settings: Settings::default(),
            rules: GameRules::default(),
            app_state: AppState::Menu,
            menu_state: None,
            state: None,
            start_time: Instant::now(),
            last_ticks: 0,
            physics_accumulator: 0,
            finished_at: None,
            ctrl_held: false,
            mouse_left: false,
            mouse_right: false,
            cursor_pos: (0.0, 0.0),
            key_bindings: input::load_keybindings().unwrap_or_default(),
            gamepad: Some(gamepad::GamepadManager::new()),
            prev_gp_states: Vec::new(),
            pending_quit: false,
            #[cfg(target_arch = "wasm32")]
            audio_needs_init: true,
        }
    }

    /// Get current ticks (ms since start).
    fn ticks(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }

    /// Get window size.
    fn window_size(&self) -> (u32, u32) {
        let size = self.window.as_ref().unwrap().inner_size();
        (size.width, size.height)
    }

    /// Swap GL buffers.
    fn swap_buffers(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let (Some(surface), Some(ctx)) = (&self.gl_surface, &self.gl_context) {
            surface.swap_buffers(ctx).ok();
        }
        // On WASM, WebGL auto-presents — no explicit swap needed.
    }

    /// On WASM, initialize audio after first user gesture (browser requirement).
    #[cfg(target_arch = "wasm32")]
    fn ensure_audio_initialized(&mut self) {
        if !self.audio_needs_init { return; }
        self.audio_needs_init = false;
        log::info!("Initializing audio after user gesture...");
        self.audio = assets::Audio::new();
        self.music = assets::Music::new();
        // Apply current settings
        if let Some(ref music) = self.music {
            if !self.settings.play_music { music.set_paused(true); }
            music.set_volume(self.settings.music_volume);
        }
        if let Some(ref audio) = self.audio {
            let vol = if self.settings.play_effects { self.settings.fx_volume } else { 0.0 };
            audio.set_fx_volume(vol);
        }
    }

    /// Set cursor grab mode for game vs menu.
    fn set_cursor_grabbed(&self, grabbed: bool) {
        if let Some(ref window) = self.window {
            if grabbed {
                // Try Locked first (hides + grabs), fall back to Confined
                if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                    window.set_cursor_grab(CursorGrabMode::Confined).ok();
                }
                window.set_cursor_visible(false);
            } else {
                window.set_cursor_grab(CursorGrabMode::None).ok();
                window.set_cursor_visible(true);
            }
        }
    }

    /// Handle redraw (called from about_to_wait via request_redraw).
    fn do_frame(&mut self) {
        // Compute values before borrowing self fields mutably
        let now = self.start_time.elapsed().as_millis() as u32;
        let win_size = self.window.as_ref().unwrap().inner_size();
        let (w, h) = (win_size.width, win_size.height);

        // Poll gamepad for menu navigation (before taking mutable borrows for rendering)
        if self.app_state == AppState::Menu {
            let gp_states = self.gamepad.as_mut().map(|gp| gp.poll()).unwrap_or_default();

            // If in key capture mode, check for gamepad button presses first
            let capturing = self.menu_state.as_ref().is_some_and(|m| m.is_capturing());
            if capturing && !gp_states.is_empty() {
                let prev_gp = self.prev_gp_states.first()
                    .cloned().unwrap_or_default();
                if let Some(btn) = gamepad::newly_pressed_button(
                    gp_states.first().unwrap(), &prev_gp,
                ) {
                    if btn == gamepad::GamepadBtn::East {
                        // B/Circle = cancel capture (like Escape)
                        self.menu_state.as_mut().unwrap().cancel_capture();
                    } else {
                        menu::menu_gamepad_capture(
                            self.menu_state.as_mut().unwrap(),
                            &mut self.key_bindings,
                            btn,
                        );
                    }
                }
            } else {
                // Normal menu navigation via gamepad
                let menu_keys = gamepad::gamepad_menu_keys(&gp_states, &self.prev_gp_states);
                for kc in menu_keys {
                    let menu_state = self.menu_state.as_mut().unwrap();
                    let action = menu::menu_key_input(
                        menu_state, &mut self.rules, &mut self.settings,
                        &mut self.key_bindings, kc,
                    );
                    if self.process_menu_action(action) {
                        self.pending_quit = true;
                    }
                    break;
                }
            }

            self.prev_gp_states = gp_states;
        }

        let r = self.r.as_mut().unwrap();
        let textures = self.textures.as_ref().unwrap();
        let models = self.models.as_ref().unwrap();
        let font = self.font.as_ref().unwrap();

        match self.app_state {
            AppState::Menu => {
                let menu_state = self.menu_state.as_mut().unwrap();
                menu_state.elapsed_ms = now;
                menu::draw_menu(r, menu_state, font, textures, w, h);
                self.swap_buffers();
            }
            AppState::Playing => {
                // Poll gamepad input (before borrowing gs)
                let gp_states = self.gamepad.as_mut().map(|gp| gp.poll()).unwrap_or_default();

                let gs = self.state.as_mut().unwrap();

                // Apply gamepad input
                let gp_action = if !gp_states.is_empty() {
                    gamepad::apply_gamepad_input(gs, &self.key_bindings, &gp_states, &mut self.prev_gp_states)
                } else {
                    gamepad::GamepadAction::None
                };

                // Handle gamepad actions that need App-level state
                match gp_action {
                    gamepad::GamepadAction::Pause => {
                        if let Some(ref audio) = self.audio { audio.stop_engine(); }
                    }
                    gamepad::GamepadAction::Resume => {
                        gs.pause = PauseState::Running;
                        self.physics_accumulator = 0;
                        self.last_ticks = now;
                        if let Some(ref audio) = self.audio { audio.start_engine(); }
                    }
                    gamepad::GamepadAction::QuitToMenu => {
                        if let Some(ref audio) = self.audio { audio.stop_engine(); }
                        self.state = None;
                        self.app_state = AppState::Menu;
                        #[cfg(target_arch = "wasm32")]
                        set_web_app_state("menu");
                        self.menu_state = Some(menu::MenuState::new(&self.rules, &self.settings, &self.key_bindings));
                        self.set_cursor_grabbed(false);
                        self.swap_buffers();
                        return;
                    }
                    gamepad::GamepadAction::NextRound => {
                        game::reset_game(gs);
                        for player in &mut gs.players {
                            player.camera.kind = self.settings.cam_type;
                            player.show_scoreboard = false;
                            let data = player.data.clone();
                            camera::init_camera(&mut player.camera, &data);
                        }
                        gs.pause = PauseState::Suspended;
                        gs.fast_forward = false;
                        self.finished_at = None;
                        self.physics_accumulator = 0;
                    }
                    gamepad::GamepadAction::EndMatch => {
                        if let Some(ref audio) = self.audio { audio.stop_engine(); }
                        self.state = None;
                        self.app_state = AppState::Menu;
                        #[cfg(target_arch = "wasm32")]
                        set_web_app_state("menu");
                        self.menu_state = Some(menu::MenuState::new(&self.rules, &self.settings, &self.key_bindings));
                        self.set_cursor_grabbed(false);
                        self.swap_buffers();
                        return;
                    }
                    gamepad::GamepadAction::None => {}
                }

                // Update time (saturating: last_ticks may be ahead if game just started this frame)
                let frame_dt = now.saturating_sub(self.last_ticks);
                self.last_ticks = now;

                if gs.pause == PauseState::Running {
                    let ff_mult: u32 = if gs.fast_forward { 8 } else { 1 };
                    self.physics_accumulator += frame_dt * ff_mult;

                    // Fixed timestep physics
                    while self.physics_accumulator >= PHYSICS_RATE {
                        gs.time.last_frame = gs.time.current;
                        gs.time.current += PHYSICS_RATE;
                        gs.time.dt = PHYSICS_RATE;

                        ai::run_ai(gs);
                        game::game_idle(gs);

                        let cam_time = gs.time.current;
                        let rec_alpha = gs.recognizer_alpha;
                        let grid_sz = gs.rules.grid_size;
                        let spectate_datas: Vec<Option<PlayerData>> = gs.players.iter().map(|p| {
                            p.spectate_target.and_then(|si| {
                                if si < gs.players.len() && gs.players[si].data.speed > 0.0 {
                                    Some(gs.players[si].data.clone())
                                } else {
                                    None
                                }
                            })
                        }).collect();
                        use crate::types::DEATH_CAMERA_DELAY;
                        for (pi, player) in gs.players.iter_mut().enumerate() {
                            if player.ai.kind == AiKind::Human && player.data.speed <= 0.0 {
                                // Delay before switching to spectator camera
                                let elapsed_since_death = player.death_time
                                    .map(|dt| cam_time.saturating_sub(dt))
                                    .unwrap_or(DEATH_CAMERA_DELAY);
                                if elapsed_since_death < DEATH_CAMERA_DELAY {
                                    // Keep camera at death position (no update)
                                } else if let Some(ref tdata) = spectate_datas[pi] {
                                    camera::update_camera_spectate_player(
                                        &mut player.camera, tdata, PHYSICS_RATE as f32, cam_time,
                                    );
                                } else {
                                    camera::update_camera_spectator(
                                        &mut player.camera, rec_alpha, grid_sz, PHYSICS_RATE as f32,
                                    );
                                }
                            } else {
                                let data = player.data.clone();
                                camera::update_camera(&mut player.camera, &data, PHYSICS_RATE as f32, cam_time);
                            }
                        }

                        self.physics_accumulator -= PHYSICS_RATE;

                        // Play crash sound
                        if gs.players.iter().any(|p| p.data.speed == -1.0) {
                            if let Some(ref audio) = self.audio {
                                audio.play_crash();
                            }
                            for p in &mut gs.players {
                                if p.data.speed == -1.0 {
                                    p.data.speed = -2.0;
                                }
                            }
                        }

                        // Stop engine when game finishes
                        if gs.pause == PauseState::Finished {
                            if let Some(ref audio) = self.audio { audio.stop_engine(); }
                        }
                    }
                }

                // Auto-reset after 5 seconds
                if gs.pause == PauseState::Finished {
                    let match_over = gs.players.iter().any(|p| p.data.wins >= gs.rules.rounds_to_win);
                    if self.finished_at.is_none() {
                        self.finished_at = Some(now);
                        for p in &mut gs.players { p.show_scoreboard = true; }
                    }
                    if !match_over {
                        if let Some(t) = self.finished_at {
                            if now - t >= 5000 {
                                game::reset_game(gs);
                                for player in &mut gs.players {
                                    player.camera.kind = self.settings.cam_type;
                                    player.show_scoreboard = false;
                                    let data = player.data.clone();
                                    camera::init_camera(&mut player.camera, &data);
                                }
                                gs.pause = PauseState::Suspended;
                                gs.fast_forward = false;
                                self.finished_at = None;
                                self.physics_accumulator = 0;
                            }
                        }
                    }
                } else {
                    self.finished_at = None;
                }

                // Continuous mouse zoom
                if self.mouse_left || self.mouse_right {
                    if let Some(hi) = gs.first_human() {
                        let cam = &mut gs.players[hi].camera;
                        camera::camera_mouse_input(cam, 0.0, 0.0, self.mouse_left, self.mouse_right, frame_dt as f32);
                    }
                }

                // Update engine pitch (stop if human player is dead)
                if let Some(ref audio) = self.audio {
                    if let Some(hi) = gs.human_index() {
                        if gs.players[hi].data.speed > 0.0 {
                            audio.set_engine_pitch(gs.players[hi].data.speed_mult);
                        } else {
                            audio.stop_engine();
                        }
                    }
                }

                // Render
                render::draw_game(r, gs, &self.settings, textures, models, font, &self.key_bindings, w, h);
                render::draw_explosions(r, gs, models);

                // Draw pause overlay
                if gs.pause == PauseState::Suspended {
                    render::begin_2d(r, w, h);
                    let wf = w as f32;
                    let hf = h as f32;
                    let font_sz = (hf * 0.044).min(36.0);
                    let char_w = font_sz * 0.72;
                    let pad_x = font_sz * 2.5;
                    let pad_y = font_sz * 0.6;
                    let line_gap = font_sz * 1.5;

                    if gs.pause_modal {
                        let lines = ["game paused", "> quit to menu"];
                        let max_text_w = lines.iter()
                            .map(|s| s.len() as f32 * char_w)
                            .fold(0.0_f32, f32::max);
                        let pill_w = max_text_w + pad_x * 2.0;
                        let pill_h = pad_y * 2.0 + font_sz + line_gap * 2.0;
                        let px = (wf - pill_w) / 2.0;
                        let py = hf - pill_h - hf * 0.01;
                        let radius = pill_h / 2.0;

                        draw_pill(r, px, py, pill_w, pill_h, radius);

                        let title = "game paused";
                        let title_w = title.len() as f32 * char_w;
                        let tx = (wf - title_w) / 2.0;
                        let ty = py + pill_h - pad_y - font_sz;
                        assets::draw_text_shadowed(r, font, tx, ty, font_sz, title, [0.7, 0.75, 0.9, 0.8]);

                        let opts = ["return", "quit to menu"];
                        for (idx, label) in opts.iter().enumerate() {
                            let oy = ty - (idx as f32 + 1.0) * line_gap;
                            let selected = gs.pause_modal_selection == idx as u8;
                            let color = if selected {
                                [0.3, 1.0, 0.5, 1.0]
                            } else {
                                [0.5, 0.5, 0.6, 0.5]
                            };
                            let prefix = if selected { "> " } else { "  " };
                            let text = format!("{}{}", prefix, label);
                            let ow = text.len() as f32 * char_w;
                            let ox = (wf - ow) / 2.0;
                            assets::draw_text_shadowed(r, font, ox, oy, font_sz, &text, color);
                        }
                    } else {
                        // Main pill: title + hint
                        let hint = "press any key to start";
                        let hint_sz = font_sz * 0.85;
                        let hint_char_w = hint_sz * 0.72;
                        let max_text_w = (hint.len() as f32 * hint_char_w)
                            .max("game paused".len() as f32 * char_w);
                        let pill_w = max_text_w + pad_x * 2.0;
                        let pill_h = pad_y * 2.0 + font_sz + line_gap;
                        let px = (wf - pill_w) / 2.0;
                        let py = hf - pill_h - hf * 0.01;
                        let radius = pill_h / 2.0;

                        draw_pill(r, px, py, pill_w, pill_h, radius);

                        let title = "game paused";
                        let title_w = title.len() as f32 * char_w;
                        let tx = (wf - title_w) / 2.0;
                        let ty = py + pill_h - pad_y - font_sz;
                        assets::draw_text_shadowed(r, font, tx, ty, font_sz, title, [1.0, 1.0, 1.0, 1.0]);

                        let hint_w = hint.len() as f32 * hint_char_w;
                        let hx = (wf - hint_w) / 2.0;
                        let hy = ty - line_gap;
                        assets::draw_text_shadowed(r, font, hx, hy, hint_sz, hint, [0.7, 0.8, 1.0, 0.85]);

                        // Per-player control pills positioned in each player's viewport area
                        struct PlayerCtrl { idx: usize, left: String, right: String, boost: String }
                        let mut humans: Vec<PlayerCtrl> = Vec::new();
                        for (i, p) in gs.players.iter().enumerate() {
                            if p.ai.kind == AiKind::Human && i < input::MAX_KEY_PLAYERS {
                                let keys = &self.key_bindings.players[i];
                                humans.push(PlayerCtrl {
                                    idx: i,
                                    left: keys.left.display_name(),
                                    right: keys.right.display_name(),
                                    boost: keys.boost.display_name(),
                                });
                            }
                        }

                        if !humans.is_empty() {
                            let n = humans.len();
                            let boost_on = gs.rules.booster.enabled;
                            let wall_accel_on = gs.rules.wall_accel;

                            let win_w = w;
                            let win_h = h;
                            for (vp_idx, hc) in humans.iter().enumerate() {
                                // Get viewport rect for this player
                                let (vx, vy, vw, vh) = render::viewport_rect(vp_idx, n, win_w, win_h);
                                let vxf = vx as f32;
                                let vyf = vy as f32;
                                let vwf = vw as f32;
                                let vhf = vh as f32;
                                let vcx = vxf + vwf / 2.0; // viewport center x

                                // Scale font to viewport size
                                let ctrl_sz = (vhf * 0.045).min(28.0).max(12.0);
                                let ctrl_cw = ctrl_sz * 0.72;
                                let ctrl_line_h = ctrl_sz * 1.45;
                                let key_col = [1.0, 1.0, 0.4, 0.95];
                                let label_col = [0.75, 0.8, 0.9, 0.8];
                                let dim_col = [0.5, 0.5, 0.6, 0.6];
                                let enabled_col = [0.1, 0.9, 0.3, 0.9];

                                // Compute pill size
                                let lines_count = 5; // header + left + right + boost + wall accel
                                let cp_pad_x = ctrl_sz * 1.5;
                                let cp_pad_y = ctrl_sz * 0.6;

                                let mut max_line_w = 0.0_f32;
                                let header = format!("P{} controls", hc.idx + 1);
                                max_line_w = max_line_w.max(header.len() as f32 * ctrl_cw);
                                let left_line = format!("turn left  [{}]", hc.left);
                                let right_line = format!("turn right [{}]", hc.right);
                                max_line_w = max_line_w.max(left_line.len() as f32 * ctrl_cw);
                                max_line_w = max_line_w.max(right_line.len() as f32 * ctrl_cw);
                                if boost_on {
                                    let bl = format!("boost  [{}]", hc.boost);
                                    max_line_w = max_line_w.max(bl.len() as f32 * ctrl_cw);
                                }

                                let cp_w = max_line_w + cp_pad_x * 2.0;
                                let cp_h = cp_pad_y * 2.0 + lines_count as f32 * ctrl_line_h;
                                let cp_x = (vcx - cp_w / 2.0).max(vxf + 4.0).min(vxf + vwf - cp_w - 4.0);
                                let cp_y = vyf + vhf * 0.05; // near bottom of viewport
                                let cp_radius = ctrl_sz * 0.5;

                                draw_pill(r, cp_x, cp_y, cp_w, cp_h, cp_radius);

                                // Helper to draw "label  [key]" centered in this pill
                                let draw_key_line_in = |r: &mut Renderer, font: &assets::BitmapFont,
                                                         y: f32, label: &str, key: &str| {
                                    let pre = format!("{}  [", label);
                                    let post = "]";
                                    let total_w = (pre.len() + key.len() + post.len()) as f32 * ctrl_cw;
                                    let lx = vcx - total_w / 2.0;
                                    assets::draw_text_shadowed(r, font, lx, y, ctrl_sz, &pre, label_col);
                                    let kx = lx + pre.len() as f32 * ctrl_cw;
                                    assets::draw_text_shadowed(r, font, kx, y, ctrl_sz, key, key_col);
                                    let bx = kx + key.len() as f32 * ctrl_cw;
                                    assets::draw_text_shadowed(r, font, bx, y, ctrl_sz, post, label_col);
                                };

                                let mut cur_y = cp_y + cp_h - cp_pad_y - ctrl_sz;

                                // Player header in player color
                                let pc = &MODEL_DIFFUSE[hc.idx];
                                let hw = header.len() as f32 * ctrl_cw;
                                assets::draw_text_shadowed(r, font,
                                    vcx - hw / 2.0, cur_y, ctrl_sz,
                                    &header, [pc[0], pc[1], pc[2], 1.0]);
                                cur_y -= ctrl_line_h;

                                draw_key_line_in(r, font, cur_y, "turn left", &hc.left);
                                cur_y -= ctrl_line_h;
                                draw_key_line_in(r, font, cur_y, "turn right", &hc.right);
                                cur_y -= ctrl_line_h;

                                if boost_on {
                                    draw_key_line_in(r, font, cur_y, "boost", &hc.boost);
                                } else {
                                    let text = "boost disabled";
                                    let tw = text.len() as f32 * ctrl_cw;
                                    assets::draw_text_shadowed(r, font, vcx - tw / 2.0, cur_y, ctrl_sz, text, dim_col);
                                }
                                cur_y -= ctrl_line_h;

                                if wall_accel_on {
                                    let text = "wall accel: on";
                                    let tw = text.len() as f32 * ctrl_cw;
                                    assets::draw_text_shadowed(r, font, vcx - tw / 2.0, cur_y, ctrl_sz, text, enabled_col);
                                } else {
                                    let text = "wall accel: off";
                                    let tw = text.len() as f32 * ctrl_cw;
                                    assets::draw_text_shadowed(r, font, vcx - tw / 2.0, cur_y, ctrl_sz, text, dim_col);
                                }
                            }
                        }
                    }
                    render::end_2d(r);
                }

                // Draw finished hint
                if gs.pause == PauseState::Finished {
                    render::begin_2d(r, w, h);
                    let match_over = gs.players.iter().any(|p| p.data.wins >= gs.rules.rounds_to_win);
                    let hint = if match_over { "press [boost] to end match" } else { "press any key to continue" };
                    let hint_size = h as f32 * 0.03;
                    let hint_w = hint.len() as f32 * hint_size * 0.72;
                    let hx = (w as f32 - hint_w) / 2.0;
                    let hy = h as f32 * 0.08;
                    assets::draw_text_shadowed(r, font, hx, hy, hint_size, hint, [0.6, 0.6, 0.8, 0.7]);
                    render::end_2d(r);
                }

                self.swap_buffers();
            }
        }
    }

    /// Process a menu action (shared by keyboard and gamepad paths).
    /// Returns true if the action was Quit (caller must handle event_loop.exit()).
    fn process_menu_action(&mut self, action: menu::MenuAction) -> bool {
        match action {
            menu::MenuAction::StartGame => {
                let human_count = self.rules.ai_players.iter().filter(|k| **k == AiKind::Human).count();
                self.settings.viewport_mode = ViewportMode::for_humans(human_count);
                let mut gs = game::new_game(self.rules.clone(), &self.settings);
                gs.pause = PauseState::Suspended;
                for player in &mut gs.players {
                    player.camera.kind = self.settings.cam_type;
                    let data = player.data.clone();
                    camera::init_camera(&mut player.camera, &data);
                }
                self.state = Some(gs);
                self.app_state = AppState::Playing;
                #[cfg(target_arch = "wasm32")]
                set_web_app_state("playing");
                self.physics_accumulator = 0;
                self.finished_at = None;
                self.last_ticks = self.ticks();
                self.set_cursor_grabbed(true);
            }
            menu::MenuAction::Resize(rw, rh) => {
                if let Some(ref window) = self.window {
                    let _ = window.request_inner_size(PhysicalSize::new(rw, rh));
                }
                self.r.as_mut().unwrap().viewport(0, 0, rw as i32, rh as i32);
            }
            menu::MenuAction::SetWindowed(windowed) => {
                if let Some(ref window) = self.window {
                    if windowed {
                        window.set_fullscreen(None);
                    } else {
                        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                    let size = window.inner_size();
                    self.r.as_mut().unwrap().viewport(0, 0, size.width as i32, size.height as i32);
                }
            }
            menu::MenuAction::AudioChanged => {
                if let Some(ref music) = self.music {
                    music.set_paused(!self.settings.play_music);
                    music.set_volume(self.settings.music_volume);
                }
                if let Some(ref audio) = self.audio {
                    let vol = if self.settings.play_effects { self.settings.fx_volume } else { 0.0 };
                    audio.set_fx_volume(vol);
                }
            }
            menu::MenuAction::Credits => {
                if let Some(ref mut ms) = self.menu_state {
                    ms.open_credits();
                }
            }
            menu::MenuAction::Quit => return true,
            menu::MenuAction::None => {}
        }
        false
    }

    /// Handle key down in menu mode.
    fn handle_menu_key(&mut self, kc: KeyCode, event_loop: &ActiveEventLoop) {
        // Screenshot (desktop only)
        #[cfg(not(target_arch = "wasm32"))]
        if kc == KeyCode::PrintScreen || (kc == KeyCode::KeyP && self.ctrl_held) {
            let (sw, sh) = self.window_size();
            save_screenshot(self.r.as_mut().unwrap(), sw, sh);
            return;
        }

        let menu_state = self.menu_state.as_mut().unwrap();
        let action = menu::menu_key_input(menu_state, &mut self.rules, &mut self.settings, &mut self.key_bindings, kc);
        if self.process_menu_action(action) {
            #[cfg(not(target_arch = "wasm32"))]
            event_loop.exit();
        }
    }

    /// Handle key down in playing mode.
    fn handle_game_key(&mut self, kc: KeyCode, repeat: bool, event_loop: &ActiveEventLoop) {
        // Screenshot (desktop only)
        #[cfg(not(target_arch = "wasm32"))]
        if kc == KeyCode::PrintScreen || (kc == KeyCode::KeyP && self.ctrl_held) {
            let (sw, sh) = self.window_size();
            save_screenshot(self.r.as_mut().unwrap(), sw, sh);
            return;
        }

        let gs = self.state.as_mut().unwrap();

        // Escape key handling
        if kc == KeyCode::Escape {
            if gs.pause == PauseState::Running {
                gs.pause = PauseState::Suspended;
                gs.pause_modal = true;
                gs.pause_modal_selection = 0;
                if let Some(ref audio) = self.audio { audio.stop_engine(); }
                #[cfg(target_arch = "wasm32")]
                set_web_app_state("paused");
            } else if gs.pause == PauseState::Suspended {
                if gs.pause_modal {
                    gs.pause_modal = false;
                    #[cfg(target_arch = "wasm32")]
                    set_web_app_state("playing");
                } else {
                    gs.pause_modal = true;
                    gs.pause_modal_selection = 0;
                    #[cfg(target_arch = "wasm32")]
                    set_web_app_state("paused");
                }
            }
            return;
        }

        // Pause modal input
        if gs.pause == PauseState::Suspended && gs.pause_modal && !repeat {
            match kc {
                KeyCode::ArrowUp | KeyCode::ArrowDown => {
                    gs.pause_modal_selection = 1 - gs.pause_modal_selection;
                }
                KeyCode::Enter | KeyCode::Space => {
                    if gs.pause_modal_selection == 0 {
                        gs.pause_modal = false;
                        #[cfg(target_arch = "wasm32")]
                        set_web_app_state("playing");
                    } else {
                        // Quit to menu
                        if let Some(ref audio) = self.audio { audio.stop_engine(); }
                        self.state = None;
                        self.app_state = AppState::Menu;
                        #[cfg(target_arch = "wasm32")]
                        set_web_app_state("menu");
                        self.menu_state = Some(menu::MenuState::new(&self.rules, &self.settings, &self.key_bindings));
                        self.set_cursor_grabbed(false);
                    }
                }
                _ => {
                    if input::is_passive_key(gs, &self.key_bindings, kc) {
                        input::handle_key_down(gs, &mut self.settings, &self.key_bindings, kc);
                    }
                }
            }
            return;
        }

        // Suspended without modal: any fresh key resumes
        if gs.pause == PauseState::Suspended && !repeat {
            if input::is_passive_key(gs, &self.key_bindings, kc) {
                input::handle_key_down(gs, &mut self.settings, &self.key_bindings, kc);
                return;
            }
            gs.pause = PauseState::Running;
            self.physics_accumulator = 0;
            self.last_ticks = self.ticks();
            if let Some(ref audio) = self.audio { audio.start_engine(); }
            return;
        }

        // Fresh key during finished state
        if gs.pause == PauseState::Finished && !repeat {
            let match_over = gs.players.iter().any(|p| p.data.wins >= gs.rules.rounds_to_win);
            if match_over {
                if input::is_boost_key(gs, &self.key_bindings, kc) {
                    if let Some(ref audio) = self.audio { audio.stop_engine(); }
                    self.state = None;
                    self.app_state = AppState::Menu;
                    self.menu_state = Some(menu::MenuState::new(&self.rules, &self.settings, &self.key_bindings));
                    self.set_cursor_grabbed(false);
                }
                return;
            }
            // Normal round end: any key → next round
            game::reset_game(gs);
            for player in &mut gs.players {
                player.camera.kind = self.settings.cam_type;
                player.show_scoreboard = false;
                let data = player.data.clone();
                camera::init_camera(&mut player.camera, &data);
            }
            gs.pause = PauseState::Suspended;
            gs.fast_forward = false;
            self.finished_at = None;
            self.physics_accumulator = 0;
            return;
        }

        // Normal gameplay input
        input::handle_key_down(gs, &mut self.settings, &self.key_bindings, kc);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already initialized
        }

        // Create window
        let window_attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

        // Platform-specific GL context creation
        #[cfg(not(target_arch = "wasm32"))]
        let (window, mut r) = {
            let template = ConfigTemplateBuilder::new()
                .with_depth_size(24)
                .with_stencil_size(8);

            let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attrs));
            let (window, gl_config) = display_builder.build(event_loop, template, |configs| {
                configs.reduce(|accum, config| {
                    if config.stencil_size() > accum.stencil_size() { config } else { accum }
                }).unwrap()
            }).expect("Failed to create GL display");

            let window = window.expect("Failed to create window");

            // Set window icon from favicon
            if let Ok(icon_data) = image::open("assets/icons/favicon.ico") {
                let icon_rgba = icon_data.to_rgba8();
                let (w, h) = (icon_rgba.width(), icon_rgba.height());
                if let Ok(icon) = winit::window::Icon::from_rgba(icon_rgba.into_raw(), w, h) {
                    window.set_window_icon(Some(icon));
                }
            }

            let raw_window_handle = window.window_handle().unwrap().as_raw();
            let gl_display = gl_config.display();

            let context_attrs = ContextAttributesBuilder::new()
                .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
                .build(Some(raw_window_handle));
            let gl_context = unsafe {
                gl_display.create_context(&gl_config, &context_attrs)
                    .expect("Failed to create GL context")
            };

            let size = window.inner_size();
            let (width, height) = (
                NonZeroU32::new(size.width.max(1)).unwrap(),
                NonZeroU32::new(size.height.max(1)).unwrap(),
            );
            let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
                raw_window_handle, width, height,
            );
            let gl_surface = unsafe {
                gl_display.create_window_surface(&gl_config, &surface_attrs)
                    .expect("Failed to create window surface")
            };

            let gl_context = gl_context.make_current(&gl_surface).expect("Failed to make GL context current");
            gl_surface.set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())).ok();

            let glow_ctx = unsafe {
                glow::Context::from_loader_function(|s| {
                    gl_display.get_proc_address(std::ffi::CString::new(s).unwrap().as_c_str()) as *const _
                })
            };

            self.gl_surface = Some(gl_surface);
            self.gl_context = Some(gl_context);
            self.gl_display = Some(gl_display);

            (window, Renderer::new(glow_ctx))
        };

        #[cfg(target_arch = "wasm32")]
        let (window, mut r, wasm_canvas_size) = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;

            let window = event_loop.create_window(window_attrs).expect("Failed to create window");
            let canvas = window.canvas().expect("Failed to get canvas");

            // Size the canvas pixel buffer to fill the browser viewport
            let web_window = web_sys::window().expect("No window");
            let dpr = web_window.device_pixel_ratio();
            let cw = web_window.inner_width().unwrap().as_f64().unwrap();
            let ch = web_window.inner_height().unwrap().as_f64().unwrap();
            let pw = (cw * dpr) as u32;
            let ph = (ch * dpr) as u32;
            canvas.set_width(pw);
            canvas.set_height(ph);
            log::info!("Canvas: CSS {}x{}, pixel {}x{}, dpr {}", cw as u32, ch as u32, pw, ph, dpr);

            // Append canvas to document body
            let document = web_window.document().expect("No document");
            let body = document.body().expect("No body");
            body.append_child(&canvas).expect("Failed to append canvas");

            // Create WebGL2 context with stencil+depth
            let attrs = web_sys::WebGlContextAttributes::new();
            attrs.set_stencil(true);
            attrs.set_depth(true);
            attrs.set_antialias(true);
            let webgl2 = canvas
                .get_context_with_context_options("webgl2", &attrs)
                .expect("get_context failed")
                .expect("No WebGL2 support")
                .dyn_into::<web_sys::WebGl2RenderingContext>()
                .expect("Not a WebGL2 context");
            let glow_ctx = glow::Context::from_webgl2_context(webgl2);

            (window, Renderer::new(glow_ctx), (pw, ph))
        };

        // Print GL info
        let version = r.get_string(renderer::VERSION);
        let gl_renderer = r.get_string(renderer::GL_RENDERER);
        log::info!("GL Version: {version}");
        log::info!("GL Renderer: {gl_renderer}");

        // Load resources
        let textures = assets::load_all_textures(&mut r);
        let models = assets::load_all_models(&mut r);
        // On WASM, defer audio init until first user gesture (browser policy)
        #[cfg(not(target_arch = "wasm32"))]
        let audio = assets::Audio::new();
        #[cfg(not(target_arch = "wasm32"))]
        let music = assets::Music::new();
        #[cfg(target_arch = "wasm32")]
        let audio: Option<assets::Audio> = None;
        #[cfg(target_arch = "wasm32")]
        let music: Option<assets::Music> = None;
        let font = assets::load_bitmap_font(&mut r);

        // Set initial viewport
        let size = window.inner_size();
        let (vp_w, vp_h) = if size.width > 0 && size.height > 0 {
            (size.width, size.height)
        } else {
            // On WASM, inner_size() may return 0x0 at init; fall back to canvas size
            #[cfg(target_arch = "wasm32")]
            { wasm_canvas_size }
            #[cfg(not(target_arch = "wasm32"))]
            { (WINDOW_WIDTH, WINDOW_HEIGHT) }
        };
        log::info!("Initial viewport: {}x{}", vp_w, vp_h);
        r.viewport(0, 0, vp_w as i32, vp_h as i32);

        render::init_gl(&mut r, &self.settings);

        // Apply initial audio settings
        if let Some(ref music) = music {
            if !self.settings.play_music { music.set_paused(true); }
            music.set_volume(self.settings.music_volume);
        }
        if let Some(ref audio) = audio {
            let vol = if self.settings.play_effects { self.settings.fx_volume } else { 0.0 };
            audio.set_fx_volume(vol);
        }

        self.menu_state = Some(menu::MenuState::new(&self.rules, &self.settings, &self.key_bindings));
        #[cfg(target_arch = "wasm32")]
        set_web_app_state("menu");

        self.window = Some(window);
        self.r = Some(r);
        self.textures = Some(textures);
        self.models = Some(models);
        self.audio = audio;
        self.music = music;
        self.font = Some(font);
        self.start_time = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // On WASM, initialize audio on first user gesture
        #[cfg(target_arch = "wasm32")]
        if matches!(event, WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. } | WindowEvent::Touch { .. }) {
            self.ensure_audio_initialized();
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_held = modifiers.state().control_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let kc = match event.physical_key {
                    PhysicalKey::Code(code) => code,
                    _ => return,
                };

                if event.state == ElementState::Pressed {
                    match self.app_state {
                        AppState::Menu => {
                            // Handle text input for naming mode
                            if let Some(ref menu_state) = self.menu_state {
                                if menu_state.is_naming() && !menu_state.is_capturing() {
                                    if let Some(ref text) = event.text {
                                        menu::menu_text_input(self.menu_state.as_mut().unwrap(), text.as_str());
                                    }
                                }
                            }
                            self.handle_menu_key(kc, event_loop);
                        }
                        AppState::Playing => {
                            self.handle_game_key(kc, event.repeat, event_loop);
                        }
                    }
                } else {
                    // Key released
                    if let AppState::Playing = self.app_state {
                        if let Some(ref mut gs) = self.state {
                            input::handle_key_up(gs, &self.key_bindings, kc);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x, position.y);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        self.mouse_left = pressed;
                        // Check for URL clicks in credits screen
                        if pressed && self.app_state == AppState::Menu {
                            if let Some(ref ms) = self.menu_state {
                                if ms.is_credits() {
                                    let (_, wh) = self.window_size();
                                    // Convert cursor pos from window coords (top-down) to GL coords (bottom-up)
                                    let gl_x = self.cursor_pos.0 as f32;
                                    let gl_y = wh as f32 - self.cursor_pos.1 as f32;
                                    if let Some(url) = ms.credits_click(gl_x, gl_y) {
                                        open_url(&url);
                                    }
                                }
                            }
                        }
                    }
                    MouseButton::Right => self.mouse_right = pressed,
                    _ => {}
                }
            }
            WindowEvent::Resized(_size) => {
                // On WASM, resize canvas pixel buffer to match browser viewport
                #[cfg(target_arch = "wasm32")]
                let size = {
                    use winit::platform::web::WindowExtWebSys;
                    if let Some(ref window) = self.window {
                        if let Some(canvas) = window.canvas() {
                            let web_window = web_sys::window().unwrap();
                            let dpr = web_window.device_pixel_ratio();
                            let cw = web_window.inner_width().unwrap().as_f64().unwrap();
                            let ch = web_window.inner_height().unwrap().as_f64().unwrap();
                            let pw = (cw * dpr) as u32;
                            let ph = (ch * dpr) as u32;
                            canvas.set_width(pw);
                            canvas.set_height(ph);
                            PhysicalSize::new(pw, ph)
                        } else { _size }
                    } else { _size }
                };
                #[cfg(not(target_arch = "wasm32"))]
                let size = _size;

                if let Some(ref _r) = self.r {
                    // Resize GL surface (desktop only)
                    #[cfg(not(target_arch = "wasm32"))]
                    if let (Some(surface), Some(ctx)) = (&self.gl_surface, &self.gl_context) {
                        let width = NonZeroU32::new(size.width.max(1)).unwrap();
                        let height = NonZeroU32::new(size.height.max(1)).unwrap();
                        surface.resize(ctx, width, height);
                    }
                }
                if let Some(ref mut r) = self.r {
                    r.viewport(0, 0, size.width as i32, size.height as i32);
                }
            }
            WindowEvent::RedrawRequested => {
                if self.r.is_some() {
                    self.do_frame();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if let AppState::Playing = self.app_state {
                if let Some(ref mut gs) = self.state {
                    input::handle_mouse_motion(gs, dx as i32, dy as i32);
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.pending_quit {
            _event_loop.exit();
        }
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app).expect("Event loop error");
    }

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let app = App::new();
        event_loop.spawn_app(app);
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
    main();
}
