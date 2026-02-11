use crate::types::*;
use crate::renderer::{self, Renderer, Vertex, textured_quad_verts, Material};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(target_arch = "wasm32")]
use crate::embedded_assets;

#[cfg(not(target_arch = "wasm32"))]
const ASSET_DIR: &str = "assets";

#[cfg(not(target_arch = "wasm32"))]
fn asset_path(sub: &str, name: &str) -> String {
    format!("{ASSET_DIR}/{sub}/{name}")
}

// === Texture Loading ===

#[cfg(not(target_arch = "wasm32"))]
pub fn load_texture(r: &mut Renderer, path: &str, wrap_s: u32, wrap_t: u32) -> u32 {
    load_texture_opts(r, path, wrap_s, wrap_t, false)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_texture_mipmapped(r: &mut Renderer, path: &str, wrap_s: u32, wrap_t: u32) -> u32 {
    load_texture_opts(r, path, wrap_s, wrap_t, true)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_texture_opts(r: &mut Renderer, path: &str, wrap_s: u32, wrap_t: u32, mipmap: bool) -> u32 {
    let img = image::open(path)
        .unwrap_or_else(|e| panic!("Failed to load texture {path}: {e}"))
        .flipv();
    let (w, h) = (img.width(), img.height());

    let tex = r.gen_texture();
    r.bind_texture_raw(tex);
    r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_WRAP_S, wrap_s as i32);
    r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_WRAP_T, wrap_t as i32);
    r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_MAG_FILTER, renderer::LINEAR as i32);

    if mipmap {
        r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_MIN_FILTER, renderer::LINEAR_MIPMAP_LINEAR as i32);

        // Anisotropic filtering
        const GL_TEXTURE_MAX_ANISOTROPY_EXT: u32 = 0x84FE;
        r.tex_parameter_f(renderer::TEXTURE_2D, GL_TEXTURE_MAX_ANISOTROPY_EXT, 16.0);

        // Manually generate and upload mipmap levels
        let mut cur_img = img.into_rgba8();
        r.tex_image_2d(renderer::TEXTURE_2D, 0, renderer::RGBA as i32, w as i32, h as i32,
            renderer::RGBA, renderer::UNSIGNED_BYTE, cur_img.as_raw());

        let mut level = 1i32;
        let mut cur_w = w;
        let mut cur_h = h;
        while cur_w > 1 || cur_h > 1 {
            let new_w = (cur_w / 2).max(1);
            let new_h = (cur_h / 2).max(1);
            cur_img = image::imageops::resize(&cur_img, new_w, new_h, image::imageops::FilterType::Triangle);
            r.tex_image_2d(renderer::TEXTURE_2D, level, renderer::RGBA as i32, new_w as i32, new_h as i32,
                renderer::RGBA, renderer::UNSIGNED_BYTE, cur_img.as_raw());
            cur_w = new_w;
            cur_h = new_h;
            level += 1;
        }
    } else {
        r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_MIN_FILTER, renderer::LINEAR as i32);
        let (format, data) = match img.color() {
            image::ColorType::Rgba8 => (renderer::RGBA, img.into_rgba8().into_raw()),
            image::ColorType::Rgb8 => (renderer::RGB, img.into_rgb8().into_raw()),
            _ => (renderer::RGBA, img.into_rgba8().into_raw()),
        };
        r.tex_image_2d(renderer::TEXTURE_2D, 0, format as i32, w as i32, h as i32,
            format, renderer::UNSIGNED_BYTE, &data);
    }

    tex
}

