use glam::{Vec2, Vec3};
use crate::renderer::{self, Renderer, Vertex, LightParams, quad_strip_to_triangles, fan_verts};
use crate::types::*;
use crate::physics;
use crate::assets;

/// Model rotation offset: the cycle model faces -Y by default,
/// so we add PI/2 to convert from mathematical angle to model rotation.
const MODEL_ANGLE_OFFSET: f32 = std::f32::consts::FRAC_PI_2;

/// Get interpolated direction angle during turn animation (for model rotation).
fn get_dir_angle(data: &PlayerData, current_time: u32) -> f32 {
    let elapsed = current_time.saturating_sub(data.turn_time);
    if elapsed < TURN_LENGTH && data.last_dir != data.dir {
        let t = elapsed as f32 / TURN_LENGTH as f32;
        let old = data.last_dir.angle();
        let new = data.dir.angle();
        let mut diff = new - old;
        while diff > std::f32::consts::PI { diff -= std::f32::consts::TAU; }
        while diff < -std::f32::consts::PI { diff += std::f32::consts::TAU; }
        old + diff * t + MODEL_ANGLE_OFFSET
    } else {
        data.dir.angle() + MODEL_ANGLE_OFFSET
    }
}

/// Draw a light cycle at its current position.
pub fn draw_cycle(
    r: &mut Renderer,
    data: &PlayerData,
    visual: &PlayerVisual,
    models: &assets::Models,
    current_time: u32,
    lod: bool,
    cam_pos: Vec3,
) {
    if data.speed <= 0.0 {
        return;
    }

    let pos = physics::current_position(data);
    let angle = get_dir_angle(data, current_time);

    // Select LOD based on distance
    let dist = ((pos.x - cam_pos.x).powi(2) + (pos.y - cam_pos.y).powi(2)).sqrt();
    let model = if !lod || dist < 100.0 {
        &models.cycle_high
    } else if dist < 300.0 {
        &models.cycle_med
    } else {
        &models.cycle_low
    };

    r.push_mv();
    r.translate(pos.x, pos.y, CYCLE_HEIGHT);
    r.rotate_deg(angle.to_degrees(), 0.0, 0.0, 1.0);

    // Tilt during turns
    let elapsed = current_time.saturating_sub(data.turn_time);
    if elapsed < TURN_LENGTH && data.last_dir != data.dir {
        let t = elapsed as f32 / TURN_LENGTH as f32;
        let tilt = NEIGUNG * (std::f32::consts::PI * t).sin();
        r.rotate_deg(tilt, 0.0, 1.0, 0.0);
    }

    // Cycle lighting: point light at camera origin (matches original)
    r.set_lighting(true);
    r.enable_light(0, true);
    r.enable_light(1, false);
    r.push_mv();
    r.load_identity_mv();
    r.set_light(0, &LightParams {
        position: [0.0, 0.0, 0.0, 1.0],
        ambient: [0.1, 0.1, 0.1, 1.0],
        diffuse: [1.0, 1.0, 1.0, 1.0],
        specular: [1.0, 1.0, 1.0, 1.0],
    });
    r.pop_mv();

    // GL_NORMALIZE not needed — shader normalizes v_normal
    assets::draw_gl_mesh(r, model, Some(&visual.diffuse));

    r.pop_mv();
}

/// Draw cycle shadow projected onto the floor.
pub fn draw_cycle_shadow(
    r: &mut Renderer,
    data: &PlayerData,
    models: &assets::Models,
    current_time: u32,
) {
    if data.speed <= 0.0 {
        return;
    }

    let pos = physics::current_position(data);
    let angle = get_dir_angle(data, current_time);

    r.push_mv();
    r.mult_matrix(&SHADOW_MATRIX);
    r.translate(pos.x, pos.y, CYCLE_HEIGHT);
    r.rotate_deg(angle.to_degrees(), 0.0, 0.0, 1.0);

    r.set_lighting(false);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    assets::draw_gl_mesh_flat(r, &models.cycle_low, [0.0, 0.0, 0.0, 0.5]);

    r.set_lighting(true);
    r.disable_blend();
    r.pop_mv();
}

