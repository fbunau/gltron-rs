use glam::{Vec2, Vec3};

pub const BUILD_VERSION: &str = env!("BUILD_VERSION");

// === Constants matching original GLTron ===

pub const MAX_PLAYERS: usize = 10;
pub const MAX_TRAIL: usize = 1000;
pub const PHYSICS_RATE: u32 = 20;
pub const TRAIL_HEIGHT: f32 = 3.5;
pub const TRAIL_FADE_DURATION: f32 = 3000.0; // ms for trail death animation
pub const WALL_H: f32 = 48.0;
pub const CYCLE_HEIGHT: f32 = 2.05; // model half-height (original uses BBox.vSize.z / 2)
pub const RECOGNIZER_HEIGHT: f32 = 50.0;
pub const SPEED_OZ_FREQ: f32 = 1200.0;
pub const SPEED_OZ_FACTOR: f32 = 0.09;
pub const TURN_LENGTH: u32 = 200;
pub const FAST_FINISH_FACTOR: f32 = 40.0;
pub const DECAL_WIDTH: f32 = 20.0;
pub const BOW_LENGTH: f32 = 6.0;
pub const BOW_DIST1: f32 = 0.4;
pub const BOW_DIST2: f32 = 0.85;
pub const BOW_DIST3: f32 = 2.0;

// Camera constants
pub const CAM_CIRCLE_DIST: f32 = 17.0;
pub const CAM_CIRCLE_Z: f32 = 8.0;
pub const CAM_FOLLOW_DIST: f32 = 18.0;
pub const CAM_FOLLOW_Z: f32 = 6.0;
pub const CAM_FOLLOW_SPEED: f32 = 0.05;
pub const CAM_COCKPIT_Z: f32 = 4.0;
pub const CAM_SPEED: f32 = 0.000349;
pub const CAM_R_MIN: f32 = 6.0;
pub const CAM_R_MAX: f32 = 45.0;
pub const CAM_CHI_MIN: f32 = std::f32::consts::PI / 8.0;
pub const CAM_CHI_MAX: f32 = 3.0 * std::f32::consts::PI / 8.0;
pub const CAM_DR: f32 = 6.4;

// Explosion constants
pub const EXP_RADIUS_MAX: f32 = 30.0;
pub const EXP_RADIUS_DELTA: f32 = 0.0025;
pub const IMPACT_RADIUS_DELTA: f32 = 0.006;
pub const IMPACT_MAX_RADIUS: f32 = 25.0;
pub const SHOCKWAVE_MAX_RADIUS: f32 = 45.0;
pub const SHOCKWAVE_WIDTH: f32 = 0.2;
pub const SHOCKWAVE_SPACING: f32 = 6.0;
pub const SHOCKWAVE_SPEED: f32 = 1.2;
pub const SHOCKWAVE_SEGMENTS: usize = 25;
pub const NUM_SHOCKWAVES: usize = 3;
pub const DEATH_CAMERA_DELAY: u32 = 4000; // ms to linger on death scene before spectating
pub const NUM_SPIRES: usize = 21;
pub const SPIRE_MAX_RADIUS: f32 = 25.0;
pub const SPIRE_WIDTH: f32 = 0.40;

// Neigung (tilt angle for turns in degrees)
pub const NEIGUNG: f32 = 25.0;

// Direction vectors: Up=0, Right=1, Down=2, Left=3
pub const DIRS_X: [f32; 4] = [0.0, -1.0, 0.0, 1.0];
pub const DIRS_Y: [f32; 4] = [-1.0, 0.0, 1.0, 0.0];

// Shadow matrix (light direction LX=2, LY=2)
pub const SHADOW_MATRIX: [f32; 16] = [
    4.0, 0.0, 0.0, 0.0,
    0.0, 4.0, 0.0, 0.0,
    -2.0, -2.0, 0.0, 0.0,
    0.0, 0.0, 0.0, 4.0,
];