#[cfg(target_arch = "wasm32")]
fn load_texture_from_memory(r: &mut Renderer, bytes: &[u8], wrap_s: u32, wrap_t: u32, mipmap: bool) -> u32 {
    let img = image::load_from_memory(bytes)
        .unwrap_or_else(|e| panic!("Failed to load texture from memory: {e}"))
        .flipv();
    let (w, h) = (img.width(), img.height());

    let tex = r.gen_texture();
    r.bind_texture_raw(tex);
    r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_WRAP_S, wrap_s as i32);
    r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_WRAP_T, wrap_t as i32);
    r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_MAG_FILTER, renderer::LINEAR as i32);

    if mipmap {
        r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_MIN_FILTER, renderer::LINEAR_MIPMAP_LINEAR as i32);
        let mut cur_img = img.into_rgba8();
        r.tex_image_2d(renderer::TEXTURE_2D, 0, renderer::RGBA as i32, w as i32, h as i32,
            renderer::RGBA, renderer::UNSIGNED_BYTE, cur_img.as_raw());
        let mut level = 1i32;
        let mut cur_w = w;
        let mut cur_h = h;
        while cur_w > 1 || cur_h > 1 {
            let new_w = (cur_w / 2).max(1);
            let new_h = (cur_h / 2).max(1);
            cur_img = image::imageops::resize(&cur_img, new_w, new_h, image::imageops::FilterType::Triangle);
            r.tex_image_2d(renderer::TEXTURE_2D, level, renderer::RGBA as i32, new_w as i32, new_h as i32,
                renderer::RGBA, renderer::UNSIGNED_BYTE, cur_img.as_raw());
            cur_w = new_w;
            cur_h = new_h;
            level += 1;
        }
    } else {
        r.tex_parameter_i(renderer::TEXTURE_2D, renderer::TEXTURE_MIN_FILTER, renderer::LINEAR as i32);
        let (format, data) = match img.color() {
            image::ColorType::Rgba8 => (renderer::RGBA, img.into_rgba8().into_raw()),
            image::ColorType::Rgb8 => (renderer::RGB, img.into_rgb8().into_raw()),
            _ => (renderer::RGBA, img.into_rgba8().into_raw()),
        };
        r.tex_image_2d(renderer::TEXTURE_2D, 0, format as i32, w as i32, h as i32,
            format, renderer::UNSIGNED_BYTE, &data);
    }
    tex
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_all_textures(r: &mut Renderer) -> Vec<u32> {
    let tex = |r: &mut Renderer, name: &str, ws: u32, wt: u32| {
        let p = asset_path("textures", name);
        if Path::new(&p).exists() { load_texture(r, &p, ws, wt) } else { 0 }
    };
    let tex_mip = |r: &mut Renderer, name: &str, ws: u32, wt: u32| {
        let p = asset_path("textures", name);
        if Path::new(&p).exists() { load_texture_mipmapped(r, &p, ws, wt) } else { 0 }
    };
    let rep = renderer::REPEAT;
    let clamp = renderer::CLAMP_TO_EDGE;

    let mut textures = vec![
        tex_mip(r, "gltron_floor.png", rep, rep),
        tex(r, "gltron_wall_1.png", rep, clamp),
        tex(r, "gltron_wall_2.png", rep, clamp),
        tex(r, "gltron_wall_3.png", rep, clamp),
        tex(r, "gltron_wall_4.png", rep, clamp),
        tex(r, "gltron_trail.png", clamp, clamp),
        tex(r, "gltron_traildecal.png", rep, clamp),
    ];
    for i in 0..6 {
        textures.push(tex(r, &format!("skybox{i}.png"), clamp, clamp));
    }
    textures.push(tex(r, "gltron_impact.png", rep, rep));
    textures.push(tex(r, "gltron_logo.png", clamp, clamp));
    textures.push(tex(r, "gltron.png", clamp, clamp));
    textures
}

#[cfg(target_arch = "wasm32")]
pub fn load_all_textures(r: &mut Renderer) -> Vec<u32> {
    use crate::embedded_assets::*;
    let rep = renderer::REPEAT;
    let clamp = renderer::CLAMP_TO_EDGE;
    let tex = |r: &mut Renderer, bytes: &[u8], ws: u32, wt: u32| load_texture_from_memory(r, bytes, ws, wt, false);
    let tex_mip = |r: &mut Renderer, bytes: &[u8], ws: u32, wt: u32| load_texture_from_memory(r, bytes, ws, wt, true);
    let mut textures = vec![
        tex_mip(r, FLOOR_PNG, rep, rep),
        tex(r, WALL_1_PNG, rep, clamp),
        tex(r, WALL_2_PNG, rep, clamp),
        tex(r, WALL_3_PNG, rep, clamp),
        tex(r, WALL_4_PNG, rep, clamp),
        tex(r, TRAIL_PNG, clamp, clamp),
        tex(r, TRAILDECAL_PNG, rep, clamp),
    ];
    let skyboxes = [SKYBOX0_PNG, SKYBOX1_PNG, SKYBOX2_PNG, SKYBOX3_PNG, SKYBOX4_PNG, SKYBOX5_PNG];
    for sb in &skyboxes {
        textures.push(tex(r, sb, clamp, clamp));
    }
    textures.push(tex(r, IMPACT_PNG, rep, rep));
    textures.push(tex(r, LOGO_PNG, clamp, clamp));
    textures.push(tex(r, GUI_PNG, clamp, clamp));
    textures
}