// === Explosion: faithful port of original GLTron 0.70 ===
//
// The original explosion has two components:
// 1. Impact effect: red shockwave rings + white radiating spires + glow
//    (drawn in a vertical plane perpendicular to cycle facing)
// 2. Model explosion: each triangle of the cycle mesh translates outward
//    along its face normal, creating a "model opens up" shattering effect.
//    NO physics/particles — just per-triangle translation scaled by exp_radius.

use crate::renderer::Material;

/// 10 hardcoded scatter vectors from original GLTron (model.c)
const EXP_VECTORS: [[f32; 3]; 10] = [
    [ 0.03, -0.06, -0.07],
    [ 0.04,  0.08, -0.03],
    [ 0.10, -0.04, -0.07],
    [ 0.06, -0.09, -0.10],
    [-0.03, -0.05,  0.02],
    [ 0.07,  0.08, -0.00],
    [ 0.01, -0.04,  0.10],
    [-0.01, -0.07,  0.09],
    [ 0.01, -0.01, -0.09],
    [-0.04,  0.04,  0.02],
];

/// 21 spire direction vectors from original GLTron (explosion.c)
/// These fan out in a semicircle from right (+X) over the top (+Y) to left (-X).
const SPIRE_VECTORS: [[f32; 3]; 21] = [
    [ 1.00,  0.20,  0.00],
    [ 0.80,  0.25,  0.00],
    [ 0.90,  0.50,  0.00],
    [ 0.70,  0.50,  0.00],
    [ 0.52,  0.45,  0.00],
    [ 0.65,  0.75,  0.00],
    [ 0.42,  0.68,  0.00],
    [ 0.40,  1.02,  0.00],
    [ 0.20,  0.90,  0.00],
    [ 0.08,  0.65,  0.00],
    [ 0.00,  1.00,  0.00], // vertical spire
    [-0.08,  0.65,  0.00],
    [-0.20,  0.90,  0.00],
    [-0.40,  1.02,  0.00],
    [-0.42,  0.68,  0.00],
    [-0.65,  0.75,  0.00],
    [-0.52,  0.45,  0.00],
    [-0.70,  0.50,  0.00],
    [-0.90,  0.50,  0.00],
    [-0.80,  0.30,  0.00],
    [-1.00,  0.20,  0.00],
];

/// Main explosion entry point.
/// Draws impact effects (shockwaves, spires, glow) and scatters the cycle model.
pub fn draw_explosion(r: &mut Renderer, visual: &mut PlayerVisual, dt: f32, mesh: &GlMesh) {
    let crash_pos = match visual.crash_pos {
        Some(p) => p,
        None => return,
    };

    if visual.exp_radius >= EXP_RADIUS_MAX {
        return; // explosion finished
    }

    // Advance radii
    visual.exp_radius += dt * EXP_RADIUS_DELTA;
    visual.impact_radius += dt * IMPACT_RADIUS_DELTA;

    let player_color = visual.diffuse;
    let crash_angle = visual.crash_angle;

    // Set up at crash position with cycle rotation
    r.push_mv();
    r.translate(crash_pos.x, crash_pos.y, 0.0);
    r.rotate_deg(crash_angle.to_degrees(), 0.0, 0.0, 1.0);

    // 1. Floor-plane effects (visible from any angle)
    r.set_lighting(false);
    if visual.impact_radius < IMPACT_MAX_RADIUS {
        // Floor-plane red expanding rings
        r.push_mv();
        r.translate(0.0, 0.0, 0.1);
        draw_floor_rings(r, visual.impact_radius * SHOCKWAVE_SPEED, &player_color);
        r.pop_mv();

        // Vertical white jagged starburst — two perpendicular planes at 45°/135°
        // offset so neither plane is edge-on to the behind-camera
        r.push_mv();
        r.rotate_deg(90.0, 0.0, 0.0, 1.0);
        r.rotate_deg(90.0, 1.0, 0.0, 0.0);
        draw_star_burst(r, visual.impact_radius);
        r.pop_mv();
        r.push_mv();
        r.rotate_deg(180.0, 0.0, 0.0, 1.0);
        r.rotate_deg(90.0, 1.0, 0.0, 0.0);
        draw_star_burst(r, visual.impact_radius);
        r.pop_mv();
    }

    // 2. Vertical-plane impact effects (original: rings + spires + glow)
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
    draw_impact(r, visual.impact_radius);

    // 3. Model explosion (at cycle height)
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
    r.push_mv();
    r.translate(0.0, 0.0, CYCLE_HEIGHT);
    draw_model_explosion(r, mesh, visual.exp_radius, &player_color);
    r.pop_mv();

    r.disable_blend();
    r.set_lighting(false);
    r.pop_mv();
}