// Default player colors (10 players max)
pub const MODEL_DIFFUSE: [[f32; 4]; MAX_PLAYERS] = [
    [1.0, 0.55, 0.14, 1.0],   // Orange
    [0.75, 0.02, 0.02, 1.0],  // Red
    [0.12, 0.52, 0.60, 1.0],  // Cyan
    [0.80, 0.80, 0.80, 1.0],  // Gray
    [0.55, 0.15, 0.80, 1.0],  // Purple
    [0.90, 0.75, 0.10, 1.0],  // Gold
    [0.15, 0.75, 0.20, 1.0],  // Green
    [0.90, 0.20, 0.60, 1.0],  // Magenta
    [0.10, 0.70, 0.70, 1.0],  // Teal
    [0.65, 0.40, 0.15, 1.0],  // Brown
];
pub const MODEL_SPECULAR: [[f32; 4]; MAX_PLAYERS] = [
    [0.50, 0.50, 0.00, 1.0],
    [0.75, 0.02, 0.02, 1.0],
    [0.12, 0.52, 0.60, 1.0],
    [1.00, 1.00, 1.00, 1.0],
    [0.60, 0.30, 0.90, 1.0],
    [1.00, 0.85, 0.20, 1.0],
    [0.30, 0.85, 0.40, 1.0],
    [1.00, 0.40, 0.70, 1.0],
    [0.20, 0.80, 0.80, 1.0],
    [0.75, 0.50, 0.25, 1.0],
];
pub const TRAIL_DIFFUSE: [[f32; 4]; MAX_PLAYERS] = [
    [1.0, 0.55, 0.14, 0.85],   // Orange
    [0.75, 0.02, 0.02, 0.85],  // Red
    [0.12, 0.52, 0.60, 0.85],  // Cyan
    [0.80, 0.80, 0.80, 0.85],  // Gray
    [0.55, 0.15, 0.80, 0.85],  // Purple
    [0.90, 0.75, 0.10, 0.85],  // Gold
    [0.15, 0.75, 0.20, 0.85],  // Green
    [0.90, 0.20, 0.60, 0.85],  // Magenta
    [0.10, 0.70, 0.70, 0.85],  // Teal
    [0.65, 0.40, 0.15, 0.85],  // Brown
];

// === Direction ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
}

impl Dir {
    pub fn turn_left(self) -> Self {
        match self {
            Dir::Up => Dir::Left,
            Dir::Left => Dir::Down,
            Dir::Down => Dir::Right,
            Dir::Right => Dir::Up,
        }
    }

    pub fn turn_right(self) -> Self {
        match self {
            Dir::Up => Dir::Right,
            Dir::Right => Dir::Down,
            Dir::Down => Dir::Left,
            Dir::Left => Dir::Up,
        }
    }

    pub fn vec(self) -> Vec2 {
        Vec2::new(DIRS_X[self as usize], DIRS_Y[self as usize])
    }

    pub fn angle(self) -> f32 {
        let v = self.vec();
        v.y.atan2(v.x)
    }

    pub fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => Dir::Up,
            1 => Dir::Right,
            2 => Dir::Down,
            _ => Dir::Left,
        }
    }
}

// === Segment ===

#[derive(Debug, Clone, Copy)]
pub struct Segment2 {
    pub start: Vec2,
    pub direction: Vec2,
}

impl Segment2 {
    pub fn new(start: Vec2, direction: Vec2) -> Self {
        Self { start, direction }
    }

    pub fn end(&self) -> Vec2 {
        self.start + self.direction
    }

    pub fn length(&self) -> f32 {
        self.direction.length()
    }

    /// Returns (intersection_point, t1, t2) where t1 is parameter on self, t2 on other.
    /// Intersection occurs if both t1 and t2 are in [0, 1].
    pub fn intersect(&self, other: &Segment2) -> Option<(Vec2, f32, f32)> {
        let d = self.direction;
        let e = other.direction;
        let denom = d.x * e.y - d.y * e.x;
        if denom.abs() < 1e-10 {
            return None;
        }
        let f = other.start - self.start;
        let t1 = (f.x * e.y - f.y * e.x) / denom;
        let t2 = (f.x * d.y - f.y * d.x) / denom;
        Some((self.start + d * t1, t1, t2))
    }
}

// === Game Event ===