// === OBJ Model Loading ===

#[cfg(not(target_arch = "wasm32"))]
pub fn load_obj_model(r: &mut Renderer, name: &str) -> GlMesh {
    let path = asset_path("models", name);
    let (models, materials) = tobj::load_obj(
        &path,
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
    ).unwrap_or_else(|e| panic!("Failed to load model {path}: {e}"));

    let materials = materials.unwrap_or_default();
    build_gl_mesh(r, &models, &materials)
}

#[cfg(target_arch = "wasm32")]
fn load_obj_from_memory(r: &mut Renderer, obj_bytes: &[u8], mtl_bytes: &[u8]) -> GlMesh {
    use std::io::Cursor;
    let mtl_bytes = mtl_bytes.to_vec();
    let (models, materials) = tobj::load_obj_buf(
        &mut Cursor::new(obj_bytes),
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
        move |_mtl_path| {
            tobj::load_mtl_buf(&mut Cursor::new(&mtl_bytes))
        },
    ).unwrap_or_else(|e| panic!("Failed to load model from memory: {e}"));

    let materials = materials.unwrap_or_default();
    build_gl_mesh(r, &models, &materials)
}

fn build_gl_mesh(r: &mut Renderer, models: &[tobj::Model], materials: &[tobj::Material]) -> GlMesh {
    let mut groups: Vec<MaterialGroup> = Vec::new();
    let mut all_positions: Vec<f32> = Vec::new();
    let mut all_normals: Vec<f32> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();

    for model in models {
        let mesh = &model.mesh;
        let base_vertex = (all_positions.len() / 3) as u32;
        let start_index = all_indices.len() as i32;

        all_positions.extend_from_slice(&mesh.positions);
        if mesh.normals.len() == mesh.positions.len() {
            all_normals.extend_from_slice(&mesh.normals);
        } else {
            all_normals.extend((0..mesh.positions.len() / 3).flat_map(|_| [0.0f32, 1.0, 0.0]));
        }

        for &idx in &mesh.indices {
            all_indices.push(idx + base_vertex);
        }

        let mat = mesh.material_id.and_then(|id| materials.get(id));
        let (ambient, diffuse, specular, shininess) = mat
            .map(|m| (
                m.ambient.unwrap_or([0.2, 0.2, 0.2]),
                m.diffuse.unwrap_or([0.8, 0.8, 0.8]),
                m.specular.unwrap_or([0.0, 0.0, 0.0]),
                m.shininess.unwrap_or(0.0),
            ))
            .unwrap_or(([0.2; 3], [0.8; 3], [0.0; 3], 0.0));

        groups.push(MaterialGroup {
            start_index,
            num_indices: mesh.indices.len() as i32,
            ambient, diffuse, specular, shininess,
        });
    }

    // Create VBOs
    let vbo = r.gen_buffer();
    r.bind_buffer(renderer::ARRAY_BUFFER, vbo);
    let pos_bytes = unsafe {
        std::slice::from_raw_parts(all_positions.as_ptr() as *const u8, all_positions.len() * 4)
    };
    r.buffer_data(renderer::ARRAY_BUFFER, pos_bytes, renderer::STATIC_DRAW);

    let nbo = r.gen_buffer();
    r.bind_buffer(renderer::ARRAY_BUFFER, nbo);
    let norm_bytes = unsafe {
        std::slice::from_raw_parts(all_normals.as_ptr() as *const u8, all_normals.len() * 4)
    };
    r.buffer_data(renderer::ARRAY_BUFFER, norm_bytes, renderer::STATIC_DRAW);

    let ebo = r.gen_buffer();
    r.bind_buffer(renderer::ELEMENT_ARRAY_BUFFER, ebo);
    let idx_bytes = unsafe {
        std::slice::from_raw_parts(all_indices.as_ptr() as *const u8, all_indices.len() * 4)
    };
    r.buffer_data(renderer::ELEMENT_ARRAY_BUFFER, idx_bytes, renderer::STATIC_DRAW);

    // Create VAO with position (attrib 0) and normal (attrib 1) only
    let vao = r.gen_vertex_array();
    r.bind_vertex_array(vao);

    r.bind_buffer(renderer::ARRAY_BUFFER, vbo);
    r.enable_vertex_attrib(0);
    r.vertex_attrib_pointer_f32(0, 3, 0, 0);

    r.bind_buffer(renderer::ARRAY_BUFFER, nbo);
    r.enable_vertex_attrib(1);
    r.vertex_attrib_pointer_f32(1, 3, 0, 0);

    // Attribs 2 (texcoord) and 3 (color) are NOT enabled — they use default values
    r.bind_vertex_array(0);
    r.bind_buffer(renderer::ARRAY_BUFFER, 0);
    r.bind_buffer(renderer::ELEMENT_ARRAY_BUFFER, 0);

    GlMesh {
        vao, vbo, nbo, ebo,
        num_indices: all_indices.len() as i32,
        material_groups: groups,
        cpu_positions: all_positions,
        cpu_normals: all_normals,
        cpu_indices: all_indices,
    }
}