/// Draw expanding red rings on the floor plane (full circles, highly visible from any camera).
fn draw_floor_rings(r: &mut Renderer, mut radius: f32, player_color: &[f32; 4]) {
    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE); // additive for glow

    let color = [player_color[0].max(0.8), player_color[1] * 0.15, player_color[2] * 0.15, 1.0];

    for _wave in 0..NUM_SHOCKWAVES {
        if radius > 0.0 && radius < SHOCKWAVE_MAX_RADIUS {
            let alpha = (1.0 - radius / SHOCKWAVE_MAX_RADIUS).max(0.0);
            let c = [color[0], color[1], color[2], alpha];
            let ring_width = 0.5 + radius * 0.04; // gets thicker as it expands

            let mut pairs = Vec::with_capacity(52 * 2);
            for seg in 0..=50 {
                let angle = std::f32::consts::TAU * seg as f32 / 50.0;
                let inner = radius;
                let outer = radius + ring_width;
                pairs.push(Vertex::pos_color(
                    [outer * angle.cos(), outer * angle.sin(), 0.0], c,
                ));
                pairs.push(Vertex::pos_color(
                    [inner * angle.cos(), inner * angle.sin(), 0.0], c,
                ));
            }
            let tris = quad_strip_to_triangles(&pairs);
            r.draw_triangles(&tris);
        }
        radius -= SHOCKWAVE_SPACING;
    }
}


/// Draw a vertical 2D white jagged starburst with many pointy unequal-length arms.
/// Each arm is a thin spike triangle (center → narrow base → sharp tip).
fn draw_star_burst(r: &mut Renderer, impact_radius: f32) {
    let progress = (impact_radius / IMPACT_MAX_RADIUS).min(1.0);
    if progress < 0.005 {
        return;
    }

    // Fade out in the last 40% of the animation
    let alpha = if progress > 0.6 {
        1.0 - (progress - 0.6) / 0.4
    } else {
        1.0
    };
    if alpha < 0.01 {
        return;
    }

    r.set_texture(None);
    r.set_blend(renderer::ONE, renderer::ONE); // additive for glow

    let center_color = [1.0f32, 1.0, 1.0, alpha];
    let tip_color = [0.7, 0.8, 1.0, 0.0]; // fade to transparent at tips

    // 40 arms with wildly varying lengths for a chaotic starburst
    const NUM_ARMS: usize = 40;
    let base_angle_step = std::f32::consts::TAU / NUM_ARMS as f32;

    let mut verts = Vec::with_capacity(NUM_ARMS * 3);

    for i in 0..NUM_ARMS {
        // Per-arm deterministic randomness
        let h1 = hash_f32(i as u32 * 7919);
        let h2 = hash_f32(i as u32 * 4813);
        let h3 = hash_f32(i as u32 * 6263);

        // Arm length: mix of short, medium, long arms (very unequal)
        // Some arms reach far, others are stubby
        let length_class = h1;
        let arm_length = progress * if length_class < 0.15 {
            38.0 + h2 * 8.0   // ~15% extra-long arms (38-46)
        } else if length_class < 0.45 {
            22.0 + h2 * 10.0  // ~30% long arms (22-32)
        } else if length_class < 0.75 {
            12.0 + h2 * 8.0   // ~30% medium arms (12-20)
        } else {
            5.0 + h2 * 6.0    // ~25% short arms (5-11)
        };

        // Slight angular jitter so arms aren't evenly spaced
        let angle = i as f32 * base_angle_step + (h3 - 0.5) * base_angle_step * 0.4;

        // Very thin spike: half-width at base is small relative to length
        let half_width = 0.15 + h2 * 0.25; // 0.15 to 0.40 units — very pointy

        // Perpendicular direction for the thin base
        let perp_x = -angle.sin();
        let perp_y = angle.cos();

        // Tip of the spike
        let tip_x = arm_length * angle.cos();
        let tip_y = arm_length * angle.sin();

        // Triangle: left-base, right-base, tip
        verts.push(Vertex::pos_color(
            [perp_x * half_width, perp_y * half_width, 0.0],
            center_color,
        ));
        verts.push(Vertex::pos_color(
            [-perp_x * half_width, -perp_y * half_width, 0.0],
            center_color,
        ));
        verts.push(Vertex::pos_color(
            [tip_x, tip_y, 0.0],
            tip_color,
        ));
    }

    r.draw_triangles(&verts);
}

