//! Embedded asset data for WASM builds.
//! On wasm32, assets are compiled into the binary via include_bytes!/include_str!.

// Textures
pub const FLOOR_PNG: &[u8] = include_bytes!("../assets/textures/gltron_floor.png");
pub const WALL_1_PNG: &[u8] = include_bytes!("../assets/textures/gltron_wall_1.png");
pub const WALL_2_PNG: &[u8] = include_bytes!("../assets/textures/gltron_wall_2.png");
pub const WALL_3_PNG: &[u8] = include_bytes!("../assets/textures/gltron_wall_3.png");
pub const WALL_4_PNG: &[u8] = include_bytes!("../assets/textures/gltron_wall_4.png");
pub const TRAIL_PNG: &[u8] = include_bytes!("../assets/textures/gltron_trail.png");
pub const TRAILDECAL_PNG: &[u8] = include_bytes!("../assets/textures/gltron_traildecal.png");
pub const SKYBOX0_PNG: &[u8] = include_bytes!("../assets/textures/skybox0.png");
pub const SKYBOX1_PNG: &[u8] = include_bytes!("../assets/textures/skybox1.png");
pub const SKYBOX2_PNG: &[u8] = include_bytes!("../assets/textures/skybox2.png");
pub const SKYBOX3_PNG: &[u8] = include_bytes!("../assets/textures/skybox3.png");
pub const SKYBOX4_PNG: &[u8] = include_bytes!("../assets/textures/skybox4.png");
pub const SKYBOX5_PNG: &[u8] = include_bytes!("../assets/textures/skybox5.png");
pub const IMPACT_PNG: &[u8] = include_bytes!("../assets/textures/gltron_impact.png");
pub const LOGO_PNG: &[u8] = include_bytes!("../assets/textures/gltron_logo.png");
pub const GUI_PNG: &[u8] = include_bytes!("../assets/textures/gltron.png");

// Fonts
pub const FONT0_PNG: &[u8] = include_bytes!("../assets/fonts/babbage.0.png");
pub const FONT1_PNG: &[u8] = include_bytes!("../assets/fonts/babbage.1.png");

// Models (OBJ + MTL)
pub const CYCLE_HIGH_OBJ: &[u8] = include_bytes!("../assets/models/lightcycle-high.obj");
pub const CYCLE_MED_OBJ: &[u8] = include_bytes!("../assets/models/lightcycle-med.obj");
pub const CYCLE_LOW_OBJ: &[u8] = include_bytes!("../assets/models/lightcycle-low.obj");
pub const RECOGNIZER_OBJ: &[u8] = include_bytes!("../assets/models/recognizer.obj");
pub const RECOGNIZER_QUAD_OBJ: &[u8] = include_bytes!("../assets/models/recognizer_quad.obj");
pub const LIGHTCYCLE_MTL: &[u8] = include_bytes!("../assets/models/lightcycle.mtl");
pub const RECOGNIZER_MTL: &[u8] = include_bytes!("../assets/models/recognizer.mtl");

// Sounds (WAV)
pub const CRASH_WAV: &[u8] = include_bytes!("../assets/sounds/game_crash.wav");
pub const ENGINE_WAV: &[u8] = include_bytes!("../assets/sounds/game_engine.wav");
pub const RECOGNIZER_WAV: &[u8] = include_bytes!("../assets/sounds/game_recognizer.wav");

// Music (OGG)
pub const MUSIC_OGG: &[u8] = include_bytes!("../assets/music/song_revenge_of_cats.ogg");

// Text data
pub const CREDITS_TXT: &str = include_str!("../assets/credits.txt");
