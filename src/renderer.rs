// Modern OpenGL 3.3 Core renderer via glow.
// Provides matrix stacks, explicit vertex array drawing, static meshes,
// shader/uniform management, and state tracking for the uber-shader.
// No legacy GL emulation (no begin/end, no primitive conversion).

use glow::HasContext;
use glam::{Mat4, Vec3, Vec4};
use std::collections::HashMap;

// ── Re-export glow constants used by other modules ──────────────────────────

pub use glow::{
    TRIANGLES, TRIANGLE_FAN, TRIANGLE_STRIP, LINES, LINE_STRIP, LINE_LOOP,
    TEXTURE_2D, DEPTH_TEST, BLEND, STENCIL_TEST, SCISSOR_TEST, CULL_FACE,
    POLYGON_OFFSET_FILL,
    FRONT, BACK, FRONT_AND_BACK, CCW, CW, LINE, FILL,
    SRC_ALPHA, ONE_MINUS_SRC_ALPHA, ONE,
    COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, STENCIL_BUFFER_BIT,
    ALWAYS, EQUAL, KEEP, REPLACE, LEQUAL, LESS, NEVER,
    REPEAT, CLAMP_TO_EDGE, LINEAR, LINEAR_MIPMAP_LINEAR,
    TEXTURE_WRAP_S, TEXTURE_WRAP_T, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER,
    RGBA, RGB, UNSIGNED_BYTE, UNSIGNED_INT, UNSIGNED_SHORT, FLOAT,
    ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER, STATIC_DRAW, DYNAMIC_DRAW,
    TEXTURE_WIDTH, TEXTURE_HEIGHT,
    VERSION, RENDERER as GL_RENDERER,
    PACK_ALIGNMENT,
};

// ── Shader sources ──────────────────────────────────────────────────────────

const VERT_BODY: &str = r#"
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_texcoord;
layout(location = 3) in vec4 a_color;

uniform mat4 u_projection;
uniform mat4 u_modelview;

out vec3 v_eye_pos;
out vec3 v_normal;
out vec2 v_texcoord;
out vec4 v_color;

void main() {
    vec4 ep = u_modelview * vec4(a_pos, 1.0);
    v_eye_pos = ep.xyz;
    v_normal = mat3(u_modelview) * a_normal;
    v_texcoord = a_texcoord;
    v_color = a_color;
    gl_Position = u_projection * ep;
}
"#;

const FRAG_BODY: &str = r#"
in vec3 v_eye_pos;
in vec3 v_normal;
in vec2 v_texcoord;
in vec4 v_color;

uniform bool u_tex_enabled;
uniform bool u_lit;
uniform bool u_fog_enabled;
uniform bool u_color_mat;
uniform int  u_tex_env;        // 0=modulate, 1=decal
uniform sampler2D u_texture;

// Up to 3 lights
uniform bool  u_light_on[3];
uniform vec4  u_light_pos[3];  // eye-space
uniform vec4  u_light_amb[3];
uniform vec4  u_light_diff[3];
uniform vec4  u_light_spec[3];

// Material
uniform vec4  u_mat_amb;
uniform vec4  u_mat_diff;
uniform vec4  u_mat_spec;
uniform float u_mat_shin;

// Fog
uniform vec4  u_fog_color;
uniform float u_fog_start;
uniform float u_fog_end;

out vec4 frag_color;

void main() {
    vec4 base = v_color;

    if (u_lit) {
        vec4 m_amb  = u_color_mat ? v_color : u_mat_amb;
        vec4 m_diff = u_color_mat ? v_color : u_mat_diff;
        vec4 m_spec = u_mat_spec;
        float m_shin = u_mat_shin;
        vec3 N = normalize(v_normal);
        vec3 V = normalize(-v_eye_pos);
        vec3 total_amb = vec3(0.0);
        vec3 total_diff = vec3(0.0);
        vec3 total_spec = vec3(0.0);
        for (int i = 0; i < 3; i++) {
            if (!u_light_on[i]) continue;
            vec3 L;
            if (u_light_pos[i].w == 0.0)
                L = normalize(u_light_pos[i].xyz);
            else
                L = normalize(u_light_pos[i].xyz - v_eye_pos);
            float NdL = max(dot(N, L), 0.0);
            // Two-sided lighting
            if (NdL == 0.0) NdL = max(dot(-N, L), 0.0);
            vec3 H = normalize(L + V);
            float spec = 0.0;
            if (NdL > 0.0 && m_shin > 0.0)
                spec = pow(max(dot(N, H), 0.0), m_shin);
            total_amb  += u_light_amb[i].rgb;
            total_diff += u_light_diff[i].rgb * NdL;
            total_spec += u_light_spec[i].rgb * spec;
        }
        base = vec4(
            m_amb.rgb * total_amb + m_diff.rgb * total_diff + m_spec.rgb * total_spec,
            m_diff.a
        );
    }

    if (u_tex_enabled) {
        vec4 tc = texture(u_texture, v_texcoord);
        if (u_tex_env == 1) {
            // DECAL: blend using texture alpha
            base = vec4(mix(base.rgb, tc.rgb, tc.a), base.a);
        } else {
            // MODULATE
            base *= tc;
        }
    }

    if (u_fog_enabled) {
        float d = length(v_eye_pos);
        float f = clamp((u_fog_end - d) / (u_fog_end - u_fog_start), 0.0, 1.0);
        base = vec4(mix(u_fog_color.rgb, base.rgb, f), base.a);
    }

    frag_color = base;
}
"#;

// ── Vertex type ─────────────────────────────────────────────────────────────