pub struct Models {
    pub cycle_high: GlMesh,
    pub cycle_med: GlMesh,
    pub cycle_low: GlMesh,
    pub recognizer: GlMesh,
    pub recognizer_quad: GlMesh,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_all_models(r: &mut Renderer) -> Models {
    Models {
        cycle_high: load_obj_model(r, "lightcycle-high.obj"),
        cycle_med: load_obj_model(r, "lightcycle-med.obj"),
        cycle_low: load_obj_model(r, "lightcycle-low.obj"),
        recognizer: load_obj_model(r, "recognizer.obj"),
        recognizer_quad: load_obj_model(r, "recognizer_quad.obj"),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_all_models(r: &mut Renderer) -> Models {
    use crate::embedded_assets::*;
    Models {
        cycle_high: load_obj_from_memory(r, CYCLE_HIGH_OBJ, LIGHTCYCLE_MTL),
        cycle_med: load_obj_from_memory(r, CYCLE_MED_OBJ, LIGHTCYCLE_MTL),
        cycle_low: load_obj_from_memory(r, CYCLE_LOW_OBJ, LIGHTCYCLE_MTL),
        recognizer: load_obj_from_memory(r, RECOGNIZER_OBJ, RECOGNIZER_MTL),
        recognizer_quad: load_obj_from_memory(r, RECOGNIZER_QUAD_OBJ, RECOGNIZER_MTL),
    }
}

// === Audio (using cpal) ===

use std::sync::{Arc, Mutex};

struct SoundInstance {
    data: Arc<Vec<i16>>,
    fpos: f64,
    volume: f32,
    looping: bool,
    rate_ratio: f64,
    pitch: Arc<Mutex<f32>>,
}

/// Mix all playing sound instances into an f32 output buffer (mono).
/// Output channels are interleaved; we write the same mono sample to all channels.
fn mix_audio(playing: &Arc<Mutex<Vec<SoundInstance>>>, out: &mut [f32], channels: usize) {
    for s in out.iter_mut() { *s = 0.0; }
    if let Ok(mut playing) = playing.try_lock() {
        playing.retain_mut(|inst| {
            let pitch = inst.pitch.try_lock().map(|p| *p).unwrap_or(1.0);
            let step = inst.rate_ratio * pitch as f64;
            let frames = out.len() / channels;
            for frame in 0..frames {
                let idx = inst.fpos as usize;
                if idx >= inst.data.len() {
                    if inst.looping {
                        inst.fpos -= inst.data.len() as f64;
                        // Retry this frame
                        let idx2 = inst.fpos as usize;
                        if idx2 >= inst.data.len() { return false; }
                        let sample = inst.data[idx2] as f32 / 32768.0 * inst.volume;
                        for ch in 0..channels {
                            out[frame * channels + ch] += sample;
                        }
                        inst.fpos += step;
                        continue;
                    } else {
                        return false;
                    }
                }
                let frac = (inst.fpos - idx as f64) as f32;
                let s0 = inst.data[idx] as f32 / 32768.0;
                let s1 = if idx + 1 < inst.data.len() {
                    inst.data[idx + 1] as f32 / 32768.0
                } else if inst.looping {
                    inst.data[0] as f32 / 32768.0
                } else {
                    s0
                };
                let sample = (s0 + frac * (s1 - s0)) * inst.volume;
                for ch in 0..channels {
                    out[frame * channels + ch] = (out[frame * channels + ch] + sample).clamp(-1.0, 1.0);
                }
                inst.fpos += step;
            }
            true
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_wav_i16(path: &str) -> Option<(Arc<Vec<i16>>, i32)> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let freq = spec.sample_rate as i32;
    let channels = spec.channels as usize;

    let samples: Vec<i16> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            reader.into_samples::<i32>().filter_map(|s| s.ok()).map(move |s| {
                if bits == 16 { s as i16 }
                else if bits == 24 { (s >> 8) as i16 }
                else if bits == 8 { ((s - 128) * 256) as i16 }
                else { (s >> (bits.saturating_sub(16))) as i16 }
            }).collect()
        }
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>().filter_map(|s| s.ok())
                .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect()
        }
    };

    let mono = if channels > 1 {
        samples.chunks(channels)
            .map(|chunk| (chunk.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16)
            .collect()
    } else {
        samples
    };
    Some((Arc::new(mono), freq))
}

#[cfg(not(target_arch = "wasm32"))]
fn load_ogg_i16(path: &str) -> Option<(Arc<Vec<i16>>, i32)> {
    use std::fs::File;
    let file = File::open(path).ok()?;
    let mut reader = lewton::inside_ogg::OggStreamReader::new(file).ok()?;
    let freq = reader.ident_hdr.audio_sample_rate as i32;
    let channels = reader.ident_hdr.audio_channels as usize;
    let mut all_samples: Vec<i16> = Vec::new();

    while let Some(packets) = reader.read_dec_packet_itl().ok()? {
        all_samples.extend_from_slice(&packets);
    }

    let mono = if channels > 1 {
        all_samples.chunks(channels)
            .map(|chunk| (chunk.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16)
            .collect()
    } else {
        all_samples
    };
    Some((Arc::new(mono), freq))
}

#[cfg(target_arch = "wasm32")]
fn load_wav_from_memory(bytes: &[u8]) -> Option<(Arc<Vec<i16>>, i32)> {
    use std::io::Cursor;
    let reader = hound::WavReader::new(Cursor::new(bytes)).ok()?;
    let spec = reader.spec();
    let freq = spec.sample_rate as i32;
    let channels = spec.channels as usize;
    let samples: Vec<i16> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            reader.into_samples::<i32>().filter_map(|s| s.ok()).map(move |s| {
                if bits == 16 { s as i16 }
                else if bits == 24 { (s >> 8) as i16 }
                else if bits == 8 { ((s - 128) * 256) as i16 }
                else { (s >> (bits.saturating_sub(16))) as i16 }
            }).collect()
        }
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>().filter_map(|s| s.ok())
                .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect()
        }
    };
    let mono = if channels > 1 {
        samples.chunks(channels)
            .map(|chunk| (chunk.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16)
            .collect()
    } else {
        samples
    };
    Some((Arc::new(mono), freq))
}

#[cfg(target_arch = "wasm32")]
fn load_ogg_from_memory(bytes: &[u8]) -> Option<(Arc<Vec<i16>>, i32)> {
    use std::io::Cursor;
    let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes)).ok()?;
    let freq = reader.ident_hdr.audio_sample_rate as i32;
    let channels = reader.ident_hdr.audio_channels as usize;
    let mut all_samples: Vec<i16> = Vec::new();
    while let Some(packets) = reader.read_dec_packet_itl().ok()? {
        all_samples.extend_from_slice(&packets);
    }
    let mono = if channels > 1 {
        all_samples.chunks(channels)
            .map(|chunk| (chunk.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16)
            .collect()
    } else {
        all_samples
    };
    Some((Arc::new(mono), freq))
}

pub struct Audio {
    _stream: cpal::Stream,
    playing: Arc<Mutex<Vec<SoundInstance>>>,
    crash_sound: Option<Arc<Vec<i16>>>,
    crash_rate_ratio: f64,
    engine_sound: Option<Arc<Vec<i16>>>,
    engine_rate_ratio: f64,
    engine_pitch: Arc<Mutex<f32>>,
    fx_volume: std::cell::Cell<f32>,
}

impl Audio {
    pub fn new() -> Option<Self> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let default_config = device.default_output_config().ok()?;
        let sample_rate = default_config.sample_rate().0 as i32;
        let channels = default_config.channels() as usize;
        eprintln!("Audio: device config: {}Hz, {} channels, {:?}", sample_rate, channels, default_config.sample_format());

        let config = cpal::StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(sample_rate as u32),
            buffer_size: cpal::BufferSize::Default,
        };

        let playing = Arc::new(Mutex::new(Vec::new()));
        let playing_cb = playing.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                mix_audio(&playing_cb, data, channels);
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        ).ok()?;
        stream.play().ok()?;

