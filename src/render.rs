use crate::renderer::{self, Renderer, Vertex, LightParams, quad_verts, quad_verts_colored, quad_strip_to_triangles, fan_verts, line_loop_verts};
use crate::types::*;
use crate::camera;
use crate::world;
use crate::trail;
use crate::effects;
use crate::assets;
use crate::input::{KeyBindings, MAX_KEY_PLAYERS};

/// Setup OpenGL state for the game.
pub fn init_gl(r: &mut Renderer, _settings: &Settings) {
    r.clear_color(0.01, 0.01, 0.08, 1.0);
    r.clear_stencil(0);
    r.set_depth_test(true);
    r.depth_func(renderer::LEQUAL);
    r.set_cull_face(true);
    r.cull_face(renderer::BACK);

    // Lighting
    r.set_lighting(true);
    r.enable_light(0, true);
    r.enable_light(1, true);

    // Light 0: front-right directional
    r.set_light(0, &LightParams {
        position: [1.0, 0.8, 0.0, 0.0],
        ambient: [0.1, 0.1, 0.1, 1.0],
        diffuse: [1.0, 1.0, 1.0, 1.0],
        specular: [1.0, 1.0, 1.0, 1.0],
    });

    // Light 1: back-left fill
    r.set_light(1, &LightParams {
        position: [-1.0, -0.8, 0.0, 0.0],
        ambient: [0.0, 0.0, 0.0, 1.0],
        diffuse: [0.66, 0.66, 0.66, 1.0],
        specular: [0.0, 0.0, 0.0, 1.0],
    });

    r.set_color_material(true);
}

/// Set up perspective projection matrix.
fn setup_perspective(r: &mut Renderer, fov: f32, aspect: f32, near: f32, far: f32) {
    r.set_perspective(fov, aspect, near, far);
}

/// Set up view matrix (gluLookAt equivalent).
fn setup_look_at(r: &mut Renderer, eye: glam::Vec3, center: glam::Vec3, up: glam::Vec3) {
    let f = (center - eye).normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);

    let view = glam::Mat4::from_cols_array(&[
        s.x, u.x, -f.x, 0.0,
        s.y, u.y, -f.y, 0.0,
        s.z, u.z, -f.z, 0.0,
        -s.dot(eye), -u.dot(eye), f.dot(eye), 1.0,
    ]);
    r.set_modelview(&view);
}

/// Draw scene objects (walls, cycles, trails) - used for both normal and reflection passes.
fn draw_scene_objects(
    r: &mut Renderer,
    state: &GameState,
    settings: &Settings,
    textures: &[u32],
    models: &assets::Models,
    player_idx: usize,
    eye: glam::Vec3,
    is_reflection: bool,
) {
    // Draw recognizer
    if settings.show_recognizer && !is_reflection {
        effects::draw_recognizer(r, state.recognizer_alpha, state.rules.grid_size, models);
    }

    // Draw walls
    if settings.show_wall {
        r.set_cull_face(false);
        world::draw_walls(r, state.rules.grid_size, textures);
    }

    // Draw cycles
    r.set_cull_face(true);
    if settings.light_cycles {
        for (i, p) in state.players.iter().enumerate() {
            effects::draw_cycle(
                r, &p.data, &state.visuals[i], models,
                state.time.current, settings.lod, state.players[player_idx].camera.pos,
            );
        }
        // Restore world lights (draw_cycle modifies LIGHT0 position)
        setup_world_lights(r);
    }

    // Draw trails
    let mut trail_batch = trail::TrailBatch::new();
    for (i, p) in state.players.iter().enumerate() {
        let mut trail_col = state.visuals[i].trail_color;
        if settings.alpha_trails {
            trail_col[3] *= 0.65;
        }
        trail::build_trail_geometry(&p.data, &trail_col, &mut trail_batch);
        trail::build_bow_geometry(&p.data, &trail_col, &mut trail_batch);
        let trail_tex = if settings.show_decals && TEX_DECAL < textures.len() {
            textures[TEX_DECAL]
        } else if TEX_TRAIL < textures.len() {
            textures[TEX_TRAIL]
        } else {
            0
        };
        trail::render_trail_batch(r, &trail_batch, trail_tex, settings.alpha_trails);
        trail_batch.clear();

        if !is_reflection {
            trail::draw_trail_lines(r, &p.data, &state.visuals[i].trail_color);
        }
    }
}