const VERTEX_BYTES: i32 = 48; // 12 floats * 4 bytes

/// Interleaved vertex: pos(3) + normal(3) + texcoord(2) + color(4) = 48 bytes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub texcoord: [f32; 2],
    pub color: [f32; 4],
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            pos: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            texcoord: [0.0; 2],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl Vertex {
    pub fn pos_color(pos: [f32; 3], color: [f32; 4]) -> Self {
        Self { pos, normal: [0.0, 0.0, 1.0], texcoord: [0.0, 0.0], color }
    }

    pub fn pos_color_2d(x: f32, y: f32, color: [f32; 4]) -> Self {
        Self::pos_color([x, y, 0.0], color)
    }

    pub fn textured(pos: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self { pos, normal: [0.0, 0.0, 1.0], texcoord: uv, color }
    }

    pub fn full(pos: [f32; 3], normal: [f32; 3], uv: [f32; 2], color: [f32; 4]) -> Self {
        Self { pos, normal, texcoord: uv, color }
    }
}

// ── Geometry helpers ────────────────────────────────────────────────────────

/// Convert a quad (4 corners CCW) to 2 triangles (6 vertices).
pub fn quad_verts(p: [[f32; 3]; 4], color: [f32; 4]) -> [Vertex; 6] {
    [
        Vertex::pos_color(p[0], color), Vertex::pos_color(p[1], color), Vertex::pos_color(p[2], color),
        Vertex::pos_color(p[0], color), Vertex::pos_color(p[2], color), Vertex::pos_color(p[3], color),
    ]
}

/// Convert a quad with per-vertex colors to 2 triangles.
pub fn quad_verts_colored(p: [[f32; 3]; 4], c: [[f32; 4]; 4]) -> [Vertex; 6] {
    [
        Vertex::pos_color(p[0], c[0]), Vertex::pos_color(p[1], c[1]), Vertex::pos_color(p[2], c[2]),
        Vertex::pos_color(p[0], c[0]), Vertex::pos_color(p[2], c[2]), Vertex::pos_color(p[3], c[3]),
    ]
}

/// Convert a textured quad to 2 triangles.
pub fn textured_quad_verts(p: [[f32; 3]; 4], uv: [[f32; 2]; 4], color: [f32; 4]) -> [Vertex; 6] {
    [
        Vertex::textured(p[0], uv[0], color), Vertex::textured(p[1], uv[1], color), Vertex::textured(p[2], uv[2], color),
        Vertex::textured(p[0], uv[0], color), Vertex::textured(p[2], uv[2], color), Vertex::textured(p[3], uv[3], color),
    ]
}

/// Build a textured quad with per-vertex normals to 2 triangles.
pub fn textured_quad_verts_n(p: [[f32; 3]; 4], uv: [[f32; 2]; 4], n: [f32; 3], color: [f32; 4]) -> [Vertex; 6] {
    [
        Vertex::full(p[0], n, uv[0], color), Vertex::full(p[1], n, uv[1], color), Vertex::full(p[2], n, uv[2], color),
        Vertex::full(p[0], n, uv[0], color), Vertex::full(p[2], n, uv[2], color), Vertex::full(p[3], n, uv[3], color),
    ]
}

/// Build a ring (annulus arc) as triangles.
/// `color_fn` takes fraction 0..1 and returns color.
pub fn ring_verts(
    cx: f32, cy: f32, r_inner: f32, r_outer: f32,
    angle_start: f32, angle_end: f32, segments: usize,
    color_fn: impl Fn(f32, bool) -> [f32; 4],
) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(segments * 6);
    for s in 0..segments {
        let t0 = s as f32 / segments as f32;
        let t1 = (s + 1) as f32 / segments as f32;
        let a0 = angle_start + t0 * (angle_end - angle_start);
        let a1 = angle_start + t1 * (angle_end - angle_start);
        let ci0 = color_fn(t0, true);
        let co0 = color_fn(t0, false);
        let ci1 = color_fn(t1, true);
        let co1 = color_fn(t1, false);
        let (c0, s0) = (a0.cos(), a0.sin());
        let (c1, s1) = (a1.cos(), a1.sin());
        // inner0, outer0, outer1 triangle
        out.push(Vertex::pos_color_2d(cx + r_inner * c0, cy + r_inner * s0, ci0));
        out.push(Vertex::pos_color_2d(cx + r_outer * c0, cy + r_outer * s0, co0));
        out.push(Vertex::pos_color_2d(cx + r_outer * c1, cy + r_outer * s1, co1));
        // inner0, outer1, inner1 triangle
        out.push(Vertex::pos_color_2d(cx + r_inner * c0, cy + r_inner * s0, ci0));
        out.push(Vertex::pos_color_2d(cx + r_outer * c1, cy + r_outer * s1, co1));
        out.push(Vertex::pos_color_2d(cx + r_inner * c1, cy + r_inner * s1, ci1));
    }
    out
}

/// Build a triangle fan as triangles. First element is center.
pub fn fan_verts(center: Vertex, perimeter: &[Vertex]) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(perimeter.len().saturating_sub(1) * 3);
    for i in 0..perimeter.len().saturating_sub(1) {
        out.push(center);
        out.push(perimeter[i]);
        out.push(perimeter[i + 1]);
    }
    out
}

/// Convert a line loop (N points) to line segment pairs for LINES mode.
pub fn line_loop_verts(points: &[Vertex]) -> Vec<Vertex> {
    let n = points.len();
    if n < 2 { return Vec::new(); }
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        out.push(points[i]);
        out.push(points[(i + 1) % n]);
    }
    out
}