        let device_rate = sample_rate;
        #[cfg(not(target_arch = "wasm32"))]
        let (crash_sound, crash_rate) = match load_wav_i16(&asset_path("sounds", "game_crash.wav")) {
            Some((data, rate)) => (Some(data), rate),
            None => (None, device_rate),
        };
        #[cfg(not(target_arch = "wasm32"))]
        let (engine_sound, engine_rate) = match load_wav_i16(&asset_path("sounds", "game_engine.wav")) {
            Some((data, rate)) => (Some(data), rate),
            None => (None, device_rate),
        };
        #[cfg(target_arch = "wasm32")]
        let (crash_sound, crash_rate) = match load_wav_from_memory(embedded_assets::CRASH_WAV) {
            Some((data, rate)) => (Some(data), rate),
            None => (None, device_rate),
        };
        #[cfg(target_arch = "wasm32")]
        let (engine_sound, engine_rate) = match load_wav_from_memory(embedded_assets::ENGINE_WAV) {
            Some((data, rate)) => (Some(data), rate),
            None => (None, device_rate),
        };
        if crash_sound.is_some() { eprintln!("Audio: loaded crash sound ({}Hz)", crash_rate); }
        if engine_sound.is_some() { eprintln!("Audio: loaded engine sound ({}Hz)", engine_rate); }
        let engine_pitch = Arc::new(Mutex::new(1.0f32));