/// Draw impact mark: shockwave rings + spires + glow in a vertical plane.
/// Port of drawImpact() + drawExplosion() from graphics_fx.c / explosion.c.
fn draw_impact(r: &mut Renderer, impact_radius: f32) {
    r.set_lighting(false);
    r.push_mv();

    // Original: glRotatef(90, 90, 0, 1) — effectively 90° around X axis
    // This tips the XY drawing plane into a vertical plane (XZ)
    r.rotate_deg(90.0, 1.0, 0.0, 0.0);
    r.translate(0.0, -0.5, -0.5);

    let shockwave_radius = impact_radius * SHOCKWAVE_SPEED;
    draw_shockwaves(r, shockwave_radius);

    if impact_radius < IMPACT_MAX_RADIUS {
        draw_spires(r, impact_radius);
    }

    r.pop_mv();
    r.set_lighting(true);
}

/// Draw red semi-circular shockwave rings.
/// Port of drawShockwaves() + drawWave() from explosion.c.
fn draw_shockwaves(r: &mut Renderer, mut radius: f32) {
    r.set_texture(None);
    // No explicit blend set here — inherits from caller (SRC_ALPHA, ONE_MINUS_SRC_ALPHA)

    let color = [1.0f32, 0.0, 0.0, 1.0]; // bright red

    for _wave in 0..NUM_SHOCKWAVES {
        if radius > 0.0 && radius < SHOCKWAVE_MAX_RADIUS {
            draw_wave(r, radius, &color);
        }
        radius -= SHOCKWAVE_SPACING;
    }
}

/// Draw a single semi-circular shockwave band.
fn draw_wave(r: &mut Renderer, mut radius: f32, color: &[f32; 4]) {
    let delta_radius = SHOCKWAVE_WIDTH / SHOCKWAVE_SEGMENTS as f32;
    let delta_angle = std::f32::consts::PI / SHOCKWAVE_SEGMENTS as f32;
    let start_angle: f32 = 3.0 * std::f32::consts::FRAC_PI_2; // 270°

    // Build thick band from SHOCKWAVE_SEGMENTS concentric strips
    for _i in 0..SHOCKWAVE_SEGMENTS {
        let mut angle = start_angle;
        let mut pairs = Vec::with_capacity((SHOCKWAVE_SEGMENTS + 1) * 2);
        for _j in 0..=SHOCKWAVE_SEGMENTS {
            let outer = radius + delta_radius;
            pairs.push(Vertex::pos_color(
                [outer * angle.sin(), outer * angle.cos(), 0.0],
                *color,
            ));
            pairs.push(Vertex::pos_color(
                [radius * angle.sin(), radius * angle.cos(), 0.0],
                *color,
            ));
            angle += delta_angle;
        }
        let tris = quad_strip_to_triangles(&pairs);
        r.draw_triangles(&tris);
        radius += delta_radius;
    }
}