/// Render the complete game scene with stencil-based floor reflections.
pub fn draw_game(
    r: &mut Renderer,
    state: &GameState,
    settings: &Settings,
    textures: &[u32],
    models: &assets::Models,
    font: &assets::BitmapFont,
    bindings: &KeyBindings,
    window_w: u32,
    window_h: u32,
) {
    // Collect human player indices for viewports
    let human_indices: Vec<usize> = state.players.iter().enumerate()
        .filter(|(_, p)| p.ai.kind == AiKind::Human)
        .map(|(i, _)| i)
        .collect();
    let vp_count = human_indices.len().max(1);

    r.clear(renderer::COLOR_BUFFER_BIT | renderer::DEPTH_BUFFER_BIT | renderer::STENCIL_BUFFER_BIT);

    for vp_idx in 0..vp_count {
        // Setup viewport
        let (vx, vy, vw, vh) = viewport_rect(vp_idx, vp_count, window_w, window_h);
        r.viewport(vx, vy, vw, vh);
        r.scissor(vx, vy, vw, vh);
        r.set_scissor_test(true);

        let aspect = vw as f32 / vh.max(1) as f32;
        setup_perspective(r, settings.fov, aspect, 0.5, 4000.0);

        let player_idx = if human_indices.is_empty() { 0 } else { human_indices[vp_idx] };
        let player = &state.players[player_idx];
        let (eye, center, up) = camera::get_view_params(&player.camera);
        r.load_identity_mv();
        setup_look_at(r, eye, center, up);

        setup_world_lights(r);

        // === 1. Draw skybox ===
        if settings.show_skybox {
            r.set_cull_face(false);
            world::draw_skybox(r, state.rules.grid_size, textures);
        }

        // === 2. Reflection pass (stencil-based planar reflection on floor) ===
        if settings.show_reflections {
            // 2a. Mark floor area in stencil buffer
            r.set_stencil_test(true);
            r.stencil_func(renderer::ALWAYS, 1, 0xFF);
            r.stencil_op(renderer::KEEP, renderer::KEEP, renderer::REPLACE);
            r.color_mask(false, false, false, false);
            r.set_depth_write(false);
            r.set_cull_face(false);

            world::draw_floor_quad(r, state.rules.grid_size);

            r.color_mask(true, true, true, true);
            r.set_depth_write(true);

            // 2b. Draw reflected scene (only where stencil == 1)
            r.stencil_func(renderer::EQUAL, 1, 0xFF);
            r.stencil_op(renderer::KEEP, renderer::KEEP, renderer::KEEP);

            r.push_mv();
            r.scale(1.0, 1.0, -1.0);

            // Flip face culling since geometry is mirrored
            r.front_face(renderer::CW);

            // Re-setup lights in mirrored space
            setup_world_lights(r);

            // Draw reflected scene objects (dimmed)
            draw_scene_objects(r, state, settings, textures, models, player_idx, eye, true);

            r.front_face(renderer::CCW);
            r.pop_mv();

            r.set_stencil_test(false);

            // 2c. Clear depth buffer (reflected geometry wrote depth values we don't want)
            r.clear(renderer::DEPTH_BUFFER_BIT);

            // Restore normal lighting
            setup_world_lights(r);
        }

        // === 3. Draw semi-transparent floor surface (tints reflections) ===
        r.set_cull_face(false);
        world::draw_floor_surface(r, state.rules.grid_size);

        // === 4. Draw grid lines ===
        world::draw_floor_grid(r, state.rules.grid_size, settings.line_spacing);

        // === 5. Draw normal scene ===
        r.set_depth_test(true);
        draw_scene_objects(r, state, settings, textures, models, player_idx, eye, false);

        // === 6. Draw glows/halos ===
        if settings.show_glow {
            for (i, p) in state.players.iter().enumerate() {
                effects::draw_glow(r, &p.data, &state.visuals[i], eye);
            }
        }

        r.set_scissor_test(false);
    }

    // Draw viewport borders when split-screen
    if vp_count > 1 {
        r.viewport(0, 0, window_w as i32, window_h as i32);
        r.push_projection();
        r.set_ortho(window_w as f32, window_h as f32);
        r.push_mv();
        r.load_identity_mv();
        r.set_depth_test(false);
        r.set_lighting(false);
        r.set_texture(None);
        r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
        r.set_line_smooth(true);
        r.line_width(1.0);

        let border_col = [0.25, 0.3, 0.5, 0.6];
        let mut lines = Vec::new();
        for vp_idx in 0..vp_count {
            let (vx, vy, vw, vh) = viewport_rect(vp_idx, vp_count, window_w, window_h);
            let (x0, y0) = (vx as f32, vy as f32);
            let (x1, y1) = (x0 + vw as f32, y0 + vh as f32);
            let loop_pts = [
                Vertex::pos_color_2d(x0, y0, border_col),
                Vertex::pos_color_2d(x1, y0, border_col),
                Vertex::pos_color_2d(x1, y1, border_col),
                Vertex::pos_color_2d(x0, y1, border_col),
            ];
            lines.extend_from_slice(&line_loop_verts(&loop_pts));
        }
        r.draw_lines(&lines);

        r.set_line_smooth(false);
        r.disable_blend();
        r.set_depth_test(true);
        r.set_lighting(true);
        r.pop_projection();
        r.pop_mv();
    }

    // Draw per-viewport HUD
    for vp_idx in 0..vp_count {
        let player_idx = if human_indices.is_empty() { 0 } else { human_indices[vp_idx] };
        let (vx, vy, vw, vh) = viewport_rect(vp_idx, vp_count, window_w, window_h);
        draw_viewport_hud(r, state, settings, font, bindings, player_idx, vx, vy, vw, vh);
    }

    // Draw global HUD (fast-forward prompt, minimap)
    draw_global_hud(r, state, settings, font, window_w, window_h);
}

/// Draw explosions (needs mutable state).
pub fn draw_explosions(r: &mut Renderer, state: &mut GameState, models: &assets::Models) {
    let dt = state.time.dt as f32;
    for visual in &mut state.visuals {
        effects::draw_explosion(r, visual, dt, &models.cycle_high);
    }
}

/// Setup world lighting.
fn setup_world_lights(r: &mut Renderer) {
    r.set_lighting(true);
    r.enable_light(0, true);
    r.enable_light(1, true);

    r.set_light(0, &LightParams {
        position: [1.0, 0.8, 0.0, 0.0],
        ambient: [0.1, 0.1, 0.1, 1.0],
        diffuse: [1.0, 1.0, 1.0, 1.0],
        specular: [1.0, 1.0, 1.0, 1.0],
    });
    r.set_light(1, &LightParams {
        position: [-1.0, -0.8, 0.0, 0.0],
        ambient: [0.0, 0.0, 0.0, 1.0],
        diffuse: [0.66, 0.66, 0.66, 1.0],
        specular: [0.0, 0.0, 0.0, 1.0],
    });
}