        Some(Self {
            _stream: stream, playing,
            crash_sound, crash_rate_ratio: crash_rate as f64 / device_rate as f64,
            engine_sound, engine_rate_ratio: engine_rate as f64 / device_rate as f64,
            engine_pitch, fx_volume: std::cell::Cell::new(1.0),
        })
    }

    pub fn play_crash(&self) {
        if let Some(ref data) = self.crash_sound {
            let vol = 0.5 * self.fx_volume.get();
            if vol > 0.0 {
                if let Ok(mut playing) = self.playing.lock() {
                    playing.push(SoundInstance {
                        data: data.clone(), fpos: 0.0, volume: vol,
                        looping: false, rate_ratio: self.crash_rate_ratio,
                        pitch: Arc::new(Mutex::new(1.0)),
                    });
                }
            }
        }
    }

    pub fn start_engine(&self) {
        if let Some(ref data) = self.engine_sound {
            let vol = 0.25 * self.fx_volume.get();
            if let Ok(mut playing) = self.playing.lock() {
                if !playing.iter().any(|s| s.looping) {
                    playing.push(SoundInstance {
                        data: data.clone(), fpos: 0.0, volume: vol,
                        looping: true, rate_ratio: self.engine_rate_ratio,
                        pitch: self.engine_pitch.clone(),
                    });
                }
            }
        }
    }

    pub fn stop_engine(&self) {
        if let Ok(mut playing) = self.playing.lock() {
            playing.retain(|s| !s.looping);
        }
    }

    pub fn set_engine_pitch(&self, speed_mult: f32) {
        let pitch = 0.8 + (speed_mult - 1.0).max(0.0) * 0.5;
        if let Ok(mut p) = self.engine_pitch.lock() { *p = pitch; }
    }

    pub fn set_fx_volume(&self, volume: f32) {
        self.fx_volume.set(volume);
        if let Ok(mut playing) = self.playing.lock() {
            for inst in playing.iter_mut() {
                if inst.looping { inst.volume = 0.25 * volume; }
                else { inst.volume = 0.5 * volume; }
            }
        }
    }
}