/// Convert a QUAD_STRIP (pairs: [inner0, outer0, inner1, outer1, ...]) to triangles.
pub fn quad_strip_to_triangles(pairs: &[Vertex]) -> Vec<Vertex> {
    let n = pairs.len() / 2;
    if n < 2 { return Vec::new(); }
    let mut out = Vec::with_capacity((n - 1) * 6);
    for i in 0..n - 1 {
        let a = i * 2;
        out.push(pairs[a]);     out.push(pairs[a + 1]); out.push(pairs[a + 3]);
        out.push(pairs[a]);     out.push(pairs[a + 3]); out.push(pairs[a + 2]);
    }
    out
}

/// Convert a triangle strip to triangles.
pub fn tri_strip_to_triangles(verts: &[Vertex]) -> Vec<Vertex> {
    let n = verts.len();
    if n < 3 { return Vec::new(); }
    let mut out = Vec::with_capacity((n - 2) * 3);
    for i in 0..n - 2 {
        if i % 2 == 0 {
            out.push(verts[i]); out.push(verts[i + 1]); out.push(verts[i + 2]);
        } else {
            out.push(verts[i + 1]); out.push(verts[i]); out.push(verts[i + 2]);
        }
    }
    out
}

// ── Static mesh ─────────────────────────────────────────────────────────────

/// GPU-resident static mesh (VBO + optional EBO in a VAO).
pub struct StaticMesh {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: Option<glow::Buffer>,
    count: i32,
    mode: u32,
}

// ── Texture mode / fog / material / light ───────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TexMode { Modulate, Decal }

#[derive(Clone, Copy)]
pub struct FogParams {
    pub color: [f32; 4],
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Copy)]
pub struct Material {
    pub ambient: [f32; 4],
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub shininess: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            ambient: [0.2, 0.2, 0.2, 1.0],
            diffuse: [0.8, 0.8, 0.8, 1.0],
            specular: [0.0, 0.0, 0.0, 1.0],
            shininess: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct LightParams {
    pub position: [f32; 4],
    pub ambient: [f32; 4],
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
}

impl Default for LightParams {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 1.0, 0.0],
            ambient: [0.0; 4],
            diffuse: [1.0, 1.0, 1.0, 1.0],
            specular: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

// ── Renderer ────────────────────────────────────────────────────────────────

pub struct Renderer {
    pub ctx: glow::Context,
    program: glow::Program,

    // Dynamic draw resources (streaming VBO/EBO)
    dyn_vao: glow::VertexArray,
    dyn_vbo: glow::Buffer,
    dyn_ebo: glow::Buffer,

    // Texture dimension tracking
    bound_tex: u32,
    tex_sizes: HashMap<u32, (i32, i32)>,

    // WASM handle maps: glow on WebGL2 uses slotmap keys, not u32 GL handles
    #[cfg(target_arch = "wasm32")]
    tex_handles: HashMap<u32, glow::Texture>,
    #[cfg(target_arch = "wasm32")]
    buf_handles: HashMap<u32, glow::Buffer>,
    #[cfg(target_arch = "wasm32")]
    vao_handles: HashMap<u32, glow::VertexArray>,
    #[cfg(target_arch = "wasm32")]
    next_handle_id: u32,

    // Matrix stacks
    projection: Mat4,
    modelview: Mat4,
    proj_stack: Vec<Mat4>,
    mv_stack: Vec<Mat4>,

    // Rendering state
    lighting: bool,
    fog: Option<FogParams>,
    tex_enabled: bool,
    tex_mode: TexMode,
    color_material: bool,

    // Lights (3 max)
    lights: [LightParams; 3],
    light_enabled: [bool; 3],

    // Material
    material: Material,