/// Calculate viewport rectangle for split-screen.
fn viewport_rect(idx: usize, count: usize, w: u32, h: u32) -> (i32, i32, i32, i32) {
    if count <= 1 {
        return (0, 0, w as i32, h as i32);
    }

    let gap = 2i32;
    let wi = w as i32;
    let hi = h as i32;
    let target_ar = w as f32 / h as f32;

    let (cx, cy, cw, ch) = match count {
        2 => {
            let cell_w = (wi - gap) / 2;
            match idx {
                0 => (0, 0, cell_w, hi),
                _ => (cell_w + gap, 0, cell_w, hi),
            }
        }
        3 => {
            let cell_w = (wi - gap) / 2;
            let cell_h = (hi - gap) / 2;
            match idx {
                0 => (0, cell_h + gap, cell_w, cell_h),
                1 => (cell_w + gap, cell_h + gap, cell_w, cell_h),
                _ => ((wi - cell_w) / 2, 0, cell_w, cell_h),
            }
        }
        _ => {
            let cols = (count as f32).sqrt().ceil() as usize;
            let rows = (count + cols - 1) / cols;
            let cell_w = (wi - gap * (cols as i32 - 1)) / cols as i32;
            let cell_h = (hi - gap * (rows as i32 - 1)) / rows as i32;
            let row = idx / cols;
            let col = idx % cols;
            let items_in_row = if row == rows - 1 { count - row * cols } else { cols };
            let row_offset = if items_in_row < cols {
                ((cols - items_in_row) as i32 * (cell_w + gap)) / 2
            } else { 0 };
            let cx = row_offset + col as i32 * (cell_w + gap);
            let cy = hi - (row as i32 + 1) * cell_h - row as i32 * gap;
            (cx, cy, cell_w, cell_h)
        }
    };

    let cell_ar = cw as f32 / ch as f32;
    let (vw, vh) = if cell_ar <= target_ar {
        (cw, (cw as f32 / target_ar) as i32)
    } else {
        ((ch as f32 * target_ar) as i32, ch)
    };

    let vx = cx + (cw - vw) / 2;
    let vy = cy + (ch - vh) / 2;

    (vx, vy, vw, vh)
}