/// Draw 21 white radiating spire triangles.
/// Port of drawSpires() from explosion.c.
fn draw_spires(r: &mut Renderer, radius: f32) {
    r.set_texture(None);
    // Original: glBlendFunc(GL_ONE, GL_ONE) — pure additive
    r.set_blend(renderer::ONE, renderer::ONE);

    let white = [1.0f32, 1.0, 1.0, 1.0];
    let z_unit = Vec3::new(0.0, 0.0, 1.0);

    let mut verts = Vec::with_capacity(NUM_SPIRES * 3);

    for i in 0..NUM_SPIRES {
        let dir = Vec3::new(SPIRE_VECTORS[i][0], SPIRE_VECTORS[i][1], SPIRE_VECTORS[i][2]);

        // Cross products to get perpendicular vectors for ribbon width
        let right = dir.cross(z_unit).normalize_or(Vec3::X) * SPIRE_WIDTH;
        let left = z_unit.cross(dir).normalize_or(Vec3::NEG_X) * SPIRE_WIDTH;

        // Triangle: base-right, tip, base-left
        verts.push(Vertex::pos_color([right.x, right.y, right.z], white));
        verts.push(Vertex::pos_color(
            [radius * dir.x, radius * dir.y, 0.0],
            white,
        ));
        verts.push(Vertex::pos_color([left.x, left.y, left.z], white));
    }

    r.draw_triangles(&verts);
}

/// Draw impact glow (fading white disc, replaces original textured quad).
fn draw_impact_glow(r: &mut Renderer, glow_radius: f32) {
    let opacity = (1.2 - glow_radius / IMPACT_MAX_RADIUS).max(0.0);
    if opacity < 0.01 {
        return;
    }

    r.set_texture(None);
    r.set_depth_write(false);

    // Draw as a circular gradient (center bright, edge transparent)
    let center_color = [1.0f32, 1.0, 1.0, opacity];
    let edge_color = [1.0f32, 1.0, 1.0, 0.0];

    const GLOW_SEGMENTS: usize = 24;
    let center = Vertex::pos_color([0.0, 0.0, 0.0], center_color);
    let mut perimeter = Vec::with_capacity(GLOW_SEGMENTS + 1);
    for i in 0..=GLOW_SEGMENTS {
        let angle = std::f32::consts::TAU * i as f32 / GLOW_SEGMENTS as f32;
        perimeter.push(Vertex::pos_color(
            [glow_radius * angle.cos(), glow_radius * angle.sin(), 0.0],
            edge_color,
        ));
    }
    let tris = fan_verts(center, &perimeter);
    r.draw_triangles(&tris);

    r.set_depth_write(true);
}