#[derive(Debug, Clone, Copy)]
pub enum GameEvent {
    TurnLeft { player: usize, x: f32, y: f32, timestamp: u32 },
    TurnRight { player: usize, x: f32, y: f32, timestamp: u32 },
    Crash { player: usize, killed_by: Option<usize>, x: f32, y: f32, timestamp: u32 },
    Stop { timestamp: u32 },
}

// === Camera ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraKind {
    Circling,
    Follow,
    Cockpit,
    Free,
}

impl CameraKind {
    pub fn next(self) -> Self {
        match self {
            CameraKind::Circling => CameraKind::Follow,
            CameraKind::Follow => CameraKind::Cockpit,
            CameraKind::Cockpit => CameraKind::Free,
            CameraKind::Free => CameraKind::Circling,
        }
    }

    pub fn defaults(self) -> (f32, f32, f32) {
        match self {
            CameraKind::Circling => (CAM_CIRCLE_DIST, std::f32::consts::PI / 3.0, 0.0),
            CameraKind::Follow => (CAM_FOLLOW_DIST, std::f32::consts::PI / 4.0, std::f32::consts::PI / 72.0),
            CameraKind::Cockpit => (CAM_COCKPIT_Z, std::f32::consts::PI / 8.0, 0.0),
            CameraKind::Free => (CAM_CIRCLE_DIST, std::f32::consts::PI / 3.0, 0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraFreedom {
    pub r: bool,
    pub phi: bool,
    pub chi: bool,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub pos: Vec3,
    pub target: Vec3,
    pub r: f32,
    pub chi: f32,
    pub phi: f32,
    pub phi_offset: f32,
    pub kind: CameraKind,
    pub interpolated_cam: bool,
    pub interpolated_target: bool,
    pub coupled: bool,
    pub freedom: CameraFreedom,
}

// === AI ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiKind {
    Human,
    Computer,
    None,
}

#[derive(Debug, Clone)]
pub struct AiDistances {
    pub front: f32,
    pub left: f32,
    pub right: f32,
    pub back_left: f32,
}

impl Default for AiDistances {
    fn default() -> Self {
        Self { front: f32::MAX, left: f32::MAX, right: f32::MAX, back_left: f32::MAX }
    }
}

#[derive(Debug, Clone)]
pub struct AiState {
    pub kind: AiKind,
    pub tdiff: i32,
    pub last_time: u32,
    pub distances: AiDistances,
}

pub struct AiParams {
    pub min_turn_time: [u32; 4],       // ms between turns
    pub max_seg_frac: [f32; 4],        // fraction of grid_size for max segment
    pub critical_frac: [f32; 4],       // fraction of grid_size for danger distance
    pub spiral: [i32; 4],              // max consecutive same-direction turns
    pub rl_delta: [f32; 4],            // left/right preference bias
}

pub const AI_PARAMS: AiParams = AiParams {
    min_turn_time: [600, 400, 200, 100],
    max_seg_frac: [0.6, 0.55, 0.50, 0.45],
    critical_frac: [0.20, 0.08037, 0.08037, 0.08037],
    spiral: [6, 6, 6, 6],
    rl_delta: [0.0, 10.0, 20.0, 30.0],
};

// === Player ===

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub dir: Dir,
    pub last_dir: Dir,
    pub speed: f32,
    pub kills: i32,
    pub deaths: i32,
    pub wins: i32,
    pub booster: f32,
    pub boost_enabled: bool,
    pub wall_accel_active: bool,
    pub speed_mult: f32,
    pub trail_height: f32,
    pub trail_fade: f32,  // >0 means fading out (ms remaining)
    pub turn_time: u32,
    pub trails: Vec<Segment2>,
    pub trail_offset: usize,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub data: PlayerData,
    pub ai: AiState,
    pub camera: Camera,
    pub show_scoreboard: bool,
    /// Spectator target after death: None = recognizer, Some(idx) = follow player.
    pub spectate_target: Option<usize>,
    /// Game time when this player died (for spectator camera delay).
    pub death_time: Option<u32>,
}

// === Visual ===

#[derive(Debug, Clone)]
pub struct ShatterParticle {
    pub local_verts: [Vec3; 3], // 3 vertex positions relative to centroid (actual model triangle)
    pub color: [f32; 4],       // material color for this fragment
    pub pos: Vec3,              // centroid world position
    pub vel: Vec3,
    pub angle: f32,
    pub angular_vel: f32,
    pub axis: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

#[derive(Debug, Clone)]
pub struct PlayerVisual {
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub trail_color: [f32; 4],
    pub impact_radius: f32,
    pub exp_radius: f32,
    pub crash_pos: Option<Vec2>,
    pub crash_dir: Option<Dir>,
    pub crash_angle: f32,
    pub shatter_particles: Vec<ShatterParticle>,
}

// === Advanced Config (all tunable gameplay parameters) ===

#[derive(Debug, Clone)]
pub struct AdvancedConfig {
    // Boost
    pub boost_accel_rate: f32,    // speed_mult units/sec while boosting
    pub boost_decel_rate: f32,    // speed_mult units/sec when not boosting
    pub max_speed_mult: f32,      // maximum speed multiplier
    pub boost_use_rate: f32,      // boost energy drain per second (base zone)
    pub boost_regen_rate: f32,    // boost energy regen per second
    pub boost_max: f32,           // max boost energy
    pub zone_yellow_mult: f32,    // speed_mult threshold for yellow zone
    pub zone_red_mult: f32,       // speed_mult threshold for red zone
    pub zone_yellow_drain: f32,   // drain multiplier in yellow zone
    pub zone_red_drain: f32,      // drain multiplier in red zone

    // Wall acceleration
    pub wall_accel_limit: f32,    // distance to trigger wall accel
    pub wall_accel_use: f32,      // acceleration rate near walls
    pub wall_accel_decrease: f32, // deceleration rate away from walls

    // Trail
    pub trail_height: f32,        // height of trail walls
    pub trail_fade_duration: f32, // ms for trail death animation

    // Speed oscillation
    pub speed_oz_factor: f32,     // oscillation amplitude (0 = none)
    pub speed_oz_freq: f32,       // oscillation period in ms
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            boost_accel_rate: 0.6,
            boost_decel_rate: 0.5,
            max_speed_mult: 3.0,
            boost_use_rate: 1.375,
            boost_regen_rate: 0.35,
            boost_max: 6.5,
            zone_yellow_mult: 1.7,
            zone_red_mult: 2.5,
            zone_yellow_drain: 2.0,
            zone_red_drain: 4.0,
            wall_accel_limit: 80.0,
            wall_accel_use: 0.15,
            wall_accel_decrease: 0.2,
            trail_height: 3.5,
            trail_fade_duration: 3000.0,
            speed_oz_factor: 0.09,
            speed_oz_freq: 1200.0,
        }
    }
}

/// Field descriptors for serialization and menu building.
pub struct AdvancedField {
    pub name: &'static str,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub decimals: usize,
}

pub const ADVANCED_FIELDS: &[AdvancedField] = &[
    AdvancedField { name: "boost_accel_rate",    label: "boost accel",       min: 0.1,  max: 5.0,    step: 0.1,  decimals: 1 },
    AdvancedField { name: "boost_decel_rate",    label: "boost decel",       min: 0.01, max: 2.0,    step: 0.01, decimals: 2 },
    AdvancedField { name: "max_speed_mult",      label: "max speed mult",    min: 1.5,  max: 5.0,    step: 0.1,  decimals: 1 },
    AdvancedField { name: "boost_use_rate",      label: "boost drain rate",  min: 0.1,  max: 5.0,    step: 0.05, decimals: 2 },
    AdvancedField { name: "boost_regen_rate",    label: "boost regen rate",  min: 0.1,  max: 3.0,    step: 0.05, decimals: 2 },
    AdvancedField { name: "boost_max",           label: "boost capacity",    min: 1.0,  max: 20.0,   step: 0.5,  decimals: 1 },
    AdvancedField { name: "zone_yellow_mult",    label: "yellow zone at",    min: 1.1,  max: 3.0,    step: 0.05, decimals: 2 },
    AdvancedField { name: "zone_red_mult",       label: "red zone at",       min: 1.2,  max: 4.0,    step: 0.05, decimals: 2 },
    AdvancedField { name: "zone_yellow_drain",   label: "yellow drain mult", min: 1.0,  max: 8.0,    step: 0.5,  decimals: 1 },
    AdvancedField { name: "zone_red_drain",      label: "red drain mult",    min: 1.0,  max: 16.0,   step: 0.5,  decimals: 1 },
    AdvancedField { name: "wall_accel_limit",    label: "wall detect dist",  min: 5.0,  max: 100.0,  step: 1.0,  decimals: 0 },
    AdvancedField { name: "wall_accel_use",      label: "wall accel rate",   min: 0.1,  max: 5.0,    step: 0.1,  decimals: 1 },
    AdvancedField { name: "wall_accel_decrease",  label: "wall decel rate",   min: 0.05, max: 3.0,    step: 0.05, decimals: 2 },
    AdvancedField { name: "trail_height",        label: "trail height",      min: 1.0,  max: 10.0,   step: 0.5,  decimals: 1 },
    AdvancedField { name: "trail_fade_duration", label: "trail fade (ms)",   min: 500.0, max: 10000.0, step: 250.0, decimals: 0 },
    AdvancedField { name: "speed_oz_factor",     label: "speed wobble",      min: 0.0,  max: 0.3,    step: 0.01, decimals: 2 },
    AdvancedField { name: "speed_oz_freq",       label: "wobble freq (ms)",  min: 200.0, max: 5000.0, step: 100.0, decimals: 0 },
];

impl AdvancedConfig {
    pub fn get_field(&self, idx: usize) -> f32 {
        match idx {
            0  => self.boost_accel_rate,
            1  => self.boost_decel_rate,
            2  => self.max_speed_mult,
            3  => self.boost_use_rate,
            4  => self.boost_regen_rate,
            5  => self.boost_max,
            6  => self.zone_yellow_mult,
            7  => self.zone_red_mult,
            8  => self.zone_yellow_drain,
            9  => self.zone_red_drain,
            10 => self.wall_accel_limit,
            11 => self.wall_accel_use,
            12 => self.wall_accel_decrease,
            13 => self.trail_height,
            14 => self.trail_fade_duration,
            15 => self.speed_oz_factor,
            16 => self.speed_oz_freq,
            _  => 0.0,
        }
    }