// === Music (OGG via lewton, played through same mixer) ===

pub struct Music {
    playing: Arc<Mutex<Vec<SoundInstance>>>,
    _stream: cpal::Stream,
    music_volume: Arc<Mutex<f32>>,
    paused: Arc<Mutex<bool>>,
}

impl Music {
    pub fn new() -> Option<Self> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        #[cfg(not(target_arch = "wasm32"))]
        let (music_data, music_rate) = {
            let music_path = asset_path("music", "song_revenge_of_cats.ogg");
            if Path::new(&music_path).exists() {
                match load_ogg_i16(&music_path) {
                    Some((data, rate)) => {
                        eprintln!("Music: loaded {music_path} ({rate}Hz, {} samples)", data.len());
                        (data, rate)
                    }
                    None => {
                        eprintln!("Music: failed to decode {music_path}");
                        return None;
                    }
                }
            } else {
                eprintln!("Music: file not found: {music_path}");
                return None;
            }
        };
        #[cfg(target_arch = "wasm32")]
        let (music_data, music_rate) = match load_ogg_from_memory(embedded_assets::MUSIC_OGG) {
            Some((data, rate)) => {
                log::info!("Music: loaded from embedded OGG ({rate}Hz, {} samples)", data.len());
                (data, rate)
            }
            None => {
                log::error!("Music: failed to decode embedded OGG");
                return None;
            }
        };

        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let default_config = device.default_output_config().ok()?;
        let sample_rate = default_config.sample_rate().0 as i32;
        let channels = default_config.channels() as usize;

        let config = cpal::StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(sample_rate as u32),
            buffer_size: cpal::BufferSize::Default,
        };

        let playing = Arc::new(Mutex::new(Vec::new()));
        let playing_cb = playing.clone();
        let music_volume = Arc::new(Mutex::new(0.5f32));
        let paused = Arc::new(Mutex::new(false));

        // Add music as a looping sound instance
        {
            let rate_ratio = music_rate as f64 / sample_rate as f64;
            let mut p = playing.lock().unwrap();
            p.push(SoundInstance {
                data: music_data,
                fpos: 0.0,
                volume: 0.5,
                looping: true,
                rate_ratio,
                pitch: Arc::new(Mutex::new(1.0)),
            });
        }

        let vol_ref = music_volume.clone();
        let paused_ref = paused.clone();
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let is_paused = paused_ref.try_lock().map(|p| *p).unwrap_or(false);
                if is_paused {
                    for s in data.iter_mut() { *s = 0.0; }
                    return;
                }
                mix_audio(&playing_cb, data, channels);
                // Apply music volume
                let vol = vol_ref.try_lock().map(|v| *v).unwrap_or(0.5);
                for s in data.iter_mut() {
                    *s *= vol;
                }
            },
            |err| eprintln!("Music stream error: {err}"),
            None,
        ).ok()?;
        stream.play().ok()?;

        Some(Self { playing, _stream: stream, music_volume, paused })
    }

    pub fn set_paused(&self, p: bool) {
        if let Ok(mut paused) = self.paused.lock() { *paused = p; }
    }

    pub fn set_volume(&self, volume: f32) {
        if let Ok(mut v) = self.music_volume.lock() { *v = volume; }
    }
}

// === GL Mesh Drawing (VAO-based for GL 3.3 Core) ===

pub fn draw_gl_mesh(r: &mut Renderer, mesh: &GlMesh, player_color: Option<&[f32; 4]>) {
    r.set_color_material(false);

    // Set default vertex color to white (attrib 3 is not enabled in mesh VAO)
    r.vertex_attrib_4f(3, 1.0, 1.0, 1.0, 1.0);

    r.bind_model_vao(mesh.vao, mesh.ebo);

    for group in &mesh.material_groups {
        let mut amb = [group.ambient[0], group.ambient[1], group.ambient[2], 1.0];
        let mut diff = [group.diffuse[0], group.diffuse[1], group.diffuse[2], 1.0];
        let spec = [group.specular[0], group.specular[1], group.specular[2], 1.0];

        if let Some(pc) = player_color {
            amb = [amb[0] * pc[0], amb[1] * pc[1], amb[2] * pc[2], 1.0];
            diff = [diff[0] * pc[0], diff[1] * pc[1], diff[2] * pc[2], 1.0];
        }

        r.set_material(&Material {
            ambient: amb,
            diffuse: diff,
            specular: spec,
            shininess: group.shininess.min(128.0),
        });

        r.sync_uniforms();
        r.draw_elements_u32(group.num_indices, group.start_index);
    }

    r.unbind_model_vao();
    r.set_color_material(true);
}