/// Simple deterministic hash for per-triangle randomness.
fn hash_f32(seed: u32) -> f32 {
    let mut h = seed;
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Draw the cycle model with each triangle scattered outward, subdivided into fragments.
///
/// Each original mesh triangle is split into 1, 2, or 4 sub-triangles for variety.
/// Each sub-fragment gets its own scatter offset so pieces fly apart independently.
fn draw_model_explosion(r: &mut Renderer, mesh: &GlMesh, exp_radius: f32, player_color: &[f32; 4]) {
    let positions = &mesh.cpu_positions;
    let normals = &mesh.cpu_normals;
    let indices = &mesh.cpu_indices;
    let groups = &mesh.material_groups;

    r.set_lighting(true);
    r.set_color_material(false);
    r.set_texture(None);

    // Cycle lighting: point light at camera origin (same as normal cycle draw)
    r.enable_light(0, true);
    r.enable_light(1, false);
    r.push_mv();
    r.load_identity_mv();
    r.set_light(0, &LightParams {
        position: [0.0, 0.0, 0.0, 1.0],
        ambient: [0.1, 0.1, 0.1, 1.0],
        diffuse: [1.0, 1.0, 1.0, 1.0],
        specular: [1.0, 1.0, 1.0, 1.0],
    });
    r.pop_mv();

    // Process each material group separately (for proper material uniforms)
    for group in groups {
        let amb = [
            group.ambient[0] * player_color[0],
            group.ambient[1] * player_color[1],
            group.ambient[2] * player_color[2],
            1.0,
        ];
        let diff = [
            group.diffuse[0] * player_color[0],
            group.diffuse[1] * player_color[1],
            group.diffuse[2] * player_color[2],
            1.0,
        ];
        let spec = [group.specular[0], group.specular[1], group.specular[2], 1.0];

        r.set_material(&Material {
            ambient: amb,
            diffuse: diff,
            specular: spec,
            shininess: group.shininess.min(128.0),
        });

        let start = group.start_index as usize;
        let count = group.num_indices as usize;
        let num_tris = count / 3;

        // Estimate ~3x vertices due to subdivision
        let mut verts = Vec::with_capacity(count * 3);

        for j in 0..num_tris {
            let base = start + j * 3;

            // Read the 3 vertices of this face
            let read_vert = |idx: usize| -> ([f32; 3], [f32; 3]) {
                let p = [positions[idx * 3], positions[idx * 3 + 1], positions[idx * 3 + 2]];
                let n = [normals[idx * 3], normals[idx * 3 + 1], normals[idx * 3 + 2]];
                (p, n)
            };

            let i0 = indices[base] as usize;
            let i1 = indices[base + 1] as usize;
            let i2 = indices[base + 2] as usize;
            let (p0, n0) = read_vert(i0);
            let (p1, n1) = read_vert(i1);
            let (p2, n2) = read_vert(i2);

            // Face normal (from first vertex, used as base scatter direction)
            let face_n = n0;

            // Decide subdivision level based on triangle index hash:
            // ~20% stay whole, ~35% split into 2, ~45% split into 4
            let h = hash_f32(j as u32 * 7919);
            if h < 0.20 {
                // No subdivision — one large fragment
                emit_fragment(&mut verts, &[p0, p1, p2], &[n0, n1, n2],
                    &face_n, j, 0, exp_radius);
            } else if h < 0.55 {
                // Split into 2: cut along midpoint of edge 0-1
                let mid_p = mid3(p0, p1);
                let mid_n = mid3(n0, n1);
                emit_fragment(&mut verts, &[p0, mid_p, p2], &[n0, mid_n, n2],
                    &face_n, j, 0, exp_radius);
                emit_fragment(&mut verts, &[mid_p, p1, p2], &[mid_n, n1, n2],
                    &face_n, j, 1, exp_radius);
            } else {
                // Split into 4: midpoint subdivision
                let m01 = mid3(p0, p1);
                let m12 = mid3(p1, p2);
                let m20 = mid3(p2, p0);
                let nm01 = mid3(n0, n1);
                let nm12 = mid3(n1, n2);
                let nm20 = mid3(n2, n0);
                emit_fragment(&mut verts, &[p0, m01, m20], &[n0, nm01, nm20],
                    &face_n, j, 0, exp_radius);
                emit_fragment(&mut verts, &[m01, p1, m12], &[nm01, n1, nm12],
                    &face_n, j, 1, exp_radius);
                emit_fragment(&mut verts, &[m20, m12, p2], &[nm20, nm12, n2],
                    &face_n, j, 2, exp_radius);
                emit_fragment(&mut verts, &[m01, m12, m20], &[nm01, nm12, nm20],
                    &face_n, j, 3, exp_radius);
            }
        }

        r.draw_triangles(&verts);
    }

    r.set_color_material(true);
    r.set_lighting(false);
}

/// Midpoint of two [f32; 3] arrays.
fn mid3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5]
}