/// Draw per-viewport HUD (speed gauge, boost bar, scoreboard) for a specific player.
fn draw_viewport_hud(r: &mut Renderer, state: &GameState, settings: &Settings, font: &assets::BitmapFont,
                     bindings: &KeyBindings, player_idx: usize, vx: i32, vy: i32, vw: i32, vh: i32) {
    let w = vw as u32;
    let h = vh as u32;
    r.viewport(vx, vy, vw, vh);
    r.scissor(vx, vy, vw, vh);
    r.set_scissor_test(true);
    r.push_projection();
    r.set_ortho(w as f32, h as f32);
    r.push_mv();
    r.load_identity_mv();

    r.set_depth_test(false);
    r.set_lighting(false);
    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // === Bottom-right: speed gauge & boost bar ===
    // When dead: show zeroed HUD during death camera delay, then spectated player's stats
    {
        let is_dead = state.players[player_idx].data.speed <= 0.0;
        let past_death_delay = is_dead && state.players[player_idx].death_time
            .map(|dt| state.time.current.saturating_sub(dt) >= DEATH_CAMERA_DELAY)
            .unwrap_or(true);
        let hi = if past_death_delay {
            state.players[player_idx].spectate_target.unwrap_or(player_idx)
        } else {
            player_idx
        };
        let p = &state.players[hi];
        let c = &state.visuals[hi].diffuse;
        let pi = std::f32::consts::PI;

        let outer_r = h as f32 * 0.11;
        let inner_r = outer_r * 0.68;
        let margin_x = outer_r * 0.35;
        let margin_y = outer_r * 1.1;
        let cx = w as f32 - margin_x - outer_r;
        let cy = margin_y + outer_r;

        let alive = p.data.speed > 0.0;
        let speed_val = if alive { state.rules.speed * p.data.speed_mult } else { 0.0 };
        let max_display_speed = state.rules.speed * state.rules.advanced.max_speed_mult;
        let speed_frac = (speed_val / max_display_speed).clamp(0.0, 1.0);

        let arc_start = -pi / 2.0;
        let segments = 64;

        // --- Background ring (dark translucent) ---
        {
            let color = [0.06, 0.07, 0.15, 0.6];
            let mut pairs = Vec::with_capacity((segments + 1) * 2);
            for s in 0..=segments {
                let t = s as f32 / segments as f32;
                let angle = arc_start + t * 2.0 * pi;
                pairs.push(Vertex::pos_color_2d(cx + inner_r * angle.cos(), cy + inner_r * angle.sin(), color));
                pairs.push(Vertex::pos_color_2d(cx + outer_r * angle.cos(), cy + outer_r * angle.sin(), color));
            }
            let tris = quad_strip_to_triangles(&pairs);
            r.draw_triangles(&tris);
        }

        // --- Zone thresholds ---
        let yellow_frac: f32 = 0.35;
        let red_frac: f32 = 0.75;
        let green_col: [f32; 4] = [0.1, 0.85, 0.3, 0.85];
        let yellow_col: [f32; 4] = [0.95, 0.8, 0.1, 0.85];
        let red_col: [f32; 4] = [0.95, 0.15, 0.1, 0.85];

        // --- Dim zone markers ---
        let zone_borders: &[(f32, f32, [f32; 4])] = &[
            (0.0, yellow_frac, [green_col[0], green_col[1], green_col[2], 0.08]),
            (yellow_frac, red_frac, [yellow_col[0], yellow_col[1], yellow_col[2], 0.08]),
            (red_frac, 1.0, [red_col[0], red_col[1], red_col[2], 0.08]),
        ];
        for &(f0, f1, col) in zone_borders {
            let s0 = (f0 * segments as f32) as usize;
            let s1 = (f1 * segments as f32) as usize;
            if s1 > s0 {
                let mut pairs = Vec::with_capacity((s1 - s0 + 1) * 2);
                for s in s0..=s1 {
                    let t = s as f32 / segments as f32;
                    let angle = arc_start + t * 2.0 * pi;
                    pairs.push(Vertex::pos_color_2d(cx + inner_r * angle.cos(), cy + inner_r * angle.sin(), col));
                    pairs.push(Vertex::pos_color_2d(cx + outer_r * angle.cos(), cy + outer_r * angle.sin(), col));
                }
                let tris = quad_strip_to_triangles(&pairs);
                r.draw_triangles(&tris);
            }
        }

        // --- Filled arc with zone colors ---
        let fill_segs = ((speed_frac * segments as f32) as usize).max(1).min(segments);
        if speed_frac > 0.001 {
            let mut pairs = Vec::with_capacity((fill_segs + 1) * 2);
            for s in 0..=fill_segs {
                let t = s as f32 / segments as f32;
                let angle = arc_start + t * 2.0 * pi;
                let col = if t >= red_frac { red_col } else if t >= yellow_frac { yellow_col } else { green_col };
                pairs.push(Vertex::pos_color_2d(cx + inner_r * angle.cos(), cy + inner_r * angle.sin(), col));
                pairs.push(Vertex::pos_color_2d(cx + outer_r * angle.cos(), cy + outer_r * angle.sin(), col));
            }
            let tris = quad_strip_to_triangles(&pairs);
            r.draw_triangles(&tris);
        }

        // --- Glowing outer ring edge ---
        r.set_line_smooth(true);
        r.line_width(1.5);
        {
            let dim_col = [0.3, 0.35, 0.6, 0.3];
            let pts: Vec<Vertex> = (0..segments).map(|s| {
                let t = s as f32 / segments as f32;
                let angle = arc_start + t * 2.0 * pi;
                Vertex::pos_color_2d(cx + outer_r * angle.cos(), cy + outer_r * angle.sin(), dim_col)
            }).collect();
            let lines = line_loop_verts(&pts);
            r.draw_lines(&lines);
        }

        // Bright edge on filled portion (zone-colored)
        if speed_frac > 0.001 {
            r.line_width(2.0);
            let strip: Vec<Vertex> = (0..=fill_segs).map(|s| {
                let t = s as f32 / segments as f32;
                let angle = arc_start + t * 2.0 * pi;
                let col = if t >= red_frac {
                    [1.0, 0.3, 0.2, 0.9]
                } else if t >= yellow_frac {
                    [1.0, 0.9, 0.2, 0.9]
                } else {
                    [0.2, 1.0, 0.5, 0.9]
                };
                Vertex::pos_color_2d(cx + outer_r * angle.cos(), cy + outer_r * angle.sin(), col)
            }).collect();
            r.draw_line_strip(&strip);
        }

        // Inner ring edge (subtle)
        r.line_width(1.0);
        {
            let inner_col = [0.2, 0.25, 0.45, 0.25];
            let pts: Vec<Vertex> = (0..segments).map(|s| {
                let t = s as f32 / segments as f32;
                let angle = arc_start + t * 2.0 * pi;
                Vertex::pos_color_2d(cx + inner_r * angle.cos(), cy + inner_r * angle.sin(), inner_col)
            }).collect();
            let lines = line_loop_verts(&pts);
            r.draw_lines(&lines);
        }

        // --- Zone divider lines ---
        r.line_width(1.5);
        {
            let div_col = [0.6, 0.6, 0.7, 0.4];
            let mut lines = Vec::new();
            for &zone_frac in &[yellow_frac, red_frac] {
                let angle = arc_start + zone_frac * 2.0 * pi;
                lines.push(Vertex::pos_color_2d(cx + (inner_r - 2.0) * angle.cos(), cy + (inner_r - 2.0) * angle.sin(), div_col));
                lines.push(Vertex::pos_color_2d(cx + (outer_r + 2.0) * angle.cos(), cy + (outer_r + 2.0) * angle.sin(), div_col));
            }
            r.draw_lines(&lines);
        }

        // --- Endpoint glow dot ---
        if speed_frac > 0.01 {
            let end_angle = arc_start + speed_frac * 2.0 * pi;
            let mid_r = (inner_r + outer_r) / 2.0;
            let dot_r = (outer_r - inner_r) * 0.35;
            let dot_cx = cx + mid_r * end_angle.cos();
            let dot_cy = cy + mid_r * end_angle.sin();
            let dot_col = [1.0, 1.0, 1.0, 0.9];
            let dot_segs = 12;
            let center = Vertex::pos_color_2d(dot_cx, dot_cy, dot_col);
            let perim: Vec<Vertex> = (0..=dot_segs).map(|s| {
                let a = s as f32 / dot_segs as f32 * 2.0 * pi;
                Vertex::pos_color_2d(dot_cx + dot_r * a.cos(), dot_cy + dot_r * a.sin(), dot_col)
            }).collect();
            let tris = fan_verts(center, &perim);
            r.draw_triangles(&tris);
        }

        r.set_line_smooth(false);
        r.line_width(1.0);

        // --- Speed number in center ---
        let display_speed = speed_val * 10.0;
        let speed_text = format!("{:.0}", display_speed);
        let font_size = inner_r * 0.6;
        let text_w = speed_text.len() as f32 * font_size * 0.72;
        let text_col = if speed_frac >= red_frac {
            [1.0, 0.3, 0.2, 0.95]
        } else if speed_frac >= yellow_frac {
            [1.0, 0.9, 0.2, 0.95]
        } else {
            [1.0, 1.0, 1.0, 0.95]
        };
        assets::draw_text(r, font, cx - text_w / 2.0, cy - font_size * 0.35, font_size, &speed_text, text_col);

        // "SPEED" label
        let label_size = inner_r * 0.25;
        let label_w = 5.0 * label_size * 0.72;
        assets::draw_text(r, font, cx - label_w / 2.0, cy - font_size * 0.35 - label_size * 1.3, label_size, "speed", [0.4, 0.5, 0.7, 0.5]);

        // Wall acceleration indicator (below the speedometer)
        if state.rules.wall_accel {
            let wa_text = "> wall acc <";
            let wa_size = label_size.max(12.0);
            let wa_w = wa_text.len() as f32 * wa_size * 0.72;
            let wa_y = cy - outer_r - wa_size * 1.3;
            let wa_color = if p.data.wall_accel_active {
                [0.1, 0.9, 0.3, 0.85]
            } else {
                [0.5, 0.6, 0.7, 0.5]
            };
            assets::draw_text_shadowed(r, font, cx - wa_w / 2.0, wa_y, wa_size, wa_text, wa_color);
        }

        // === Boost bar ===
        if state.rules.booster.enabled {
        let bar_w = outer_r * 2.2;
        let bar_h = h as f32 * 0.025;
        let bar_x = cx - bar_w / 2.0;
        let bar_y = cy - outer_r - bar_h - outer_r * 0.25;

        let adv = &state.rules.advanced;
        let boost_min = 1.0f32;
        let boost_frac = if adv.boost_max > boost_min {
            ((p.data.booster - boost_min) / (adv.boost_max - boost_min)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Bar background
        {
            let bg_col = [0.06, 0.07, 0.15, 0.6];
            let verts = quad_verts(
                [[bar_x, bar_y, 0.0], [bar_x + bar_w, bar_y, 0.0], [bar_x + bar_w, bar_y + bar_h, 0.0], [bar_x, bar_y + bar_h, 0.0]],
                bg_col,
            );
            r.draw_triangles(&verts);
        }

        // Bar fill (solid)
        let fill_w = bar_w * boost_frac;
        if fill_w > 0.5 {
            let fill_col = [c[0], c[1], c[2], 0.95];
            let verts = quad_verts(
                [[bar_x, bar_y, 0.0], [bar_x + fill_w, bar_y, 0.0], [bar_x + fill_w, bar_y + bar_h, 0.0], [bar_x, bar_y + bar_h, 0.0]],
                fill_col,
            );
            r.draw_triangles(&verts);
        }

        // Bar border
        r.set_line_smooth(true);
        r.line_width(1.0);
        {
            let bdr_col = [0.4, 0.45, 0.7, 0.5];
            let pts = [
                Vertex::pos_color_2d(bar_x, bar_y, bdr_col),
                Vertex::pos_color_2d(bar_x + bar_w, bar_y, bdr_col),
                Vertex::pos_color_2d(bar_x + bar_w, bar_y + bar_h, bdr_col),
                Vertex::pos_color_2d(bar_x, bar_y + bar_h, bdr_col),
            ];
            let lines = line_loop_verts(&pts);
            r.draw_lines(&lines);
        }
        r.set_line_smooth(false);

        // Boost key hint (hidden while actively boosting)
        if !p.data.boost_enabled && player_idx < MAX_KEY_PLAYERS {
            let key_name = bindings.players[player_idx].boost.display_name();
            let hint_pre = "press [";
            let hint_post = "] to boost";
            let hint_size = (inner_r * 0.25).max(12.0);
            let char_w = hint_size * 0.72;
            let total_w = (hint_pre.len() + key_name.len() + hint_post.len()) as f32 * char_w;
            let hint_x = cx - total_w / 2.0;
            let hint_y = bar_y - hint_size * 1.3;
            let dim_col = [0.7, 0.8, 0.9, 0.7];
            let key_col = [1.0, 1.0, 0.4, 0.95];
            assets::draw_text_shadowed(r, font, hint_x, hint_y, hint_size, hint_pre, dim_col);
            let kx = hint_x + hint_pre.len() as f32 * char_w;
            assets::draw_text_shadowed(r, font, kx, hint_y, hint_size, &key_name, key_col);
            let px = kx + key_name.len() as f32 * char_w;
            assets::draw_text_shadowed(r, font, px, hint_y, hint_size, hint_post, dim_col);
        }
        } // end boost bar
    }

    // === Finished overlay title (per viewport) ===
    if state.pause == PauseState::Finished {
        let is_round_winner = state.winner == Some(player_idx);
        let match_winner = state.players.iter().position(|p| p.data.wins >= state.rules.rounds_to_win);
        let match_over = match_winner.is_some();
        let is_match_winner = match_winner == Some(player_idx);

        let title_opt = if match_over {
            if is_match_winner {
                Some(("match victory!", [0.2, 1.0, 0.4, 1.0_f32]))
            } else {
                Some(("match over", [1.0, 0.2, 0.2, 1.0]))
            }
        } else if is_round_winner {
            Some(("round win!", [0.2, 1.0, 0.4, 1.0]))
        } else {
            None
        };

        if let Some((title, title_color)) = title_opt {
            let font_size = h as f32 * 0.07;
            let title_w = title.len() as f32 * font_size * 0.72;
            let tx = (w as f32 - title_w) / 2.0;
            let ty = h as f32 * 0.75;
            assets::draw_text_shadowed(r, font, tx, ty, font_size, title, title_color);
        }
    }

    // === Scoreboard overlay (per player) ===
    if state.players[player_idx].show_scoreboard {
        draw_scoreboard(r, state, font, w, h, player_idx);
    }

    r.set_depth_test(true);
    r.set_lighting(true);
    r.disable_blend();
    r.set_scissor_test(false);

    r.pop_projection();
    r.pop_mv();
}

/// Draw global HUD elements (fast-forward prompt, minimap) on the full window.
fn draw_global_hud(r: &mut Renderer, state: &GameState, settings: &Settings, font: &assets::BitmapFont, w: u32, h: u32) {
    r.viewport(0, 0, w as i32, h as i32);
    r.push_projection();
    r.set_ortho(w as f32, h as f32);
    r.push_mv();
    r.load_identity_mv();

    r.set_depth_test(false);
    r.set_lighting(false);
    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // === Spectator fast-forward prompt ===
    {
        let all_humans_dead = state.players.iter()
            .filter(|p| p.ai.kind == AiKind::Human)
            .all(|p| p.data.speed <= 0.0);
        let any_human = state.players.iter().any(|p| p.ai.kind == AiKind::Human);
        if any_human && all_humans_dead && state.pause == PauseState::Running {
            let hf = h as f32;
            let wf = w as f32;
            let prompt = if state.fast_forward {
                ">> fast forward >>"
            } else {
                "hold [boost] to fast forward"
            };
            let sz = hf * 0.028;
            let pw = prompt.len() as f32 * sz * 0.72;
            let px = (wf - pw) / 2.0;
            let py = hf * 0.15;
            let col = if state.fast_forward {
                [0.4, 1.0, 0.5, 0.9]
            } else {
                [0.6, 0.6, 0.8, 0.6]
            };
            assets::draw_text_shadowed(r, font, px, py, sz, prompt, col);
        }
    }

    // === Minimap ===
    if settings.show_minimap {
        draw_minimap(r, state, w, h);
    }

    r.set_depth_test(true);
    r.set_lighting(true);
    r.disable_blend();

    r.pop_projection();
    r.pop_mv();
}

/// Draw a 2D minimap in the top-right corner.
fn draw_minimap(r: &mut Renderer, state: &GameState, w: u32, h: u32) {
    let half_grid = state.rules.grid_size / 2.0;
    let map_size = h as f32 * 0.25;
    let margin = 10.0;
    let map_x = w as f32 - map_size - margin;
    let map_y = h as f32 - map_size - margin;

    let to_screen = |wx: f32, wy: f32| -> (f32, f32) {
        let nx = (wx + half_grid) / (half_grid * 2.0);
        let ny = (wy + half_grid) / (half_grid * 2.0);
        (map_x + nx * map_size, map_y + ny * map_size)
    };

    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // Background
    {
        let bg_col = [0.02, 0.03, 0.1, 0.65];
        let verts = quad_verts(
            [[map_x, map_y, 0.0], [map_x + map_size, map_y, 0.0], [map_x + map_size, map_y + map_size, 0.0], [map_x, map_y + map_size, 0.0]],
            bg_col,
        );
        r.draw_triangles(&verts);
    }

    // Border
    r.set_line_smooth(true);
    r.line_width(1.0);
    {
        let bdr_col = [0.3, 0.4, 0.6, 0.5];
        let pts = [
            Vertex::pos_color_2d(map_x, map_y, bdr_col),
            Vertex::pos_color_2d(map_x + map_size, map_y, bdr_col),
            Vertex::pos_color_2d(map_x + map_size, map_y + map_size, bdr_col),
            Vertex::pos_color_2d(map_x, map_y + map_size, bdr_col),
        ];
        let lines = line_loop_verts(&pts);
        r.draw_lines(&lines);
    }

    // Draw trails for each player (batched per player)
    for (pi, player) in state.players.iter().enumerate() {
        let c = &state.visuals[pi].trail_color;
        let alive = player.data.speed > 0.0;
        let alpha = if alive { c[3].max(0.8) } else if player.data.trail_height > 0.0 { c[3] } else { 0.0 };
        if alpha < 0.01 { continue; }

        let color = [c[0], c[1], c[2], alpha];
        r.line_width(if alive { 2.0 } else { 1.0 });

        let count = (player.data.trail_offset + 1).min(player.data.trails.len());
        let mut lines = Vec::new();
        for seg in &player.data.trails[..count] {
            let (x0, y0) = to_screen(seg.start.x, seg.start.y);
            let end = seg.end();
            let (x1, y1) = to_screen(end.x, end.y);
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx * dx + dy * dy < 0.1 { continue; }
            lines.push(Vertex::pos_color_2d(x0, y0, color));
            lines.push(Vertex::pos_color_2d(x1, y1, color));
        }
        if !lines.is_empty() {
            r.draw_lines(&lines);
        }
    }

    // Draw player position dots
    for (pi, player) in state.players.iter().enumerate() {
        if player.data.speed <= 0.0 && player.data.trail_height <= 0.0 { continue; }
        let pos = crate::physics::current_position(&player.data);
        let (sx, sy) = to_screen(pos.x, pos.y);
        let c = &state.visuals[pi].diffuse;
        let dot_r = if player.data.speed > 0.0 { 3.0 } else { 2.0 };
        let dot_col = [c[0], c[1], c[2], 1.0];

        let center = Vertex::pos_color_2d(sx, sy, dot_col);
        let perim: Vec<Vertex> = (0..=8).map(|s| {
            let a = s as f32 * std::f32::consts::TAU / 8.0;
            Vertex::pos_color_2d(sx + dot_r * a.cos(), sy + dot_r * a.sin(), dot_col)
        }).collect();
        let tris = fan_verts(center, &perim);
        r.draw_triangles(&tris);
    }

    r.set_line_smooth(false);
    r.disable_blend();
}

/// Draw the full scoreboard overlay.
fn draw_scoreboard(r: &mut Renderer, state: &GameState, font: &assets::BitmapFont, w: u32, h: u32, viewer_idx: usize) {
    let wf = w as f32;
    let hf = h as f32;
    let n = state.players.len();

    let board_w = wf * 0.65;
    let row_h = hf * 0.07;
    let header_h = row_h * 1.3;
    let footer_h = row_h * 0.8;
    let board_h = header_h + row_h * n as f32 + footer_h;
    let bx = (wf - board_w) / 2.0;
    let by = (hf - board_h) / 2.0;
    let pad = board_w * 0.03;

    let col_color_bar = bx + pad;
    let col_player = bx + pad + row_h * 0.6;
    let col_w = bx + board_w * 0.50;
    let col_k = bx + board_w * 0.65;
    let col_d = bx + board_w * 0.80;

    let font_sz = row_h * 0.55;
    let hdr_sz = font_sz * 1.0;

    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // Background
    {
        let bg_col = [0.02, 0.03, 0.08, 0.90];
        let verts = quad_verts(
            [[bx, by, 0.0], [bx + board_w, by, 0.0], [bx + board_w, by + board_h, 0.0], [bx, by + board_h, 0.0]],
            bg_col,
        );
        r.draw_triangles(&verts);
    }

    // Border
    r.set_line_smooth(true);
    r.line_width(2.0);
    {
        let bdr_col = [0.3, 0.4, 0.8, 0.7];
        let pts = [
            Vertex::pos_color_2d(bx, by, bdr_col),
            Vertex::pos_color_2d(bx + board_w, by, bdr_col),
            Vertex::pos_color_2d(bx + board_w, by + board_h, bdr_col),
            Vertex::pos_color_2d(bx, by + board_h, bdr_col),
        ];
        let lines = line_loop_verts(&pts);
        r.draw_lines(&lines);
    }

    // Header separator line
    let sep_y = by + board_h - header_h;
    {
        r.line_width(1.5);
        let sep_col = [0.4, 0.5, 0.8, 0.5];
        let lines = vec![
            Vertex::pos_color_2d(bx + pad, sep_y, sep_col),
            Vertex::pos_color_2d(bx + board_w - pad, sep_y, sep_col),
        ];
        r.draw_lines(&lines);
    }

    // Footer separator line
    let foot_sep_y = by + footer_h;
    {
        let sep_col = [0.3, 0.4, 0.7, 0.3];
        let lines = vec![
            Vertex::pos_color_2d(bx + pad, foot_sep_y, sep_col),
            Vertex::pos_color_2d(bx + board_w - pad, foot_sep_y, sep_col),
        ];
        r.draw_lines(&lines);
    }

    // Player rows
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| state.players[*b].data.wins.cmp(&state.players[*a].data.wins));

    let match_winner = state.players.iter().position(|p| p.data.wins >= state.rules.rounds_to_win);
    let human_idx = Some(viewer_idx);
    use std::sync::OnceLock;
    use web_time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let time_s = START.get_or_init(Instant::now).elapsed().as_secs_f32();

    for (row, &pi) in order.iter().enumerate() {
        let row_top = by + board_h - header_h - row as f32 * row_h;
        let row_bot = row_top - row_h;
        let rx0 = bx + pad * 0.5;
        let rx1 = bx + board_w - pad * 0.5;

        if match_winner == Some(pi) {
            let c = &state.visuals[pi].diffuse;

            // Tinted background
            {
                let bg_col = [c[0] * 0.25, c[1] * 0.25, c[2] * 0.25, 0.4];
                let verts = quad_verts(
                    [[rx0, row_bot, 0.0], [rx1, row_bot, 0.0], [rx1, row_top, 0.0], [rx0, row_top, 0.0]],
                    bg_col,
                );
                r.draw_triangles(&verts);
            }

            // Sparkles traveling along the perimeter
            let rw = rx1 - rx0;
            let rh = row_top - row_bot;
            let perimeter = 2.0 * rw + 2.0 * rh;
            let px_sz = (row_h * 0.09).max(2.0);
            let num_sparkles = 30;

            let perimeter_pos = |frac: f32| -> (f32, f32) {
                let d = frac * perimeter;
                if d < rw {
                    (rx0 + d, row_top)
                } else if d < rw + rh {
                    (rx1, row_top - (d - rw))
                } else if d < 2.0 * rw + rh {
                    (rx1 - (d - rw - rh), row_bot)
                } else {
                    (rx0, row_bot + (d - 2.0 * rw - rh))
                }
            };

            let mut sparkle_tris = Vec::new();
            for i in 0..num_sparkles {
                let fi = i as f32;
                let base_offset = fi / num_sparkles as f32;
                let speed = 0.08 + ((fi * 127.1).sin().abs()) * 0.12;
                let pos = (base_offset + time_s * speed).fract();
                let (sx, sy) = perimeter_pos(pos);

                let shimmer = (time_s * 6.0 + fi * 1.7).sin() * 0.5 + 0.5;
                let alpha = 0.3 + shimmer * 0.7;
                let sz = px_sz * (0.6 + shimmer * 0.6);

                let w_mix = shimmer * 0.5;
                let sr = c[0] + (1.0 - c[0]) * w_mix;
                let sg = c[1] + (1.0 - c[1]) * w_mix;
                let sb = c[2] + (1.0 - c[2]) * w_mix;
                let col = [sr, sg, sb, alpha];

                let q = quad_verts(
                    [[sx - sz * 0.5, sy - sz * 0.5, 0.0], [sx + sz * 0.5, sy - sz * 0.5, 0.0],
                     [sx + sz * 0.5, sy + sz * 0.5, 0.0], [sx - sz * 0.5, sy + sz * 0.5, 0.0]],
                    col,
                );
                sparkle_tris.extend_from_slice(&q);
            }
            r.draw_triangles(&sparkle_tris);
        } else if human_idx == Some(pi) {
            let c = &state.visuals[pi].diffuse;
            {
                let bg_col = [c[0] * 0.25, c[1] * 0.25, c[2] * 0.25, 0.35];
                let verts = quad_verts(
                    [[rx0, row_bot, 0.0], [rx1, row_bot, 0.0], [rx1, row_top, 0.0], [rx0, row_top, 0.0]],
                    bg_col,
                );
                r.draw_triangles(&verts);
            }
            // Subtle border
            r.set_line_smooth(true);
            r.line_width(1.0);
            {
                let bdr_col = [c[0], c[1], c[2], 0.4];
                let pts = [
                    Vertex::pos_color_2d(rx0, row_bot, bdr_col),
                    Vertex::pos_color_2d(rx1, row_bot, bdr_col),
                    Vertex::pos_color_2d(rx1, row_top, bdr_col),
                    Vertex::pos_color_2d(rx0, row_top, bdr_col),
                ];
                let lines = line_loop_verts(&pts);
                r.draw_lines(&lines);
            }
            r.set_line_smooth(false);
        } else if row % 2 == 1 {
            let bg_col = [0.05, 0.06, 0.15, 0.3];
            let verts = quad_verts(
                [[rx0, row_bot, 0.0], [rx1, row_bot, 0.0], [rx1, row_top, 0.0], [rx0, row_top, 0.0]],
                bg_col,
            );
            r.draw_triangles(&verts);
        }

        // Color indicator bar
        let c = &state.visuals[pi].diffuse;
        let alive = state.players[pi].data.speed > 0.0;
        let alpha = if alive { 0.9 } else { 0.35 };
        let bar_h = row_h * 0.6;
        let bar_w = row_h * 0.35;
        let bar_y = row_bot + (row_h - bar_h) / 2.0;
        {
            let bar_col = [c[0], c[1], c[2], alpha];
            let verts = quad_verts(
                [[col_color_bar, bar_y, 0.0], [col_color_bar + bar_w, bar_y, 0.0],
                 [col_color_bar + bar_w, bar_y + bar_h, 0.0], [col_color_bar, bar_y + bar_h, 0.0]],
                bar_col,
            );
            r.draw_triangles(&verts);
        }
    }

    r.set_line_smooth(false);
    r.disable_blend();

    // Header text
    let header_y = by + board_h - header_h * 0.4;
    let hdr_col = [0.7, 0.8, 1.0, 0.9];
    assets::draw_text_shadowed(r, font, col_player, header_y, hdr_sz, "PLAYER", hdr_col);
    assets::draw_text_shadowed(r, font, col_w, header_y, hdr_sz, "W", hdr_col);
    assets::draw_text_shadowed(r, font, col_k, header_y, hdr_sz, "K", hdr_col);
    assets::draw_text_shadowed(r, font, col_d, header_y, hdr_sz, "D", hdr_col);

    // Player rows text
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| state.players[*b].data.wins.cmp(&state.players[*a].data.wins));

    for (row, &pi) in order.iter().enumerate() {
        let p = &state.players[pi];
        let c = &state.visuals[pi].diffuse;
        let alive = p.data.speed > 0.0;
        let alpha = if alive { 1.0 } else { 0.45 };
        let row_top = by + board_h - header_h - row as f32 * row_h;
        let text_y = row_top - row_h * 0.65;

        let kind_str = match p.ai.kind {
            AiKind::Human => "HUMAN",
            AiKind::Computer => "CPU",
            AiKind::None => "---",
        };
        let name = format!("P{} {}", pi + 1, kind_str);
        let col = [c[0], c[1], c[2], alpha];

        assets::draw_text_shadowed(r, font, col_player, text_y, font_sz, &name, col);

        let stat_col = [0.9, 0.9, 0.95, alpha];
        assets::draw_text_shadowed(r, font, col_w, text_y, font_sz, &p.data.wins.to_string(), stat_col);
        assets::draw_text_shadowed(r, font, col_k, text_y, font_sz, &p.data.kills.to_string(), stat_col);
        assets::draw_text_shadowed(r, font, col_d, text_y, font_sz, &p.data.deaths.to_string(), stat_col);
    }

    // Footer: rounds to win
    let footer_y = by + footer_h * 0.15;
    let rtw = format!("first to {} wins", state.rules.rounds_to_win);
    let ftw_sz = font_sz * 0.85;
    let ftw_w = rtw.len() as f32 * ftw_sz * 0.72;
    assets::draw_text_shadowed(r, font, bx + (board_w - ftw_w) / 2.0, footer_y, ftw_sz, &rtw, [0.5, 0.55, 0.75, 0.6]);
}

/// Setup 2D rendering mode (for menus etc).
pub fn begin_2d(r: &mut Renderer, w: u32, h: u32) {
    r.push_projection();
    r.set_ortho(w as f32, h as f32);
    r.push_mv();
    r.load_identity_mv();
    r.set_depth_test(false);
    r.set_lighting(false);
}

pub fn end_2d(r: &mut Renderer) {
    r.set_depth_test(true);
    r.set_lighting(true);
    r.pop_projection();
    r.pop_mv();
}