/// Draw mesh as flat colored (for shadows).
pub fn draw_gl_mesh_flat(r: &mut Renderer, mesh: &GlMesh, color: [f32; 4]) {
    r.vertex_attrib_4f(3, color[0], color[1], color[2], color[3]);

    r.bind_model_vao(mesh.vao, mesh.ebo);
    r.sync_uniforms();
    r.draw_elements_u32(mesh.num_indices, 0);
    r.unbind_model_vao();
}

// === Bitmap Font ===

pub struct BitmapFont {
    pub tex_ids: [u32; 2],
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_bitmap_font(r: &mut Renderer) -> BitmapFont {
    let clamp = renderer::CLAMP_TO_EDGE;
    let t0 = {
        let p = format!("{ASSET_DIR}/fonts/babbage.0.png");
        if Path::new(&p).exists() { load_texture(r, &p, clamp, clamp) } else { 0 }
    };
    let t1 = {
        let p = format!("{ASSET_DIR}/fonts/babbage.1.png");
        if Path::new(&p).exists() { load_texture(r, &p, clamp, clamp) } else { 0 }
    };
    BitmapFont { tex_ids: [t0, t1] }
}

#[cfg(target_arch = "wasm32")]
pub fn load_bitmap_font(r: &mut Renderer) -> BitmapFont {
    let clamp = renderer::CLAMP_TO_EDGE;
    let t0 = load_texture_from_memory(r, embedded_assets::FONT0_PNG, clamp, clamp, false);
    let t1 = load_texture_from_memory(r, embedded_assets::FONT1_PNG, clamp, clamp, false);
    BitmapFont { tex_ids: [t0, t1] }
}

/// Batched text rendering: groups characters by texture page, 1-2 draw calls.
pub fn draw_text(r: &mut Renderer, font: &BitmapFont, x: f32, y: f32, size: f32, text: &str, color: [f32; 4]) {
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    let mut batch: Vec<Vertex> = Vec::new();
    let mut cur_tex: i32 = -1;
    let mut cx = x;

    for ch in text.chars() {
        let code = ch as u32;
        if code < 32 || code > 126 { cx += size * 0.5; continue; }
        let idx = (code - 32 + 1) as usize;
        let tex_idx = idx / 64;
        let local = idx % 64;
        let col = local % 8;
        let row = local / 8;

        if tex_idx as i32 != cur_tex {
            if !batch.is_empty() {
                r.draw_triangles(&batch);
                batch.clear();
            }
            r.set_texture(Some(font.tex_ids[tex_idx]));
            cur_tex = tex_idx as i32;
        }

        let u0 = col as f32 / 8.0;
        let u1 = (col + 1) as f32 / 8.0;
        let v_bottom = 1.0 - (row + 1) as f32 / 8.0;
        let v_top = 1.0 - row as f32 / 8.0;

        let verts = textured_quad_verts(
            [[cx, y, 0.0], [cx + size, y, 0.0], [cx + size, y + size, 0.0], [cx, y + size, 0.0]],
            [[u0, v_bottom], [u1, v_bottom], [u1, v_top], [u0, v_top]],
            color,
        );
        batch.extend_from_slice(&verts);
        cx += size * 0.72;
    }

    if !batch.is_empty() {
        r.draw_triangles(&batch);
    }

    r.set_texture(None);
    r.disable_blend();
}

pub fn draw_text_shadowed(r: &mut Renderer, font: &BitmapFont, x: f32, y: f32, size: f32, text: &str, color: [f32; 4]) {
    let offset = (size * 0.06).max(1.0);
    draw_text(r, font, x + offset, y - offset, size, text, [0.0, 0.0, 0.0, color[3] * 0.7]);
    draw_text(r, font, x, y, size, text, color);
}
