use glam::Vec2;
use crate::renderer::{self, Renderer, Vertex};
use crate::types::*;

/// Trail batch: vertices + indices for indexed drawing.
pub struct TrailBatch {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl TrailBatch {
    pub fn new() -> Self {
        Self {
            verts: Vec::with_capacity(MAX_TRAIL * 8),
            indices: Vec::with_capacity(MAX_TRAIL * 12),
        }
    }

    pub fn clear(&mut self) {
        self.verts.clear();
        self.indices.clear();
    }

    pub fn vertex_count(&self) -> u16 {
        self.verts.len() as u16
    }
}

/// Build trail mesh geometry for a player.
pub fn build_trail_geometry(data: &PlayerData, color: &[f32; 4], batch: &mut TrailBatch) {
    batch.clear();

    let trail_h = data.trail_height;
    if trail_h <= 0.0 {
        return;
    }

    let segments = &data.trails[..=data.trail_offset.min(data.trails.len() - 1)];

    for seg in segments.iter() {
        if seg.direction.length_squared() < 1e-6 {
            continue;
        }

        let start = seg.start;
        let end = seg.end();

        let dir_norm = seg.direction.normalize();
        let normal = Vec2::new(-dir_norm.y, dir_norm.x);

        let u_start = 0.0;
        let u_end = seg.length() / DECAL_WIDTH;

        let base = batch.vertex_count();

        // Side 1: start_bottom, start_top, end_bottom, end_top
        batch.verts.push(Vertex::full([start.x, start.y, 0.0], [normal.x, normal.y, 0.0], [u_start, 0.0], *color));
        batch.verts.push(Vertex::full([start.x, start.y, trail_h], [normal.x, normal.y, 0.0], [u_start, 1.0], *color));
        batch.verts.push(Vertex::full([end.x, end.y, 0.0], [normal.x, normal.y, 0.0], [u_end, 0.0], *color));
        batch.verts.push(Vertex::full([end.x, end.y, trail_h], [normal.x, normal.y, 0.0], [u_end, 1.0], *color));

        batch.indices.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);

        // Side 2 (opposite normal)
        let base2 = batch.vertex_count();
        let nn = [-normal.x, -normal.y, 0.0];

        batch.verts.push(Vertex::full([start.x, start.y, 0.0], nn, [u_start, 0.0], *color));
        batch.verts.push(Vertex::full([start.x, start.y, trail_h], nn, [u_start, 1.0], *color));
        batch.verts.push(Vertex::full([end.x, end.y, 0.0], nn, [u_end, 0.0], *color));
        batch.verts.push(Vertex::full([end.x, end.y, trail_h], nn, [u_end, 1.0], *color));

        batch.indices.extend_from_slice(&[base2, base2 + 3, base2 + 1, base2, base2 + 2, base2 + 3]);
    }
}

/// Build bow (curved front) geometry at the end of the trail.
pub fn build_bow_geometry(data: &PlayerData, color: &[f32; 4], batch: &mut TrailBatch) {
    if data.trail_offset >= data.trails.len() || data.trail_height <= 0.0 {
        return;
    }

    let seg = &data.trails[data.trail_offset];
    let seg_len = seg.length();
    if seg_len < 0.1 {
        return;
    }

    let end = seg.end();
    let dir_norm = seg.direction.normalize();
    let normal = Vec2::new(-dir_norm.y, dir_norm.x);
    let bow_len = BOW_LENGTH.min(seg_len);
    let trail_h = data.trail_height;

    let steps = 10;
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;

        let pos0 = end - dir_norm * bow_len * (1.0 - t0);
        let pos1 = end - dir_norm * bow_len * (1.0 - t1);

        let h0 = trail_h * (1.0 - t0 * t0).sqrt().max(0.0);
        let h1 = trail_h * (1.0 - t1 * t1).sqrt().max(0.0);

        let c0 = bow_color(color, t0);
        let c1 = bow_color(color, t1);

        let base = batch.vertex_count();

        // Side 1
        batch.verts.push(Vertex::full([pos0.x, pos0.y, 0.0], [normal.x, normal.y, 0.0], [t0, 0.0], c0));
        batch.verts.push(Vertex::full([pos0.x, pos0.y, h0], [normal.x, normal.y, 0.0], [t0, 1.0], c0));
        batch.verts.push(Vertex::full([pos1.x, pos1.y, 0.0], [normal.x, normal.y, 0.0], [t1, 0.0], c1));
        batch.verts.push(Vertex::full([pos1.x, pos1.y, h1], [normal.x, normal.y, 0.0], [t1, 1.0], c1));

        batch.indices.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    }
}