    pub fn set_field(&mut self, idx: usize, val: f32) {
        match idx {
            0  => self.boost_accel_rate = val,
            1  => self.boost_decel_rate = val,
            2  => self.max_speed_mult = val,
            3  => self.boost_use_rate = val,
            4  => self.boost_regen_rate = val,
            5  => self.boost_max = val,
            6  => self.zone_yellow_mult = val,
            7  => self.zone_red_mult = val,
            8  => self.zone_yellow_drain = val,
            9  => self.zone_red_drain = val,
            10 => self.wall_accel_limit = val,
            11 => self.wall_accel_use = val,
            12 => self.wall_accel_decrease = val,
            13 => self.trail_height = val,
            14 => self.trail_fade_duration = val,
            15 => self.speed_oz_factor = val,
            16 => self.speed_oz_freq = val,
            _  => {}
        }
    }

    /// Serialize to lines "name=value\n".
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        for (i, f) in ADVANCED_FIELDS.iter().enumerate() {
            s.push_str(&format!("{}={}\n", f.name, self.get_field(i)));
        }
        s
    }

    /// Deserialize from "name=value\n" lines.
    pub fn from_str(text: &str) -> Self {
        let mut cfg = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if let Some((key, val_str)) = line.split_once('=') {
                if let Ok(val) = val_str.trim().parse::<f32>() {
                    if let Some(idx) = ADVANCED_FIELDS.iter().position(|f| f.name == key.trim()) {
                        cfg.set_field(idx, val);
                    }
                }
            }
        }
        cfg
    }
}