    // Uniform locations
    u_projection: Option<glow::UniformLocation>,
    u_modelview: Option<glow::UniformLocation>,
    u_tex_enabled: Option<glow::UniformLocation>,
    u_lit: Option<glow::UniformLocation>,
    u_fog_enabled: Option<glow::UniformLocation>,
    u_color_mat: Option<glow::UniformLocation>,
    u_tex_env: Option<glow::UniformLocation>,
    u_texture: Option<glow::UniformLocation>,
    u_light_on: [Option<glow::UniformLocation>; 3],
    u_light_pos: [Option<glow::UniformLocation>; 3],
    u_light_amb: [Option<glow::UniformLocation>; 3],
    u_light_diff: [Option<glow::UniformLocation>; 3],
    u_light_spec: [Option<glow::UniformLocation>; 3],
    u_mat_amb: Option<glow::UniformLocation>,
    u_mat_diff: Option<glow::UniformLocation>,
    u_mat_spec: Option<glow::UniformLocation>,
    u_mat_shin: Option<glow::UniformLocation>,
    u_fog_color: Option<glow::UniformLocation>,
    u_fog_start: Option<glow::UniformLocation>,
    u_fog_end: Option<glow::UniformLocation>,
}

impl Renderer {
    pub fn new(ctx: glow::Context) -> Self {
        unsafe {
            let program = compile_program(&ctx);
            ctx.use_program(Some(program));

            let dyn_vao = ctx.create_vertex_array().unwrap();
            let dyn_vbo = ctx.create_buffer().unwrap();
            let dyn_ebo = ctx.create_buffer().unwrap();
            ctx.bind_vertex_array(Some(dyn_vao));
            ctx.bind_buffer(glow::ARRAY_BUFFER, Some(dyn_vbo));
            setup_vertex_attribs(&ctx);
            ctx.bind_vertex_array(None);

            let u = |name: &str| ctx.get_uniform_location(program, name);
            let ua = |base: &str, i: usize| ctx.get_uniform_location(program, &format!("{base}[{i}]"));

            let r = Self {
                program,
                dyn_vao,
                dyn_vbo,
                dyn_ebo,
                bound_tex: 0,
                tex_sizes: HashMap::new(),
                #[cfg(target_arch = "wasm32")]
                tex_handles: HashMap::new(),
                #[cfg(target_arch = "wasm32")]
                buf_handles: HashMap::new(),
                #[cfg(target_arch = "wasm32")]
                vao_handles: HashMap::new(),
                #[cfg(target_arch = "wasm32")]
                next_handle_id: 0,
                projection: Mat4::IDENTITY,
                modelview: Mat4::IDENTITY,
                proj_stack: Vec::new(),
                mv_stack: Vec::new(),
                lighting: false,
                fog: None,
                tex_enabled: false,
                tex_mode: TexMode::Modulate,
                color_material: false,
                lights: std::array::from_fn(|_| LightParams::default()),
                light_enabled: [false; 3],
                material: Material::default(),
                u_projection: u("u_projection"),
                u_modelview: u("u_modelview"),
                u_tex_enabled: u("u_tex_enabled"),
                u_lit: u("u_lit"),
                u_fog_enabled: u("u_fog_enabled"),
                u_color_mat: u("u_color_mat"),
                u_tex_env: u("u_tex_env"),
                u_texture: u("u_texture"),
                u_light_on: [ua("u_light_on", 0), ua("u_light_on", 1), ua("u_light_on", 2)],
                u_light_pos: [ua("u_light_pos", 0), ua("u_light_pos", 1), ua("u_light_pos", 2)],
                u_light_amb: [ua("u_light_amb", 0), ua("u_light_amb", 1), ua("u_light_amb", 2)],
                u_light_diff: [ua("u_light_diff", 0), ua("u_light_diff", 1), ua("u_light_diff", 2)],
                u_light_spec: [ua("u_light_spec", 0), ua("u_light_spec", 1), ua("u_light_spec", 2)],
                u_mat_amb: u("u_mat_amb"),
                u_mat_diff: u("u_mat_diff"),
                u_mat_spec: u("u_mat_spec"),
                u_mat_shin: u("u_mat_shin"),
                u_fog_color: u("u_fog_color"),
                u_fog_start: u("u_fog_start"),
                u_fog_end: u("u_fog_end"),
                ctx,
            };
            r.ctx.uniform_1_i32(r.u_texture.as_ref(), 0);
            r
        }
    }

    // ── Matrix operations (separate projection/modelview) ────────────────

    pub fn set_projection(&mut self, m: &Mat4) { self.projection = *m; }

    pub fn push_projection(&mut self) { self.proj_stack.push(self.projection); }

    pub fn pop_projection(&mut self) {
        if let Some(m) = self.proj_stack.pop() { self.projection = m; }
    }

    pub fn push_mv(&mut self) { self.mv_stack.push(self.modelview); }

    pub fn pop_mv(&mut self) {
        if let Some(m) = self.mv_stack.pop() { self.modelview = m; }
    }

    pub fn set_modelview(&mut self, m: &Mat4) { self.modelview = *m; }

    pub fn load_identity_mv(&mut self) { self.modelview = Mat4::IDENTITY; }

    pub fn mult_matrix(&mut self, m: &[f32; 16]) {
        self.modelview = self.modelview * Mat4::from_cols_array(m);
    }

    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.modelview = self.modelview * Mat4::from_translation(Vec3::new(x, y, z));
    }