/// Emit one sub-fragment triangle with its own scatter translation.
/// Each sub-fragment gets a unique offset derived from (tri_index, sub_index).
fn emit_fragment(
    verts: &mut Vec<Vertex>,
    positions: &[[f32; 3]; 3],
    normals_arr: &[[f32; 3]; 3],
    face_normal: &[f32; 3],
    tri_idx: usize,
    sub_idx: usize,
    exp_radius: f32,
) {
    // Each sub-fragment gets its own scatter vector:
    // base direction from face normal + EXP_VECTORS, plus a sub-fragment offset
    let ev = &EXP_VECTORS[tri_idx % 10];
    let sub_seed = (tri_idx * 4 + sub_idx) as u32;
    let sub_offset = [
        (hash_f32(sub_seed.wrapping_mul(31337)) - 0.5) * 0.15,
        (hash_f32(sub_seed.wrapping_mul(48271)) - 0.5) * 0.15,
        (hash_f32(sub_seed.wrapping_mul(16381)) - 0.5) * 0.10,
    ];

    let tx = exp_radius * (face_normal[0] + ev[0] + sub_offset[0]);
    let ty = exp_radius * (face_normal[1] + ev[1] + sub_offset[1]);
    let tz = (exp_radius * (face_normal[2] + ev[2] + sub_offset[2])).abs();

    for k in 0..3 {
        verts.push(Vertex {
            pos: [positions[k][0] + tx, positions[k][1] + ty, positions[k][2] + tz],
            normal: normals_arr[k],
            texcoord: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
}

/// Recognizer parametric motion equations.
pub fn recognizer_position(alpha: f32, grid_size: f32) -> (Vec2, Vec2) {
    let half = grid_size / 2.0 * 0.8;
    let x = 0.5 * (0.3245 * alpha + 0.6).sin() - 0.5 * (0.68 * alpha - 0.3).sin();
    let y = 0.8 * (1.0 * alpha).cos() - 0.2 * (0.2 * alpha).sin();
    let dx = 0.3245 * 0.5 * (0.3245 * alpha + 0.6).cos() - 0.68 * 0.5 * (0.68 * alpha - 0.3).cos();
    let dy = -1.0 * 0.8 * (1.0 * alpha).sin() - 0.2 * 0.2 * (0.2 * alpha).cos();

    let pos = Vec2::new(x * half, y * half);
    let vel = Vec2::new(dx * half, dy * half);
    (pos, vel)
}

/// Get recognizer rotation angle from velocity.
fn recognizer_angle(vel: Vec2) -> f32 {
    vel.y.atan2(vel.x).to_degrees() + 90.0
}

/// Draw the recognizer hovering above the grid.
pub fn draw_recognizer(r: &mut Renderer, alpha: f32, grid_size: f32, models: &assets::Models) {
    let (pos, vel) = recognizer_position(alpha, grid_size);
    let angle = recognizer_angle(vel);
    let scale = 0.25;

    r.push_mv();
    r.translate(pos.x, pos.y, RECOGNIZER_HEIGHT);
    r.rotate_deg(angle, 0.0, 0.0, 1.0);
    r.scale(scale, scale, scale);

    // Setup recognizer lighting
    r.enable_light(0, false);
    r.enable_light(1, false);
    r.enable_light(2, true);
    r.set_light(2, &LightParams {
        position: [0.0, 0.0, 1.0, 0.0],
        ambient: [0.0, 0.0, 0.0, 1.0],
        diffuse: [1.0, 1.0, 1.0, 1.0],
        specular: [1.0, 1.0, 1.0, 1.0],
    });

    // Solid model
    r.set_polygon_offset(true, 1.0, 1.0);
    assets::draw_gl_mesh(r, &models.recognizer, None);
    r.set_polygon_offset(false, 0.0, 0.0);

    // Wireframe outline (not supported on WebGL — polygon_mode(LINE) unavailable)
    #[cfg(not(target_arch = "wasm32"))]
    {
        r.polygon_mode(renderer::FRONT_AND_BACK, renderer::LINE);
        r.set_lighting(false);
        r.set_line_smooth(true);
        r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
        r.line_width(1.5);
        assets::draw_gl_mesh_flat(r, &models.recognizer_quad, [0.0, 0.8, 0.0, 1.0]);
        r.set_line_smooth(false);
        r.disable_blend();
        r.polygon_mode(renderer::FRONT_AND_BACK, renderer::FILL);
        r.set_lighting(true);
    }

    r.enable_light(0, true);
    r.enable_light(1, true);
    r.enable_light(2, false);

    r.pop_mv();
}

/// Draw recognizer shadow.
pub fn draw_recognizer_shadow(r: &mut Renderer, alpha: f32, grid_size: f32, models: &assets::Models) {
    let (pos, vel) = recognizer_position(alpha, grid_size);
    let angle = recognizer_angle(vel);
    let scale = 0.25;

    r.push_mv();
    r.mult_matrix(&SHADOW_MATRIX);
    r.translate(pos.x, pos.y, RECOGNIZER_HEIGHT);
    r.rotate_deg(angle, 0.0, 0.0, 1.0);
    r.scale(scale, scale, scale);

    r.set_lighting(false);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    assets::draw_gl_mesh_flat(r, &models.recognizer, [0.0, 0.0, 0.0, 0.3]);

    r.set_lighting(true);
    r.disable_blend();
    r.pop_mv();
}

/// Draw glow/halo around a player (billboard pentagon fan fading to transparent).
pub fn draw_glow(
    r: &mut Renderer,
    data: &PlayerData,
    visual: &PlayerVisual,
    cam_pos: Vec3,
) {
    if data.speed <= 0.0 {
        return;
    }

    let pos = physics::current_position(data);
    let dim = TRAIL_HEIGHT * 4.0;

    // Distance-based alpha: fade in between 30..100 units from camera
    let dx = pos.x - cam_pos.x;
    let dy = pos.y - cam_pos.y;
    let dz = 0.0 - cam_pos.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let alpha = if dist < 30.0 {
        0.0
    } else if dist > 100.0 {
        1.0
    } else {
        (dist - 30.0) / 70.0
    };

    if alpha < 0.01 {
        return;
    }

    r.push_mv();
    r.translate(pos.x, pos.y, 0.0);

    r.set_depth_write(false);
    r.set_depth_test(true);
    r.set_lighting(false);
    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE);

    // Billboard: extract modelview matrix and zero out rotation components
    let mut mat = r.get_modelview_array();
    mat[0] = 1.0; mat[1] = 0.0; mat[2] = 0.0;
    mat[4] = 0.0; mat[5] = 1.0; mat[6] = 0.0;
    mat[8] = 0.0; mat[9] = 0.0; mat[10] = 1.0;
    r.set_modelview(&glam::Mat4::from_cols_array(&mat));

    let pi = std::f32::consts::PI;
    let half_h = TRAIL_HEIGHT / 2.0;

    // Pentagon fan: center at (0, TRAIL_HEIGHT/2, 0), edges fade to transparent
    let center_color = [visual.diffuse[0], visual.diffuse[1], visual.diffuse[2], alpha];
    let edge_color = [0.0, 0.0, 0.0, 0.0];

    let center = Vertex::pos_color([0.0, half_h, 0.0], center_color);
    let fan_angles: [f32; 7] = [-0.2, 1.0, 2.0, 3.0, 4.0, 5.2, -0.2];
    let perimeter: Vec<Vertex> = fan_angles.iter()
        .map(|&t| {
            let angle = t * pi / 5.0;
            Vertex::pos_color([dim * angle.cos(), half_h + dim * angle.sin(), 0.0], edge_color)
        })
        .collect();
    let fan_tris = fan_verts(center, &perimeter);
    r.draw_triangles(&fan_tris);

    // Two extra triangles to fill the bottom gap (matches original)
    let a0 = -0.2 * pi / 5.0;
    let a1 = 5.2 * pi / 5.0;
    let extra_tris = vec![
        Vertex::pos_color([0.0, half_h, 0.0], center_color),
        Vertex::pos_color([0.0, -TRAIL_HEIGHT / 4.0, 0.0], edge_color),
        Vertex::pos_color([dim * a0.cos(), half_h + dim * a0.sin(), 0.0], edge_color),

        Vertex::pos_color([0.0, half_h, 0.0], center_color),
        Vertex::pos_color([dim * a1.cos(), half_h + dim * a1.sin(), 0.0], edge_color),
        Vertex::pos_color([0.0, -TRAIL_HEIGHT / 4.0, 0.0], edge_color),
    ];
    r.draw_triangles(&extra_tris);

    r.set_depth_write(true);
    r.disable_blend();
    r.set_lighting(true);
    r.pop_mv();
}