// === Game Rules ===

#[derive(Debug, Clone)]
pub struct BoosterRules {
    pub enabled: bool,
}

impl Default for BoosterRules {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone)]
pub struct GameRules {
    pub speed: f32,
    pub grid_size: f32,
    pub erase_crashed: bool,
    pub booster: BoosterRules,
    pub ai_level: usize,
    pub human_slots: [bool; 4],
    pub num_ai: usize,
    pub ai_players: Vec<AiKind>,
    pub wall_accel: bool,
    pub advanced: AdvancedConfig,
    pub rounds_to_win: i32,
}

impl Default for GameRules {
    fn default() -> Self {
        let mut rules = Self {
            speed: 8.5,
            grid_size: 720.0,
            erase_crashed: true,
            booster: BoosterRules::default(),
            ai_level: 2,
            human_slots: [true, false, false, false],
            num_ai: 3,
            ai_players: Vec::new(),
            wall_accel: true,
            advanced: AdvancedConfig::default(),
            rounds_to_win: 10,
        };
        rules.rebuild_players();
        rules
    }
}

impl GameRules {
    /// Count of enabled human slots.
    pub fn num_humans(&self) -> usize {
        self.human_slots.iter().filter(|&&e| e).count()
    }

    /// Rebuild ai_players Vec from human_slots and num_ai.
    /// Human players occupy their exact slot positions (for keyboard mapping).
    /// Empty slots before the last human are filled with AI (counting against num_ai).
    /// Remaining AI are appended after.
    pub fn rebuild_players(&mut self) {
        // Find highest enabled human slot (1-indexed length needed)
        let max_slot = self.human_slots.iter().enumerate()
            .rev()
            .find(|(_, e)| **e)
            .map(|(i, _)| i + 1)
            .unwrap_or(0);

        let num_humans = self.num_humans();
        let total = max_slot.max(num_humans + self.num_ai);

        self.ai_players = Vec::with_capacity(total);
        for i in 0..total {
            if i < 4 && self.human_slots[i] {
                self.ai_players.push(AiKind::Human);
            } else {
                self.ai_players.push(AiKind::Computer);
            }
        }

        // Ensure at least 1 player
        if self.ai_players.is_empty() {
            self.ai_players.push(AiKind::Computer);
            self.num_ai = 1;
        }
    }
}