    pub fn rotate_deg(&mut self, deg: f32, x: f32, y: f32, z: f32) {
        let axis = Vec3::new(x, y, z);
        if axis.length_squared() < 1e-10 { return; }
        self.modelview = self.modelview * Mat4::from_axis_angle(axis.normalize(), deg.to_radians());
    }

    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.modelview = self.modelview * Mat4::from_scale(Vec3::new(x, y, z));
    }

    pub fn get_modelview(&self) -> Mat4 { self.modelview }

    pub fn get_modelview_array(&self) -> [f32; 16] { self.modelview.to_cols_array() }

    /// Set a perspective projection matrix.
    pub fn set_perspective(&mut self, fov_deg: f32, aspect: f32, near: f32, far: f32) {
        let f = 1.0 / (fov_deg.to_radians() / 2.0).tan();
        self.projection = Mat4::from_cols_array(&[
            f / aspect, 0.0, 0.0, 0.0,
            0.0, f, 0.0, 0.0,
            0.0, 0.0, (far + near) / (near - far), -1.0,
            0.0, 0.0, 2.0 * far * near / (near - far), 0.0,
        ]);
    }

    /// Set an orthographic projection for 2D rendering.
    pub fn set_ortho(&mut self, w: f32, h: f32) {
        self.projection = Mat4::from_cols_array(&[
            2.0 / w, 0.0, 0.0, 0.0,
            0.0, 2.0 / h, 0.0, 0.0,
            0.0, 0.0, -1.0, 0.0,
            -1.0, -1.0, 0.0, 1.0,
        ]);
    }

    // ── State management ────────────────────────────────────────────────

    pub fn set_lighting(&mut self, on: bool) { self.lighting = on; }
    pub fn set_color_material(&mut self, on: bool) { self.color_material = on; }

    pub fn set_fog(&mut self, fog: Option<FogParams>) { self.fog = fog; }

    pub fn set_texture(&mut self, tex: Option<u32>) {
        match tex {
            Some(id) if id != 0 => {
                self.tex_enabled = true;
                self.bound_tex = id;
                unsafe {
                    self.ctx.bind_texture(glow::TEXTURE_2D, Some(self.u32_to_texture(id)));
                }
            }
            _ => {
                self.tex_enabled = false;
                self.bound_tex = 0;
                unsafe { self.ctx.bind_texture(glow::TEXTURE_2D, None); }
            }
        }
    }

    pub fn set_tex_mode(&mut self, mode: TexMode) { self.tex_mode = mode; }

    pub fn enable_light(&mut self, idx: usize, on: bool) {
        if idx < 3 { self.light_enabled[idx] = on; }
    }

    /// Set light parameters. Position is transformed by current modelview.
    pub fn set_light(&mut self, idx: usize, params: &LightParams) {
        if idx >= 3 { return; }
        let p = Vec4::new(params.position[0], params.position[1],
                          params.position[2], params.position[3]);
        let t = self.modelview * p;
        self.lights[idx] = params.clone();
        self.lights[idx].position = [t.x, t.y, t.z, t.w];
    }

    /// Set light parameters WITHOUT transforming position (already in eye space).
    pub fn set_light_raw(&mut self, idx: usize, params: &LightParams) {
        if idx < 3 { self.lights[idx] = params.clone(); }
    }

    pub fn set_material(&mut self, mat: &Material) {
        self.material = *mat;
        self.material.shininess = self.material.shininess.min(128.0);
    }

    // ── GL state (thin wrappers around real GL calls) ────────────────────

    pub fn set_blend(&mut self, src: u32, dst: u32) {
        unsafe { self.ctx.enable(glow::BLEND); self.ctx.blend_func(src, dst); }
    }

    pub fn disable_blend(&mut self) {
        unsafe { self.ctx.disable(glow::BLEND); }
    }

    pub fn set_depth_test(&mut self, on: bool) {
        unsafe { if on { self.ctx.enable(glow::DEPTH_TEST); } else { self.ctx.disable(glow::DEPTH_TEST); } }
    }

    pub fn set_depth_write(&mut self, on: bool) {
        unsafe { self.ctx.depth_mask(on); }
    }

    pub fn depth_func(&mut self, func: u32) {
        unsafe { self.ctx.depth_func(func); }
    }

    pub fn set_cull_face(&mut self, on: bool) {
        unsafe { if on { self.ctx.enable(glow::CULL_FACE); } else { self.ctx.disable(glow::CULL_FACE); } }
    }

    pub fn cull_face(&mut self, mode: u32) {
        unsafe { self.ctx.cull_face(mode); }
    }

    pub fn front_face(&mut self, mode: u32) {
        unsafe { self.ctx.front_face(mode); }
    }

    pub fn viewport(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe { self.ctx.viewport(x, y, w, h); }
    }

    pub fn scissor(&self, x: i32, y: i32, w: i32, h: i32) {
        unsafe { self.ctx.scissor(x, y, w, h); }
    }

    pub fn set_scissor_test(&mut self, on: bool) {
        unsafe { if on { self.ctx.enable(glow::SCISSOR_TEST); } else { self.ctx.disable(glow::SCISSOR_TEST); } }
    }

    pub fn clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        unsafe { self.ctx.clear_color(r, g, b, a); }
    }

    pub fn clear(&self, mask: u32) {
        unsafe { self.ctx.clear(mask); }
    }

    pub fn clear_stencil(&self, val: i32) {
        unsafe { self.ctx.clear_stencil(val); }
    }

    pub fn set_stencil_test(&mut self, on: bool) {
        unsafe { if on { self.ctx.enable(glow::STENCIL_TEST); } else { self.ctx.disable(glow::STENCIL_TEST); } }
    }

    pub fn stencil_func(&self, func: u32, ref_val: i32, mask: u32) {
        unsafe { self.ctx.stencil_func(func, ref_val, mask); }
    }

    pub fn stencil_op(&self, sfail: u32, dpfail: u32, dppass: u32) {
        unsafe { self.ctx.stencil_op(sfail, dpfail, dppass); }
    }

    pub fn stencil_mask(&self, mask: u32) {
        unsafe { self.ctx.stencil_mask(mask); }
    }

    pub fn color_mask(&self, r: bool, g: bool, b: bool, a: bool) {
        unsafe { self.ctx.color_mask(r, g, b, a); }
    }

    pub fn line_width(&self, w: f32) {
        unsafe { self.ctx.line_width(w); }
    }

    pub fn set_line_smooth(&mut self, _on: bool) {
        // GL_LINE_SMOOTH (0x0B20) is not supported on WebGL
        #[cfg(not(target_arch = "wasm32"))]
        unsafe {
            if _on { self.ctx.enable(0x0B20); }
            else { self.ctx.disable(0x0B20); }
        }
    }

    pub fn polygon_mode(&self, face: u32, mode: u32) {
        unsafe { self.ctx.polygon_mode(face, mode); }
    }

    pub fn set_polygon_offset(&mut self, on: bool, factor: f32, units: f32) {
        unsafe {
            if on {
                self.ctx.enable(glow::POLYGON_OFFSET_FILL);
                self.ctx.polygon_offset(factor, units);
            } else {
                self.ctx.disable(glow::POLYGON_OFFSET_FILL);
            }
        }
    }

    pub fn pixel_store_i(&self, param: u32, val: i32) {
        unsafe { self.ctx.pixel_store_i32(param, val); }
    }

    // ── Uniform upload ──────────────────────────────────────────────────

    fn upload_uniforms(&self) {
        unsafe {
            self.ctx.use_program(Some(self.program));
            self.ctx.uniform_matrix_4_f32_slice(self.u_projection.as_ref(), false, &self.projection.to_cols_array());
            self.ctx.uniform_matrix_4_f32_slice(self.u_modelview.as_ref(), false, &self.modelview.to_cols_array());
            self.ctx.uniform_1_i32(self.u_tex_enabled.as_ref(), self.tex_enabled as i32);
            self.ctx.uniform_1_i32(self.u_lit.as_ref(), self.lighting as i32);
            self.ctx.uniform_1_i32(self.u_fog_enabled.as_ref(), self.fog.is_some() as i32);
            self.ctx.uniform_1_i32(self.u_color_mat.as_ref(), self.color_material as i32);
            self.ctx.uniform_1_i32(self.u_tex_env.as_ref(), if self.tex_mode == TexMode::Decal { 1 } else { 0 });

            for i in 0..3 {
                self.ctx.uniform_1_i32(self.u_light_on[i].as_ref(), self.light_enabled[i] as i32);
                self.ctx.uniform_4_f32_slice(self.u_light_pos[i].as_ref(), &self.lights[i].position);
                self.ctx.uniform_4_f32_slice(self.u_light_amb[i].as_ref(), &self.lights[i].ambient);
                self.ctx.uniform_4_f32_slice(self.u_light_diff[i].as_ref(), &self.lights[i].diffuse);
                self.ctx.uniform_4_f32_slice(self.u_light_spec[i].as_ref(), &self.lights[i].specular);
            }

            self.ctx.uniform_4_f32_slice(self.u_mat_amb.as_ref(), &self.material.ambient);
            self.ctx.uniform_4_f32_slice(self.u_mat_diff.as_ref(), &self.material.diffuse);
            self.ctx.uniform_4_f32_slice(self.u_mat_spec.as_ref(), &self.material.specular);
            self.ctx.uniform_1_f32(self.u_mat_shin.as_ref(), self.material.shininess);

            if let Some(ref fog) = self.fog {
                self.ctx.uniform_4_f32_slice(self.u_fog_color.as_ref(), &fog.color);
                self.ctx.uniform_1_f32(self.u_fog_start.as_ref(), fog.start);
                self.ctx.uniform_1_f32(self.u_fog_end.as_ref(), fog.end);
            }
        }
    }

    /// Public uniform sync for external VAO draws (models).
    pub fn sync_uniforms(&self) { self.upload_uniforms(); }

    // ── Dynamic drawing ─────────────────────────────────────────────────

    /// Draw non-indexed triangles.
    pub fn draw_triangles(&mut self, verts: &[Vertex]) {
        if verts.is_empty() { return; }
        self.upload_uniforms();
        unsafe {
            self.ctx.bind_vertex_array(Some(self.dyn_vao));
            self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.dyn_vbo));
            let bytes = std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<Vertex>(),
            );
            self.ctx.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            self.ctx.draw_arrays(glow::TRIANGLES, 0, verts.len() as i32);
            self.ctx.bind_vertex_array(None);
        }
    }

    /// Draw non-indexed lines (pairs of vertices).
    pub fn draw_lines(&mut self, verts: &[Vertex]) {
        if verts.is_empty() { return; }
        self.upload_uniforms();
        unsafe {
            self.ctx.bind_vertex_array(Some(self.dyn_vao));
            self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.dyn_vbo));
            let bytes = std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<Vertex>(),
            );
            self.ctx.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            self.ctx.draw_arrays(glow::LINES, 0, verts.len() as i32);
            self.ctx.bind_vertex_array(None);
        }
    }

    /// Draw a non-indexed line strip.
    pub fn draw_line_strip(&mut self, verts: &[Vertex]) {
        if verts.is_empty() { return; }
        self.upload_uniforms();
        unsafe {
            self.ctx.bind_vertex_array(Some(self.dyn_vao));
            self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.dyn_vbo));
            let bytes = std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<Vertex>(),
            );
            self.ctx.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            self.ctx.draw_arrays(glow::LINE_STRIP, 0, verts.len() as i32);
            self.ctx.bind_vertex_array(None);
        }
    }

    /// Draw indexed triangles (u16 indices).
    pub fn draw_indexed(&mut self, verts: &[Vertex], indices: &[u16]) {
        if verts.is_empty() || indices.is_empty() { return; }
        self.upload_uniforms();
        unsafe {
            self.ctx.bind_vertex_array(Some(self.dyn_vao));
            self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(self.dyn_vbo));
            let vbytes = std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<Vertex>(),
            );
            self.ctx.buffer_data_u8_slice(glow::ARRAY_BUFFER, vbytes, glow::STREAM_DRAW);
            self.ctx.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.dyn_ebo));
            let ibytes = std::slice::from_raw_parts(
                indices.as_ptr() as *const u8,
                indices.len() * 2,
            );
            self.ctx.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, ibytes, glow::STREAM_DRAW);
            self.ctx.draw_elements(glow::TRIANGLES, indices.len() as i32, glow::UNSIGNED_SHORT, 0);
            self.ctx.bind_vertex_array(None);
        }
    }

    // ── Static mesh ─────────────────────────────────────────────────────

    /// Create a GPU-resident static mesh. Upload once, draw many times.
    pub fn create_static_mesh(&mut self, verts: &[Vertex], indices: Option<&[u16]>, mode: u32) -> StaticMesh {
        unsafe {
            let vao = self.ctx.create_vertex_array().unwrap();
            let vbo = self.ctx.create_buffer().unwrap();
            self.ctx.bind_vertex_array(Some(vao));
            self.ctx.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let bytes = std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<Vertex>(),
            );
            self.ctx.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            setup_vertex_attribs(&self.ctx);

            let (ebo, count) = if let Some(idx) = indices {
                let ebo_handle = self.ctx.create_buffer().unwrap();
                self.ctx.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo_handle));
                let ibytes = std::slice::from_raw_parts(
                    idx.as_ptr() as *const u8,
                    idx.len() * 2,
                );
                self.ctx.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, ibytes, glow::STATIC_DRAW);
                (Some(ebo_handle), idx.len() as i32)
            } else {
                (None, verts.len() as i32)
            };

            self.ctx.bind_vertex_array(None);
            StaticMesh { vao, vbo, ebo, count, mode }
        }
    }

    /// Draw a static mesh.
    pub fn draw_static_mesh(&mut self, mesh: &StaticMesh) {
        self.upload_uniforms();
        unsafe {
            self.ctx.bind_vertex_array(Some(mesh.vao));
            if mesh.ebo.is_some() {
                self.ctx.draw_elements(mesh.mode, mesh.count, glow::UNSIGNED_SHORT, 0);
            } else {
                self.ctx.draw_arrays(mesh.mode, 0, mesh.count);
            }
            self.ctx.bind_vertex_array(None);
        }
    }

    // ── Model VAO support ───────────────────────────────────────────────

    pub fn bind_model_vao(&self, vao: u32, ebo: u32) {
        unsafe {
            let v = if vao != 0 { Some(self.u32_to_vao(vao)) } else { None };
            self.ctx.bind_vertex_array(v);
            let b = if ebo != 0 { Some(self.u32_to_buffer(ebo)) } else { None };
            self.ctx.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, b);
        }
    }

    pub fn unbind_model_vao(&self) {
        unsafe {
            self.ctx.bind_vertex_array(None);
            self.ctx.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        }
    }

    pub fn draw_elements_u32(&self, count: i32, offset: i32) {
        unsafe {
            self.ctx.draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, offset * 4);
        }
    }

    pub fn vertex_attrib_4f(&self, index: u32, x: f32, y: f32, z: f32, w: f32) {
        unsafe { self.ctx.vertex_attrib_4_f32(index, x, y, z, w); }
    }

    // ── Texture helpers ─────────────────────────────────────────────────

    pub fn bind_texture_raw(&mut self, id: u32) {
        self.bound_tex = id;
        unsafe {
            let tex = if id != 0 { Some(self.u32_to_texture(id)) } else { None };
            self.ctx.bind_texture(glow::TEXTURE_2D, tex);
        }
    }

    pub fn gen_texture(&mut self) -> u32 {
        let tex = unsafe { self.ctx.create_texture().unwrap() };
        self.texture_to_u32(tex)
    }

    pub fn tex_parameter_i(&self, target: u32, param: u32, val: i32) {
        unsafe { self.ctx.tex_parameter_i32(target, param, val); }
    }

    pub fn tex_parameter_f(&self, target: u32, param: u32, val: f32) {
        unsafe { self.ctx.tex_parameter_f32(target, param, val); }
    }

    pub fn tex_image_2d(&mut self, target: u32, level: i32, internal: i32, w: i32, h: i32,
                        format: u32, ty: u32, data: &[u8]) {
        if level == 0 && self.bound_tex != 0 {
            self.tex_sizes.insert(self.bound_tex, (w, h));
        }
        unsafe {
            self.ctx.tex_image_2d(target, level, internal, w, h, 0, format, ty,
                                  glow::PixelUnpackData::Slice(Some(data)));
        }
    }

    pub fn get_tex_dimensions(&self, tex_id: u32) -> (i32, i32) {
        self.tex_sizes.get(&tex_id).copied().unwrap_or((0, 0))
    }

    /// Query dimensions of currently bound texture (for backwards compat).
    pub fn get_tex_level_parameter_i(&self, _target: u32, _level: i32, param: u32) -> i32 {
        match param {
            glow::TEXTURE_WIDTH => self.tex_sizes.get(&self.bound_tex).map_or(0, |s| s.0),
            glow::TEXTURE_HEIGHT => self.tex_sizes.get(&self.bound_tex).map_or(0, |s| s.1),
            _ => 0,
        }
    }

    // ── Buffer helpers ──────────────────────────────────────────────────

    pub fn gen_buffer(&mut self) -> u32 {
        let buf = unsafe { self.ctx.create_buffer().unwrap() };
        self.buffer_to_u32(buf)
    }

    pub fn bind_buffer(&self, target: u32, buf: u32) {
        unsafe {
            let b = if buf != 0 { Some(self.u32_to_buffer(buf)) } else { None };
            self.ctx.bind_buffer(target, b);
        }
    }

    pub fn buffer_data(&self, target: u32, data: &[u8], usage: u32) {
        unsafe { self.ctx.buffer_data_u8_slice(target, data, usage); }
    }

    // ── VAO helpers ─────────────────────────────────────────────────────

    pub fn gen_vertex_array(&mut self) -> u32 {
        let vao = unsafe { self.ctx.create_vertex_array().unwrap() };
        self.vao_to_u32(vao)
    }

    pub fn bind_vertex_array(&self, vao: u32) {
        unsafe {
            let v = if vao != 0 { Some(self.u32_to_vao(vao)) } else { None };
            self.ctx.bind_vertex_array(v);
        }
    }

    pub fn enable_vertex_attrib(&self, index: u32) {
        unsafe { self.ctx.enable_vertex_attrib_array(index); }
    }

    pub fn vertex_attrib_pointer_f32(&self, index: u32, size: i32, stride: i32, offset: i32) {
        unsafe {
            self.ctx.vertex_attrib_pointer_f32(index, size, glow::FLOAT, false, stride, offset);
        }
    }

    // ── Screenshot / Query ──────────────────────────────────────────────

    pub fn read_pixels(&self, x: i32, y: i32, w: i32, h: i32, format: u32, ty: u32, buf: &mut [u8]) {
        unsafe {
            self.ctx.read_pixels(x, y, w, h, format, ty, glow::PixelPackData::Slice(Some(buf)));
        }
    }

    pub fn get_string(&self, name: u32) -> String {
        unsafe { self.ctx.get_parameter_string(name) }
    }

    // ── Handle conversion helpers (u32 <-> glow types) ───────────────────

    #[cfg(not(target_arch = "wasm32"))]
    fn texture_to_u32(&mut self, tex: glow::Texture) -> u32 { tex.0.get() }
    #[cfg(not(target_arch = "wasm32"))]
    fn u32_to_texture(&self, id: u32) -> glow::Texture {
        glow::NativeTexture(std::num::NonZeroU32::new(id).unwrap())
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn buffer_to_u32(&mut self, buf: glow::Buffer) -> u32 { buf.0.get() }
    #[cfg(not(target_arch = "wasm32"))]
    fn u32_to_buffer(&self, id: u32) -> glow::Buffer {
        glow::NativeBuffer(std::num::NonZeroU32::new(id).unwrap())
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn vao_to_u32(&mut self, vao: glow::VertexArray) -> u32 { vao.0.get() }
    #[cfg(not(target_arch = "wasm32"))]
    fn u32_to_vao(&self, id: u32) -> glow::VertexArray {
        glow::NativeVertexArray(std::num::NonZeroU32::new(id).unwrap())
    }

    #[cfg(target_arch = "wasm32")]
    fn texture_to_u32(&mut self, tex: glow::Texture) -> u32 {
        self.next_handle_id += 1;
        self.tex_handles.insert(self.next_handle_id, tex);
        self.next_handle_id
    }
    #[cfg(target_arch = "wasm32")]
    fn u32_to_texture(&self, id: u32) -> glow::Texture {
        *self.tex_handles.get(&id).expect("invalid texture handle")
    }
    #[cfg(target_arch = "wasm32")]
    fn buffer_to_u32(&mut self, buf: glow::Buffer) -> u32 {
        self.next_handle_id += 1;
        self.buf_handles.insert(self.next_handle_id, buf);
        self.next_handle_id
    }
    #[cfg(target_arch = "wasm32")]
    fn u32_to_buffer(&self, id: u32) -> glow::Buffer {
        *self.buf_handles.get(&id).expect("invalid buffer handle")
    }
    #[cfg(target_arch = "wasm32")]
    fn vao_to_u32(&mut self, vao: glow::VertexArray) -> u32 {
        self.next_handle_id += 1;
        self.vao_handles.insert(self.next_handle_id, vao);
        self.next_handle_id
    }
    #[cfg(target_arch = "wasm32")]
    fn u32_to_vao(&self, id: u32) -> glow::VertexArray {
        *self.vao_handles.get(&id).expect("invalid VAO handle")
    }
}

// ── Shader compilation ──────────────────────────────────────────────────────

unsafe fn compile_program(ctx: &glow::Context) -> glow::Program {
    unsafe {
        // GLSL 330 core for desktop OpenGL, 300 es for WebGL2
        #[cfg(not(target_arch = "wasm32"))]
        let (vert_src, frag_src) = (
            format!("#version 330 core\n{VERT_BODY}"),
            format!("#version 330 core\n{FRAG_BODY}"),
        );
        #[cfg(target_arch = "wasm32")]
        let (vert_src, frag_src) = (
            format!("#version 300 es\nprecision mediump float;\n{VERT_BODY}"),
            format!("#version 300 es\nprecision mediump float;\n{FRAG_BODY}"),
        );

        let vert = ctx.create_shader(glow::VERTEX_SHADER).unwrap();
        ctx.shader_source(vert, &vert_src);
        ctx.compile_shader(vert);
        if !ctx.get_shader_compile_status(vert) {
            panic!("Vertex shader error: {}", ctx.get_shader_info_log(vert));
        }

        let frag = ctx.create_shader(glow::FRAGMENT_SHADER).unwrap();
        ctx.shader_source(frag, &frag_src);
        ctx.compile_shader(frag);
        if !ctx.get_shader_compile_status(frag) {
            panic!("Fragment shader error: {}", ctx.get_shader_info_log(frag));
        }

        let prog = ctx.create_program().unwrap();
        ctx.attach_shader(prog, vert);
        ctx.attach_shader(prog, frag);
        ctx.link_program(prog);
        if !ctx.get_program_link_status(prog) {
            panic!("Shader link error: {}", ctx.get_program_info_log(prog));
        }

        ctx.delete_shader(vert);
        ctx.delete_shader(frag);
        prog
    }
}

/// Set up vertex attribute pointers for the interleaved Vertex format.
unsafe fn setup_vertex_attribs(ctx: &glow::Context) {
    unsafe {
        // pos(3) + normal(3) + texcoord(2) + color(4) = 12 floats = 48 bytes
        ctx.enable_vertex_attrib_array(0);
        ctx.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, VERTEX_BYTES, 0);
        ctx.enable_vertex_attrib_array(1);
        ctx.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, VERTEX_BYTES, 12);
        ctx.enable_vertex_attrib_array(2);
        ctx.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, VERTEX_BYTES, 24);
        ctx.enable_vertex_attrib_array(3);
        ctx.vertex_attrib_pointer_f32(3, 4, glow::FLOAT, false, VERTEX_BYTES, 32);
    }
}