/// Compute bow color: lerps from trail color (at t=0) to bright white (at t=1).
fn bow_color(color: &[f32; 4], t: f32) -> [f32; 4] {
    let glow = t * t;
    [
        (color[0] + glow * (1.0 - color[0])).min(1.0),
        (color[1] + glow * (1.0 - color[1])).min(1.0),
        (color[2] + glow * (1.0 - color[2])).min(1.0),
        1.0,
    ]
}

/// Render trail batch.
pub fn render_trail_batch(r: &mut Renderer, batch: &TrailBatch, tex_id: u32, alpha_trails: bool) {
    if batch.indices.is_empty() {
        return;
    }

    r.set_lighting(false);

    if alpha_trails {
        r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
        r.set_depth_write(false);
    }

    r.set_cull_face(false);
    r.set_polygon_offset(true, 1.0, 1.0);

    if tex_id != 0 {
        r.set_texture(Some(tex_id));
        if alpha_trails {
            r.set_tex_mode(renderer::TexMode::Modulate);
        } else {
            r.set_tex_mode(renderer::TexMode::Decal);
        }
    }

    r.draw_indexed(&batch.verts, &batch.indices);

    r.set_tex_mode(renderer::TexMode::Modulate);
    r.set_texture(None);
    r.set_polygon_offset(false, 0.0, 0.0);
    r.set_cull_face(true);

    if alpha_trails {
        r.set_depth_write(true);
        r.disable_blend();
    }

    r.set_lighting(true);
}

/// Draw neon trail edge lines.
pub fn draw_trail_lines(r: &mut Renderer, data: &PlayerData, color: &[f32; 4]) {
    if data.trail_height <= 0.0 {
        return;
    }

    let trail_h = data.trail_height;
    let segments = &data.trails[..=data.trail_offset.min(data.trails.len() - 1)];

    let lr = (color[0] * 0.5 + 0.5).min(1.0);
    let lg = (color[1] * 0.5 + 0.5).min(1.0);
    let lb = (color[2] * 0.5 + 0.5).min(1.0);

    r.set_lighting(false);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE);
    r.set_line_smooth(true);
    r.line_width(1.5);

    let mut lines = Vec::new();
    for seg in segments {
        if seg.direction.length_squared() < 1e-6 {
            continue;
        }
        let start = seg.start;
        let end = seg.end();

        // Top edge
        lines.push(Vertex::pos_color([start.x, start.y, trail_h], [lr, lg, lb, 0.6]));
        lines.push(Vertex::pos_color([end.x, end.y, trail_h], [lr, lg, lb, 0.6]));

        // Bottom edge
        lines.push(Vertex::pos_color([start.x, start.y, 0.01], [lr, lg, lb, 0.3]));
        lines.push(Vertex::pos_color([end.x, end.y, 0.01], [lr, lg, lb, 0.3]));
    }
    r.draw_lines(&lines);

    r.set_line_smooth(false);
    r.disable_blend();
    r.set_lighting(true);
}

/// Draw trail shadow projected onto the floor.
pub fn draw_trail_shadow(r: &mut Renderer, data: &PlayerData) {
    if data.trail_offset == 0 || data.trail_height <= 0.0 {
        return;
    }

    let trail_h = data.trail_height;
    let segments = &data.trails[..=data.trail_offset.min(data.trails.len() - 1)];

    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
    r.set_lighting(false);

    r.push_mv();
    r.mult_matrix(&SHADOW_MATRIX);

    let shadow_color = [0.0, 0.0, 0.0, 0.5];
    let mut tris = Vec::new();
    for seg in segments {
        if seg.direction.length_squared() < 1e-6 {
            continue;
        }
        let s = seg.start;
        let e = seg.end();
        let verts = renderer::quad_verts(
            [[s.x, s.y, 0.0], [e.x, e.y, 0.0], [e.x, e.y, trail_h], [s.x, s.y, trail_h]],
            shadow_color,
        );
        tris.extend_from_slice(&verts);
    }
    r.draw_triangles(&tris);

    r.pop_mv();
    r.set_lighting(true);
    r.disable_blend();
}