// === Game Time ===

#[derive(Debug, Clone, Default)]
pub struct GameTime {
    pub current: u32,
    pub last_frame: u32,
    pub dt: u32,
}

// === Pause State ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseState {
    NoGame,
    Running,
    Finished,
    Suspended,
}

// === Full Game State ===

pub struct GameState {
    pub players: Vec<Player>,
    pub visuals: Vec<PlayerVisual>,
    pub rules: GameRules,
    pub time: GameTime,
    pub winner: Option<usize>,
    pub running: usize,
    pub pause: PauseState,
    pub pause_modal: bool,
    pub pause_modal_selection: u8, // 0 = Resume, 1 = Quit to Menu
    pub events: Vec<GameEvent>,
    pub recognizer_alpha: f32,
    /// Fast-forward mode (hold boost when ALL humans dead to speed up AI play).
    pub fast_forward: bool,
}

impl GameState {
    /// Find the index of the first human player that is still alive.
    pub fn first_human(&self) -> Option<usize> {
        self.players.iter().position(|p| p.ai.kind == AiKind::Human && p.data.speed > -2.0)
    }

    /// Find the index of the first human player regardless of alive/dead status.
    pub fn human_index(&self) -> Option<usize> {
        self.players.iter().position(|p| p.ai.kind == AiKind::Human)
    }
}

// === Arena Walls ===

pub fn arena_walls(grid_size: f32) -> [Segment2; 4] {
    let g = grid_size / 2.0;
    [
        Segment2::new(Vec2::new(-g, -g), Vec2::new(grid_size, 0.0)),  // bottom
        Segment2::new(Vec2::new(g, -g), Vec2::new(0.0, grid_size)),   // right
        Segment2::new(Vec2::new(g, g), Vec2::new(-grid_size, 0.0)),   // top
        Segment2::new(Vec2::new(-g, g), Vec2::new(0.0, -grid_size)),  // left
    ]
}

