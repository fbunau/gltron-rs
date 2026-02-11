use crate::renderer::{self, Renderer, Vertex, quad_verts, textured_quad_verts};
use crate::types::*;

/// Floor color: dark navy blue (matches modern Tron aesthetic)
pub const FLOOR_COLOR: [f32; 4] = [0.02, 0.03, 0.18, 0.55];

/// Draw just the floor quad geometry (no state changes).
/// Used for stencil marking and as the semi-transparent floor overlay.
pub fn draw_floor_quad(r: &mut Renderer, grid_size: f32) {
    let half = grid_size / 2.0;
    let verts = quad_verts(
        [[-half, -half, 0.0], [half, -half, 0.0], [half, half, 0.0], [-half, half, 0.0]],
        [1.0, 1.0, 1.0, 1.0],
    );
    r.draw_triangles(&verts);
}

/// Draw the semi-transparent reflective floor surface.
pub fn draw_floor_surface(r: &mut Renderer, grid_size: f32) {
    let half = grid_size / 2.0;
    r.set_lighting(false);
    r.set_texture(None);
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
    r.set_depth_write(true);
    let verts = quad_verts(
        [[-half, -half, 0.0], [half, -half, 0.0], [half, half, 0.0], [-half, half, 0.0]],
        FLOOR_COLOR,
    );
    r.draw_triangles(&verts);
    r.disable_blend();
    r.set_lighting(true);
}

/// Draw grid lines on the floor.
pub fn draw_floor_grid(r: &mut Renderer, grid_size: f32, spacing: f32) {
    r.set_lighting(false);
    r.set_texture(None);

    r.set_fog(Some(renderer::FogParams {
        color: [FLOOR_COLOR[0], FLOOR_COLOR[1], FLOOR_COLOR[2], 1.0],
        start: 80.0,
        end: 400.0,
    }));

    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
    r.set_line_smooth(true);
    r.line_width(1.0);

    let half = grid_size / 2.0;
    let color = [0.4, 0.5, 0.9, 0.7];

    let sp = spacing as i32;
    let gs = grid_size as i32;

    let mut lines = Vec::new();
    let mut i = 0;
    while i <= gs {
        let fi = i as f32 - half;
        lines.push(Vertex::pos_color([-half, fi, 0.01], color));
        lines.push(Vertex::pos_color([half, fi, 0.01], color));
        lines.push(Vertex::pos_color([fi, -half, 0.01], color));
        lines.push(Vertex::pos_color([fi, half, 0.01], color));
        i += sp;
    }
    r.draw_lines(&lines);

    r.set_line_smooth(false);
    r.disable_blend();
    r.set_fog(None);
    r.set_lighting(true);
}

/// Draw 4 arena walls (matching original: height=48, tex tiles, no lighting).
pub fn draw_walls(r: &mut Renderer, grid_size: f32, textures: &[u32]) {
    let half = grid_size / 2.0;
    let h = WALL_H;
    let tex_repeat = grid_size / 240.0;

    let wall_data: [(usize, [[f32; 3]; 4], [[f32; 2]; 4]); 4] = [
        (TEX_WALL1,
         [[-half,-half,0.0], [-half,-half,h], [half,-half,h], [half,-half,0.0]],
         [[tex_repeat, 0.0], [tex_repeat, 1.0], [0.0, 1.0], [0.0, 0.0]]),
        (TEX_WALL2,
         [[half,-half,0.0], [half,-half,h], [half,half,h], [half,half,0.0]],
         [[tex_repeat, 0.0], [tex_repeat, 1.0], [0.0, 1.0], [0.0, 0.0]]),
        (TEX_WALL3,
         [[half,half,0.0], [half,half,h], [-half,half,h], [-half,half,0.0]],
         [[tex_repeat, 0.0], [tex_repeat, 1.0], [0.0, 1.0], [0.0, 0.0]]),
        (TEX_WALL4,
         [[-half,half,0.0], [-half,half,h], [-half,-half,h], [-half,-half,0.0]],
         [[tex_repeat, 0.0], [tex_repeat, 1.0], [0.0, 1.0], [0.0, 0.0]]),
    ];

    r.set_lighting(false);
    r.set_cull_face(true);
    r.disable_blend();
    let white = [1.0, 1.0, 1.0, 1.0];

    for &(tex_idx, ref verts, ref uvs) in &wall_data {
        if tex_idx < textures.len() {
            r.set_texture(Some(textures[tex_idx]));
        }
        let tris = textured_quad_verts(*verts, *uvs, white);
        r.draw_triangles(&tris);
    }

    r.set_texture(None);
    r.set_cull_face(false);
    r.set_lighting(true);
}

/// Draw 6-face skybox.
pub fn draw_skybox(r: &mut Renderer, grid_size: f32, textures: &[u32]) {
    let d = grid_size * 3.0;

    let faces: [[[f32; 3]; 4]; 6] = [
        // front
        [[-d, -d, -d], [d, -d, -d], [d, -d, d], [-d, -d, d]],
        // right
        [[d, -d, -d], [d, d, -d], [d, d, d], [d, -d, d]],
        // back
        [[d, d, -d], [-d, d, -d], [-d, d, d], [d, d, d]],
        // left
        [[-d, d, -d], [-d, -d, -d], [-d, -d, d], [-d, d, d]],
        // top
        [[-d, -d, d], [d, -d, d], [d, d, d], [-d, d, d]],
        // bottom
        [[-d, d, -d], [d, d, -d], [d, -d, -d], [-d, -d, -d]],
    ];

    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let white = [1.0, 1.0, 1.0, 1.0];

    r.set_depth_write(false);
    r.set_lighting(false);

    for (i, verts) in faces.iter().enumerate() {
        let tex_idx = TEX_SKYBOX + i;
        if tex_idx < textures.len() {
            r.set_texture(Some(textures[tex_idx]));
        }
        let tris = textured_quad_verts(*verts, uvs, white);
        r.draw_triangles(&tris);
    }

    r.set_texture(None);
    r.set_lighting(true);
    r.set_depth_write(true);
}