// === Rendering Resources ===

pub struct GlMesh {
    pub vao: u32,
    pub vbo: u32,
    pub nbo: u32,
    pub ebo: u32,
    pub num_indices: i32,
    pub material_groups: Vec<MaterialGroup>,
    pub cpu_positions: Vec<f32>,  // [x,y,z, ...] flat - retained for model shattering
    pub cpu_normals: Vec<f32>,    // [nx,ny,nz, ...] flat
    pub cpu_indices: Vec<u32>,    // triangle indices
}

pub struct MaterialGroup {
    pub start_index: i32,
    pub num_indices: i32,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub shininess: f32,
}


// === Texture indices ===

pub const TEX_FLOOR: usize = 0;
pub const TEX_WALL1: usize = 1;
pub const TEX_WALL2: usize = 2;
pub const TEX_WALL3: usize = 3;
pub const TEX_WALL4: usize = 4;
pub const TEX_TRAIL: usize = 5;
pub const TEX_DECAL: usize = 6;
pub const TEX_SKYBOX: usize = 7; // 7..12 (6 faces)
pub const TEX_IMPACT: usize = 13;
pub const TEX_LOGO: usize = 14;
pub const TEX_GUI: usize = 15;
pub const NUM_TEXTURES: usize = 16;

// === App State ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Menu,
    Playing,
}

// === Viewport ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportMode {
    Single,
    Split,
    ThreeWay,
    FourWay,
}

impl ViewportMode {
    pub fn count(self) -> usize {
        match self {
            ViewportMode::Single => 1,
            ViewportMode::Split => 2,
            ViewportMode::ThreeWay => 3,
            ViewportMode::FourWay => 4,
        }
    }

    /// Pick the right viewport layout for a given number of human players.
    pub fn for_humans(count: usize) -> Self {
        match count {
            0 | 1 => ViewportMode::Single,
            2 => ViewportMode::Split,
            3 => ViewportMode::ThreeWay,
            _ => ViewportMode::FourWay,
        }
    }
}

// === Settings ===

#[derive(Debug, Clone)]
pub struct Settings {
    pub fov: f32,
    pub show_floor_texture: bool,
    pub show_wall: bool,
    pub show_skybox: bool,
    pub show_recognizer: bool,
    pub show_impact: bool,
    pub show_glow: bool,
    pub show_reflections: bool,
    pub alpha_trails: bool,
    pub antialias_lines: bool,
    pub show_decals: bool,
    pub turn_cycle: bool,
    pub light_cycles: bool,
    pub lod: bool,
    pub line_spacing: f32,
    pub fast_finish: bool,
    pub cam_type: CameraKind,
    pub use_stencil: bool,
    pub viewport_mode: ViewportMode,
    pub show_scores: bool,
    pub show_minimap: bool,
    pub play_music: bool,
    pub play_effects: bool,
    pub music_volume: f32,
    pub fx_volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fov: 105.0,
            // Hardcoded graphics options (always max quality, no menu toggle):
            // - Floor texture: on
            // - Walls: on
            // - Skybox: on
            // - Impact effects: on (explosion glow on crash)
            // - Antialiased lines: on
            // - Trail decals: on (textured trail surfaces)
            // - Turn cycle animation: on
            // - Light cycles rendered: on
            // - LOD (level of detail): on (high-detail cycle models)
            // - Stencil shadows: on
            // Menu-togglable: transparent trails, halos, recognizers, fps, ai status, scores
            show_floor_texture: true,
            show_wall: true,
            show_skybox: true,
            show_recognizer: true,
            show_impact: true,
            show_glow: true,
            show_reflections: true,
            alpha_trails: false,
            antialias_lines: true,
            show_decals: true,
            turn_cycle: true,
            light_cycles: true,
            lod: true,
            use_stencil: true,
            line_spacing: 20.0,
            fast_finish: true,
            cam_type: CameraKind::Follow,
            viewport_mode: ViewportMode::Single,
            show_scores: true,
            show_minimap: true,
            play_music: true,
            play_effects: true,
            music_volume: 0.15,
            fx_volume: 0.4,
        }
    }
}
