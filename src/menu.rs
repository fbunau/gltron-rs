use winit::keyboard::KeyCode;
use crate::renderer::{self, Renderer, Vertex, quad_verts, textured_quad_verts, line_loop_verts};
use crate::types::*;
use crate::assets::{self, BitmapFont};
use crate::render;
use crate::input::{KeyBindings, InputBinding, PlayerAction, ALL_ACTIONS, save_keybindings, MAX_KEY_PLAYERS};
use crate::gamepad::GamepadBtn;

/// Actions that the menu can trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    None,
    StartGame,
    Quit,
    Credits,
    Resize(u32, u32),
    SetWindowed(bool),
    AudioChanged,
}

/// Identifies which setting a list item controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingId {
    Booster,
    WallAccel,
    GameSpeed,
    BotSkill,
    ArenaSize,
    EraseDeadPlayers,
    FastFinish,
    CameraMode,
    Map2D,
    NumAi,
    TransparentTrails,
    Halos,
    Recognizers,
    Scores,
    Music,
    SoundFx,
    MusicVolume,
    FxVolume,
    Windowed,
    Preset,
    RoundsToWin,
}

/// Kind of menu item.
#[derive(Debug, Clone)]
enum MenuItemKind {
    Action(MenuAction),
    List {
        options: Vec<&'static str>,
        current: usize,
        setting: SettingId,
    },
    SubMenu {
        items: Vec<MenuItem>,
    },
    Label(String),
    /// Slider for numeric values. field_idx indexes into ADVANCED_FIELDS.
    Slider {
        field_idx: usize,
        value: f32,
    },
    /// Special actions for preset/advanced management
    SpecialAction(SpecialActionKind),
    /// Key binding item: Enter activates "press a key" capture mode.
    KeyBind {
        player: usize,
        action: PlayerAction,
        current_name: String,
    },
    /// Human player slot toggles: [P1] [P2] [P3] [P4] with colors.
    PlayerToggle {
        enabled: [bool; 4],
        cursor: usize,
    },
    /// Controls: inline [P1] [P2] [P3] [P4] selector; Enter opens that player's bindings.
    ControlsSelect {
        cursor: usize,
    },
}

#[derive(Debug, Clone, Copy)]
enum SpecialActionKind {
    SavePreset,
    DeletePreset(usize), // index in preset list
    LoadPreset(usize),   // index in preset list (0 = Default)
    ResetAdvanced,
}

/// A single menu item.
#[derive(Debug, Clone)]
struct MenuItem {
    label: String,
    kind: MenuItemKind,
}

/// Saved preset (name + config data).
#[derive(Debug, Clone)]
struct Preset {
    name: String,
    config: AdvancedConfig,
}

// ============================================================================
// Credits screen types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum CreditsStyle {
    Header,
    SubHeader,
    Normal,
    Separator,
    Blank,
}

#[derive(Debug, Clone)]
struct CreditsSpan {
    text: String,
    highlighted: bool,  // **bold** → teal
    accent: bool,       // *name* → gold
    is_url: bool,
}

#[derive(Debug, Clone)]
struct CreditsParsedLine {
    spans: Vec<CreditsSpan>,
    style: CreditsStyle,
    url: Option<String>,
}

/// A clickable URL region on screen (computed during rendering).
struct UrlHitRegion {
    url: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct CreditsState {
    lines: Vec<CreditsParsedLine>,
    scroll_offset: f32,
    total_height: f32,
    url_regions: Vec<UrlHitRegion>,
}

/// Full menu state.
pub struct MenuState {
    stack: Vec<(Vec<MenuItem>, usize)>,
    pub elapsed_ms: u32,
    presets: Vec<Preset>,       // index 0 is always "Default"
    active_preset: usize,       // which preset is currently active (0 = Default)
    naming: Option<String>,     // Some(text) when typing a preset name
    capturing_key: Option<(usize, PlayerAction)>, // Some((player, action)) when waiting for key press
    credits: Option<CreditsState>,
}

impl MenuState {
    pub fn new(rules: &GameRules, settings: &Settings, bindings: &KeyBindings) -> Self {
        let mut presets = vec![Preset {
            name: "Default".into(),
            config: AdvancedConfig::default(),
        }];
        #[cfg(not(target_arch = "wasm32"))]
        presets.extend(load_presets_from_disk());
        #[cfg(target_arch = "wasm32")]
        presets.extend(load_presets_from_storage());

        let root = build_root_menu(rules, settings, &presets, 0, bindings);
        Self {
            stack: vec![(root, 0)],
            elapsed_ms: 0,
            presets,
            active_preset: 0,
            naming: None,
            capturing_key: None,
            credits: None,
        }
    }

    pub fn is_naming(&self) -> bool {
        self.naming.is_some()
    }

    pub fn is_capturing(&self) -> bool {
        self.capturing_key.is_some()
    }

    pub fn cancel_capture(&mut self) {
        self.capturing_key = None;
    }

    pub fn is_credits(&self) -> bool {
        self.credits.is_some()
    }

    pub fn open_credits(&mut self) {
        let text = load_credits_text();
        let lines = parse_credits(&text, 70);
        self.credits = Some(CreditsState {
            lines,
            scroll_offset: 0.0,
            total_height: 0.0,
            url_regions: Vec::new(),
        });
    }

    /// Check if a mouse click at (x, y) in screen coords hits a URL in the credits.
    /// The y coordinate uses GL convention (0 = bottom).
    /// Returns the URL string if a hit was found.
    pub fn credits_click(&self, screen_x: f32, screen_y: f32) -> Option<String> {
        let credits = self.credits.as_ref()?;
        for region in &credits.url_regions {
            if screen_x >= region.x && screen_x <= region.x + region.w
                && screen_y >= region.y && screen_y <= region.y + region.h
            {
                return Some(region.url.clone());
            }
        }
        None
    }
}

// ============================================================================
// Credits loading & parsing
// ============================================================================

fn load_credits_text() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string("assets/credits.txt")
            .unwrap_or_else(|_| "(credits file not found)".to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        crate::embedded_assets::CREDITS_TXT.to_string()
    }
}

fn wrap_line(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn make_span(text: &str) -> CreditsSpan {
    CreditsSpan { text: text.to_string(), highlighted: false, accent: false, is_url: false }
}

fn parse_spans(text: &str) -> Vec<CreditsSpan> {
    // Pass 1: split on **bold** markers
    let mut pass1 = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("**") {
        if start > 0 {
            pass1.push(make_span(&rest[..start]));
        }
        rest = &rest[start + 2..];
        if let Some(end) = rest.find("**") {
            pass1.push(CreditsSpan { highlighted: true, ..make_span(&rest[..end]) });
            rest = &rest[end + 2..];
        } else {
            pass1.push(make_span(rest));
            rest = "";
        }
    }
    if !rest.is_empty() {
        pass1.push(make_span(rest));
    }
    if pass1.is_empty() {
        pass1.push(make_span(text));
    }

    // Pass 2: split spans on *accent* markers (works inside **bold** too)
    let mut pass2 = Vec::new();
    for span in pass1 {
        if span.is_url {
            pass2.push(span);
            continue;
        }
        let parent_highlighted = span.highlighted;
        let mut r = span.text.as_str();
        let mut any_pushed = false;
        while let Some(start) = r.find('*') {
            if start > 0 {
                pass2.push(CreditsSpan { highlighted: parent_highlighted, ..make_span(&r[..start]) });
                any_pushed = true;
            }
            r = &r[start + 1..];
            if let Some(end) = r.find('*') {
                pass2.push(CreditsSpan { accent: true, highlighted: parent_highlighted, ..make_span(&r[..end]) });
                r = &r[end + 1..];
                any_pushed = true;
            } else {
                pass2.push(CreditsSpan { highlighted: parent_highlighted, ..make_span(&format!("*{}", r)) });
                r = "";
            }
        }
        if !r.is_empty() {
            pass2.push(CreditsSpan { highlighted: parent_highlighted, ..make_span(r) });
        } else if !any_pushed {
            pass2.push(span);
        }
    }
    if pass2.is_empty() {
        pass2.push(make_span(text));
    }

    // Pass 3: split plain spans that contain URLs
    let mut result = Vec::new();
    for span in pass2 {
        if span.highlighted || span.accent || span.is_url {
            result.push(span);
            continue;
        }
        let t = &span.text;
        let url_start = t.find("https://").or_else(|| t.find("http://"));
        if let Some(start) = url_start {
            if start > 0 {
                result.push(make_span(&t[..start]));
            }
            let url_end = t[start..].find(' ').map(|i| start + i).unwrap_or(t.len());
            result.push(CreditsSpan { is_url: true, ..make_span(&t[start..url_end]) });
            if url_end < t.len() {
                result.push(make_span(&t[url_end..]));
            }
        } else {
            result.push(span);
        }
    }
    result
}

fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            return Some(word.to_string());
        }
    }
    None
}

fn parse_credits(text: &str, max_chars: usize) -> Vec<CreditsParsedLine> {
    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            result.push(CreditsParsedLine { spans: vec![], style: CreditsStyle::Blank, url: None });
            continue;
        }
        if trimmed == "---" {
            result.push(CreditsParsedLine { spans: vec![], style: CreditsStyle::Separator, url: None });
            continue;
        }
        let (style, content) = if let Some(rest) = trimmed.strip_prefix("## ") {
            (CreditsStyle::SubHeader, rest)
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            (CreditsStyle::Header, rest)
        } else {
            (CreditsStyle::Normal, trimmed)
        };
        let url = extract_url(content);
        // Strip ** markers for line-length measurement, then wrap
        let plain = content.replace("**", "");
        let wrapped = wrap_line(&plain, max_chars);
        if wrapped.len() == 1 {
            result.push(CreditsParsedLine { spans: parse_spans(content), style, url });
        } else {
            for w in &wrapped {
                let line_url = extract_url(w);
                result.push(CreditsParsedLine { spans: parse_spans(w), style, url: line_url });
            }
        }
    }
    // Post-process: absorb blank lines adjacent to separators so the
    // separator gap is the sole source of spacing around divider lines.
    let mut cleaned = Vec::with_capacity(result.len());
    for (i, line) in result.iter().enumerate() {
        if line.style == CreditsStyle::Blank {
            // Drop blank if next line is a separator
            if result.get(i + 1).is_some_and(|l| l.style == CreditsStyle::Separator) {
                continue;
            }
            // Drop blank if previous line is a separator
            if i > 0 && result[i - 1].style == CreditsStyle::Separator {
                continue;
            }
        }
        cleaned.push(line.clone());
    }
    cleaned
}

// ============================================================================
// Preset file I/O
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
const PRESETS_DIR: &str = "presets";

#[cfg(not(target_arch = "wasm32"))]
fn load_presets_from_disk() -> Vec<Preset> {
    let dir = std::path::Path::new(PRESETS_DIR);
    if !dir.exists() {
        return Vec::new();
    }
    let mut presets = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        files.sort_by_key(|e| e.file_name());
        for entry in files {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "txt") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    let name = path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    presets.push(Preset {
                        name,
                        config: AdvancedConfig::from_str(&text),
                    });
                }
            }
        }
    }
    presets
}

#[cfg(not(target_arch = "wasm32"))]
fn save_preset_to_disk(name: &str, config: &AdvancedConfig) {
    let dir = std::path::Path::new(PRESETS_DIR);
    if !dir.exists() {
        std::fs::create_dir_all(dir).ok();
    }
    let path = dir.join(format!("{}.txt", name));
    std::fs::write(path, config.to_string()).ok();
}

#[cfg(not(target_arch = "wasm32"))]
fn delete_preset_from_disk(name: &str) {
    let path = std::path::Path::new(PRESETS_DIR).join(format!("{}.txt", name));
    std::fs::remove_file(path).ok();
}

#[cfg(target_arch = "wasm32")]
fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn load_presets_from_storage() -> Vec<Preset> {
    let Some(storage) = get_local_storage() else { return Vec::new() };
    let len = storage.length().unwrap_or(0);
    let mut presets = Vec::new();
    for i in 0..len {
        if let Ok(Some(key)) = storage.key(i) {
            if let Some(name) = key.strip_prefix("gltron_preset:") {
                if let Ok(Some(text)) = storage.get_item(&key) {
                    presets.push(Preset {
                        name: name.to_string(),
                        config: AdvancedConfig::from_str(&text),
                    });
                }
            }
        }
    }
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    presets
}

#[cfg(target_arch = "wasm32")]
fn save_preset_to_storage(name: &str, config: &AdvancedConfig) {
    if let Some(storage) = get_local_storage() {
        storage.set_item(&format!("gltron_preset:{name}"), &config.to_string()).ok();
    }
}

#[cfg(target_arch = "wasm32")]
fn delete_preset_from_storage(name: &str) {
    if let Some(storage) = get_local_storage() {
        storage.remove_item(&format!("gltron_preset:{name}")).ok();
    }
}

fn save_preset(name: &str, config: &AdvancedConfig) {
    #[cfg(not(target_arch = "wasm32"))]
    save_preset_to_disk(name, config);
    #[cfg(target_arch = "wasm32")]
    save_preset_to_storage(name, config);
}

fn delete_preset(name: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    delete_preset_from_disk(name);
    #[cfg(target_arch = "wasm32")]
    delete_preset_from_storage(name);
}

fn next_preset_name(presets: &[Preset]) -> String {
    let mut n = 1;
    loop {
        let name = format!("Preset {}", n);
        if !presets.iter().any(|p| p.name == name) {
            return name;
        }
        n += 1;
    }
}

// ============================================================================
// Menu tree construction
// ============================================================================

fn build_root_menu(rules: &GameRules, settings: &Settings, presets: &[Preset], active_preset: usize, bindings: &KeyBindings) -> Vec<MenuItem> {
    vec![
        MenuItem {
            label: "game".into(),
            kind: MenuItemKind::SubMenu { items: build_game_menu(rules, settings, presets, active_preset, bindings) },
        },
        MenuItem {
            label: "video".into(),
            kind: MenuItemKind::SubMenu { items: build_video_menu(settings) },
        },
        MenuItem {
            label: "audio".into(),
            kind: MenuItemKind::SubMenu { items: build_audio_menu(settings) },
        },
        MenuItem {
            label: "credits".into(),
            kind: MenuItemKind::Action(MenuAction::Credits),
        },
        MenuItem {
            label: "quit".into(),
            kind: MenuItemKind::Action(MenuAction::Quit),
        },
    ]
}

fn build_game_menu(rules: &GameRules, settings: &Settings, presets: &[Preset], active_preset: usize, bindings: &KeyBindings) -> Vec<MenuItem> {
    vec![
        MenuItem { label: "start game".into(), kind: MenuItemKind::Action(MenuAction::StartGame) },
        MenuItem {
            label: "human players".into(),
            kind: MenuItemKind::PlayerToggle {
                enabled: rules.human_slots,
                cursor: 0,
            },
        },
        MenuItem {
            label: "game rules".into(),
            kind: MenuItemKind::SubMenu { items: build_game_rules(rules, presets, active_preset) },
        },
        MenuItem {
            label: "hud and fx".into(),
            kind: MenuItemKind::SubMenu { items: build_play_settings(rules, settings) },
        },
        MenuItem {
            label: "controls".into(),
            kind: MenuItemKind::ControlsSelect { cursor: 0 },
        },
    ]
}

fn build_game_rules(rules: &GameRules, presets: &[Preset], active_preset: usize) -> Vec<MenuItem> {
    let booster_idx = if rules.booster.enabled { 1 } else { 0 };
    let wall_accel_idx = if rules.wall_accel { 1 } else { 0 };

    let speed_idx = if rules.speed <= 5.5 { 0 }
        else if rules.speed <= 7.5 { 1 }
        else if rules.speed <= 10.0 { 2 }
        else { 3 };

    let ai_idx = rules.ai_level.min(3);

    let arena_idx = if rules.grid_size <= 200.0 { 0 }
        else if rules.grid_size <= 400.0 { 1 }
        else if rules.grid_size <= 600.0 { 2 }
        else if rules.grid_size <= 900.0 { 3 }
        else { 4 };

    let erase_idx = if rules.erase_crashed { 1 } else { 0 };

    vec![
        MenuItem {
            label: "presets".into(),
            kind: MenuItemKind::SubMenu { items: build_presets_menu(presets, active_preset) },
        },
        MenuItem {
            label: "ai players".into(),
            kind: MenuItemKind::List {
                options: vec!["0", "1", "2", "3", "4", "5", "6"],
                current: rules.num_ai,
                setting: SettingId::NumAi,
            },
        },
        MenuItem {
            label: "bot skill".into(),
            kind: MenuItemKind::List {
                options: vec!["easy", "normal", "hard", "very hard"],
                current: ai_idx,
                setting: SettingId::BotSkill,
            },
        },
        MenuItem {
            label: "rounds to win".into(),
            kind: MenuItemKind::List {
                options: vec!["3", "5", "7", "10", "15", "20"],
                current: match rules.rounds_to_win {
                    3 => 0, 5 => 1, 7 => 2, 10 => 3, 15 => 4, _ => 5,
                },
                setting: SettingId::RoundsToWin,
            },
        },
        MenuItem {
            label: "booster".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: booster_idx,
                setting: SettingId::Booster,
            },
        },
        MenuItem {
            label: "wall acceleration".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: wall_accel_idx,
                setting: SettingId::WallAccel,
            },
        },
        MenuItem {
            label: "game speed".into(),
            kind: MenuItemKind::List {
                options: vec!["boring", "normal", "fast", "crazy"],
                current: speed_idx,
                setting: SettingId::GameSpeed,
            },
        },
        MenuItem {
            label: "arena size".into(),
            kind: MenuItemKind::List {
                options: vec!["tiny", "small", "medium", "large", "extra large"],
                current: arena_idx,
                setting: SettingId::ArenaSize,
            },
        },
        MenuItem {
            label: "erase dead players".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: erase_idx,
                setting: SettingId::EraseDeadPlayers,
            },
        },
        MenuItem {
            label: "advanced settings".into(),
            kind: MenuItemKind::SubMenu { items: build_advanced_menu(&rules.advanced) },
        },
    ]
}


// ============================================================================
// Advanced settings submenus
// ============================================================================

fn build_advanced_menu(adv: &AdvancedConfig) -> Vec<MenuItem> {
    vec![
        MenuItem {
            label: "boost settings".into(),
            kind: MenuItemKind::SubMenu { items: build_boost_sliders(adv) },
        },
        MenuItem {
            label: "wall accel settings".into(),
            kind: MenuItemKind::SubMenu { items: build_wall_accel_sliders(adv) },
        },
        MenuItem {
            label: "general settings".into(),
            kind: MenuItemKind::SubMenu { items: build_general_sliders(adv) },
        },
        MenuItem {
            label: "reset to defaults".into(),
            kind: MenuItemKind::SpecialAction(SpecialActionKind::ResetAdvanced),
        },
    ]
}

fn make_slider(field_idx: usize, adv: &AdvancedConfig) -> MenuItem {
    let field = &ADVANCED_FIELDS[field_idx];
    MenuItem {
        label: field.label.into(),
        kind: MenuItemKind::Slider {
            field_idx,
            value: adv.get_field(field_idx),
        },
    }
}

fn build_boost_sliders(adv: &AdvancedConfig) -> Vec<MenuItem> {
    // Fields 0-9: boost_accel_rate through zone_red_drain
    (0..=9).map(|i| make_slider(i, adv)).collect()
}

fn build_wall_accel_sliders(adv: &AdvancedConfig) -> Vec<MenuItem> {
    // Fields 10-12: wall_accel_limit, wall_accel_use, wall_accel_decrease
    (10..=12).map(|i| make_slider(i, adv)).collect()
}

fn build_general_sliders(adv: &AdvancedConfig) -> Vec<MenuItem> {
    // Fields 13-16: trail_height, trail_fade_duration, speed_oz_factor, speed_oz_freq
    (13..=16).map(|i| make_slider(i, adv)).collect()
}

// ============================================================================
// Presets submenu
// ============================================================================

fn build_presets_menu(presets: &[Preset], active_idx: usize) -> Vec<MenuItem> {
    let mut items = Vec::new();

    // Hint at the top
    items.push(MenuItem {
        label: "[enter: select, del: delete]".into(),
        kind: MenuItemKind::Label(String::new()),
    });

    // List all presets
    for (i, p) in presets.iter().enumerate() {
        let marker = if i == active_idx { "* " } else { "  " };
        items.push(MenuItem {
            label: format!("{}{}", marker, p.name),
            kind: MenuItemKind::SpecialAction(SpecialActionKind::LoadPreset(i)),
        });
    }

    // Save current as new preset
    items.push(MenuItem {
        label: "save as preset".into(),
        kind: MenuItemKind::SpecialAction(SpecialActionKind::SavePreset),
    });

    items
}

// ============================================================================
// Other menu builders (unchanged)
// ============================================================================

fn build_play_settings(rules: &GameRules, settings: &Settings) -> Vec<MenuItem> {
    let cam_idx = match settings.cam_type {
        CameraKind::Follow => 0,
        CameraKind::Cockpit => 1,
        CameraKind::Free => 2,
        CameraKind::Circling => 3,
    };
    let trails_idx = if settings.alpha_trails { 1 } else { 0 };
    let halos_idx = if settings.show_glow { 1 } else { 0 };
    let recog_idx = if settings.show_recognizer { 1 } else { 0 };

    vec![
        MenuItem {
            label: "camera mode".into(),
            kind: MenuItemKind::List {
                options: vec!["behind", "cockpit", "mouse", "circling"],
                current: cam_idx,
                setting: SettingId::CameraMode,
            },
        },
        MenuItem {
            label: "2d map".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: if settings.show_minimap { 1 } else { 0 },
                setting: SettingId::Map2D,
            },
        },
        MenuItem {
            label: "halos".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: halos_idx,
                setting: SettingId::Halos,
            },
        },
        MenuItem {
            label: "recognizers".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: recog_idx,
                setting: SettingId::Recognizers,
            },
        },
        MenuItem {
            label: "transparent trails".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: trails_idx,
                setting: SettingId::TransparentTrails,
            },
        },
    ]
}

fn build_player_keys(bindings: &KeyBindings, p: usize) -> Vec<MenuItem> {
    let keys = &bindings.players[p];
    ALL_ACTIONS.iter().map(|&action| {
        MenuItem {
            label: action.label().into(),
            kind: MenuItemKind::KeyBind {
                player: p,
                action,
                current_name: keys.get(action).display_name(),
            },
        }
    }).collect()
}

fn build_video_menu(_settings: &Settings) -> Vec<MenuItem> {
    let mut items = Vec::new();

    // Resolution options (desktop only — on WASM we auto-resize to browser window)
    #[cfg(not(target_arch = "wasm32"))]
    {
        items.push(MenuItem { label: "1280 x 720".into(), kind: MenuItemKind::Action(MenuAction::Resize(1280, 720)) });
        items.push(MenuItem { label: "1600 x 900".into(), kind: MenuItemKind::Action(MenuAction::Resize(1600, 900)) });
        items.push(MenuItem { label: "1920 x 1080".into(), kind: MenuItemKind::Action(MenuAction::Resize(1920, 1080)) });
        items.push(MenuItem { label: "2560 x 1440".into(), kind: MenuItemKind::Action(MenuAction::Resize(2560, 1440)) });
        items.push(MenuItem { label: "3440 x 1440".into(), kind: MenuItemKind::Action(MenuAction::Resize(3440, 1440)) });
        items.push(MenuItem { label: "3840 x 2160".into(), kind: MenuItemKind::Action(MenuAction::Resize(3840, 2160)) });
    }

    items.push(MenuItem {
        label: "windowed".into(),
        kind: MenuItemKind::List {
            options: vec!["off", "on"],
            current: 1,
            setting: SettingId::Windowed,
        },
    });

    items
}

fn build_audio_menu(settings: &Settings) -> Vec<MenuItem> {
    let music_idx = if settings.play_music { 1 } else { 0 };
    let fx_idx = if settings.play_effects { 1 } else { 0 };
    let music_vol = (settings.music_volume * 10.0).round() as usize;
    let fx_vol = (settings.fx_volume * 10.0).round() as usize;

    vec![
        MenuItem {
            label: "music".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: music_idx,
                setting: SettingId::Music,
            },
        },
        MenuItem {
            label: "sound fx".into(),
            kind: MenuItemKind::List {
                options: vec!["off", "on"],
                current: fx_idx,
                setting: SettingId::SoundFx,
            },
        },
        MenuItem {
            label: "music volume".into(),
            kind: MenuItemKind::List {
                options: vec!["0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%"],
                current: music_vol.min(10),
                setting: SettingId::MusicVolume,
            },
        },
        MenuItem {
            label: "fx volume".into(),
            kind: MenuItemKind::List {
                options: vec!["0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%"],
                current: fx_vol.min(10),
                setting: SettingId::FxVolume,
            },
        },
        MenuItem {
            label: "song: revenge of cats".into(),
            kind: MenuItemKind::Action(MenuAction::None),
        },
    ]
}

// ============================================================================
// Setting application
// ============================================================================

fn apply_setting(setting: SettingId, value: usize, rules: &mut GameRules, settings: &mut Settings) {
    match setting {
        SettingId::Booster => {
            rules.booster.enabled = value == 1;
        }
        SettingId::WallAccel => {
            rules.wall_accel = value == 1;
        }
        SettingId::GameSpeed => {
            rules.speed = match value { 0 => 5.0, 1 => 6.5, 2 => 8.5, _ => 12.0 };
        }
        SettingId::BotSkill => {
            rules.ai_level = value.min(3);
        }
        SettingId::ArenaSize => {
            rules.grid_size = match value { 0 => 160.0, 1 => 240.0, 2 => 480.0, 3 => 720.0, _ => 1200.0 };
        }
        SettingId::EraseDeadPlayers => {
            rules.erase_crashed = value == 1;
        }
        SettingId::FastFinish => {
            settings.fast_finish = value == 1;
        }
        SettingId::CameraMode => {
            settings.cam_type = match value {
                0 => CameraKind::Follow,
                1 => CameraKind::Cockpit,
                2 => CameraKind::Free,
                _ => CameraKind::Circling,
            };
        }
        SettingId::Map2D => {
            settings.show_minimap = value >= 1;
        }
        SettingId::NumAi => {
            rules.num_ai = value.min(6);
            if rules.num_humans() + rules.num_ai < 1 {
                rules.num_ai = 1;
            }
            rules.rebuild_players();
        }
        SettingId::TransparentTrails => {
            settings.alpha_trails = value == 1;
        }
        SettingId::Halos => {
            settings.show_glow = value == 1;
        }
        SettingId::Recognizers => {
            settings.show_recognizer = value == 1;
        }
        SettingId::Scores => {
            settings.show_scores = value == 1;
        }
        SettingId::Music => {
            settings.play_music = value == 1;
        }
        SettingId::SoundFx => {
            settings.play_effects = value == 1;
        }
        SettingId::MusicVolume => {
            settings.music_volume = value as f32 / 10.0;
        }
        SettingId::FxVolume => {
            settings.fx_volume = value as f32 / 10.0;
        }
        SettingId::Windowed => { /* always windowed */ }
        SettingId::Preset => { /* handled via SpecialAction */ }
        SettingId::RoundsToWin => {
            rules.rounds_to_win = match value { 0 => 3, 1 => 5, 2 => 7, 3 => 10, 4 => 15, _ => 20 };
        }
    }
}

/// After applying NumAi, sync sibling menu item display values.
fn sync_player_counts(items: &mut [MenuItem], rules: &GameRules) {
    for item in items.iter_mut() {
        if let MenuItemKind::List { setting, current, .. } = &mut item.kind {
            if *setting == SettingId::NumAi {
                *current = rules.num_ai;
            }
        }
    }
}

// ============================================================================
// Input handling
// ============================================================================

/// Check if a setting change requires a special action returned to main.
fn setting_action(setting: SettingId, value: usize) -> MenuAction {
    match setting {
        SettingId::Windowed => MenuAction::SetWindowed(value == 1),
        SettingId::Music | SettingId::SoundFx | SettingId::MusicVolume | SettingId::FxVolume => MenuAction::AudioChanged,
        _ => MenuAction::None,
    }
}

/// Handle a keypress in the menu. Returns the action to take.
pub fn menu_key_input(
    menu: &mut MenuState,
    rules: &mut GameRules,
    settings: &mut Settings,
    bindings: &mut KeyBindings,
    keycode: KeyCode,
) -> MenuAction {
    // If we're in key capture mode, handle it
    if let Some((player, action)) = menu.capturing_key {
        return handle_key_capture(menu, bindings, player, action, keycode);
    }

    // If we're in naming mode, handle text input
    if menu.naming.is_some() {
        return handle_naming_input(menu, rules, keycode);
    }

    // If credits screen is displayed, handle credits input
    if menu.credits.is_some() {
        credits_key_input(menu, keycode);
        return MenuAction::None;
    }

    let stack_len = menu.stack.len();
    let (items, active) = menu.stack.last().unwrap();
    let active_idx = *active;
    let num_items = items.len();

    match keycode {
        KeyCode::ArrowUp => {
            let (_, active) = menu.stack.last_mut().unwrap();
            *active = if *active > 0 { *active - 1 } else { num_items - 1 };
        }
        KeyCode::ArrowDown => {
            let (_, active) = menu.stack.last_mut().unwrap();
            *active = (*active + 1) % num_items;
        }
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
            enum Act {
                DoAction(MenuAction),
                EnterSub(Vec<MenuItem>),
                CycleList(SettingId, usize),
                DoSpecial(SpecialActionKind),
                CaptureKey(usize, PlayerAction),
                TogglePlayer,
                ControlsEnter,
                Nothing,
            }
            let act = match &items[active_idx].kind {
                MenuItemKind::Action(a) => Act::DoAction(*a),
                MenuItemKind::SubMenu { items: sub } => Act::EnterSub(sub.clone()),
                MenuItemKind::List { options, current, setting } => {
                    Act::CycleList(*setting, (*current + 1) % options.len())
                }
                MenuItemKind::SpecialAction(sa) => Act::DoSpecial(*sa),
                MenuItemKind::KeyBind { player, action, .. } => Act::CaptureKey(*player, *action),
                MenuItemKind::PlayerToggle { .. } => Act::TogglePlayer,
                MenuItemKind::ControlsSelect { .. } => Act::ControlsEnter,
                MenuItemKind::Label(_) | MenuItemKind::Slider { .. } => Act::Nothing,
            };
            match act {
                Act::DoAction(a) => return a,
                Act::EnterSub(sub) => {
                    menu.stack.push((sub, 0));
                }
                Act::TogglePlayer => {
                    let (items, active) = menu.stack.last_mut().unwrap();
                    if let MenuItemKind::PlayerToggle { enabled, cursor } = &mut items[*active].kind {
                        enabled[*cursor] = !enabled[*cursor];
                        rules.human_slots = *enabled;
                        // Ensure at least 1 player total
                        if rules.num_humans() + rules.num_ai < 1 {
                            rules.num_ai = 1;
                        }
                        rules.rebuild_players();
                    }
                }
                Act::CycleList(setting, new_val) => {
                    let (items, active) = menu.stack.last_mut().unwrap();
                    if let MenuItemKind::List { current, .. } = &mut items[*active].kind {
                        *current = new_val;
                    }
                    apply_setting(setting, new_val, rules, settings);
                    if setting == SettingId::NumAi {
                        sync_player_counts(items, rules);
                    }
                    let sa = setting_action(setting, new_val);
                    if sa != MenuAction::None { return sa; }
                }
                Act::DoSpecial(sa) => {
                    handle_special_action(menu, rules, sa);
                }
                Act::CaptureKey(player, action) => {
                    menu.capturing_key = Some((player, action));
                }
                Act::ControlsEnter => {
                    let (items, active) = menu.stack.last_mut().unwrap();
                    if let MenuItemKind::ControlsSelect { cursor, .. } = &items[*active].kind {
                        let p = *cursor;
                        let player_items = build_player_keys(bindings, p);
                        menu.stack.push((player_items, 0));
                    }
                }
                Act::Nothing => {}
            }
        }
        KeyCode::ArrowLeft => {
            let (items, active) = menu.stack.last_mut().unwrap();
            match &mut items[*active].kind {
                MenuItemKind::PlayerToggle { cursor, .. } => {
                    *cursor = if *cursor > 0 { *cursor - 1 } else { 3 };
                }
                MenuItemKind::ControlsSelect { cursor, .. } => {
                    *cursor = if *cursor > 0 { *cursor - 1 } else { 3 };
                }
                MenuItemKind::List { options, current, setting } => {
                    let s = *setting;
                    *current = if *current > 0 { *current - 1 } else { options.len() - 1 };
                    let v = *current;
                    apply_setting(s, v, rules, settings);
                    if s == SettingId::NumAi {
                        sync_player_counts(items, rules);
                    }
                    let sa = setting_action(s, v);
                    if sa != MenuAction::None { return sa; }
                }
                MenuItemKind::Slider { field_idx, value } => {
                    let fi = *field_idx;
                    let field = &ADVANCED_FIELDS[fi];
                    *value = (*value - field.step).max(field.min);
                    *value = ((*value / field.step).round() * field.step).max(field.min);
                    rules.advanced.set_field(fi, *value);
                }
                _ => {}
            }
        }
        KeyCode::ArrowRight => {
            let (items, active) = menu.stack.last_mut().unwrap();
            match &mut items[*active].kind {
                MenuItemKind::PlayerToggle { cursor, .. } => {
                    *cursor = (*cursor + 1) % 4;
                }
                MenuItemKind::ControlsSelect { cursor, .. } => {
                    *cursor = (*cursor + 1) % 4;
                }
                MenuItemKind::List { options, current, setting } => {
                    let s = *setting;
                    *current = (*current + 1) % options.len();
                    let v = *current;
                    apply_setting(s, v, rules, settings);
                    if s == SettingId::NumAi {
                        sync_player_counts(items, rules);
                    }
                    let sa = setting_action(s, v);
                    if sa != MenuAction::None { return sa; }
                }
                MenuItemKind::Slider { field_idx, value } => {
                    let fi = *field_idx;
                    let field = &ADVANCED_FIELDS[fi];
                    *value = (*value + field.step).min(field.max);
                    *value = ((*value / field.step).round() * field.step).min(field.max);
                    rules.advanced.set_field(fi, *value);
                }
                _ => {}
            }
        }
        KeyCode::Delete => {
            // Delete key: delete the selected preset (if it's a preset item and not Default)
            let (items, active) = menu.stack.last().unwrap();
            let active_idx = *active;
            if let MenuItemKind::SpecialAction(SpecialActionKind::LoadPreset(idx)) = &items[active_idx].kind {
                let idx = *idx;
                if idx > 0 {
                    handle_special_action(menu, rules, SpecialActionKind::DeletePreset(idx));
                }
            }
        }
        KeyCode::Escape | KeyCode::Backspace => {
            if stack_len > 1 {
                menu.stack.pop();
            }
        }
        _ => {}
    }
    MenuAction::None
}

/// Handle keypress while credits screen is displayed.
fn credits_key_input(menu: &mut MenuState, keycode: KeyCode) {
    if let Some(ref mut credits) = menu.credits {
        match keycode {
            KeyCode::Escape | KeyCode::Backspace => {
                menu.credits = None;
            }
            KeyCode::ArrowUp => {
                credits.scroll_offset = (credits.scroll_offset - 40.0).max(0.0);
            }
            KeyCode::ArrowDown => {
                if credits.total_height > 0.0 {
                    credits.scroll_offset = (credits.scroll_offset + 40.0).min(credits.total_height);
                }
            }
            _ => {}
        }
    }
}

/// Handle text input (actual typed characters with shift/caps).
/// Called from main.rs when naming mode is active.
pub fn menu_text_input(menu: &mut MenuState, text: &str) {
    if let Some(ref mut name) = menu.naming {
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                if name.len() < 24 {
                    name.push(ch);
                }
            }
        }
    }
}

/// Handle a key press while in key capture mode.
/// Escape cancels, any other key rebinds the action and saves.
fn handle_key_capture(
    menu: &mut MenuState,
    bindings: &mut KeyBindings,
    player: usize,
    action: PlayerAction,
    keycode: KeyCode,
) -> MenuAction {
    if keycode == KeyCode::Escape {
        menu.capturing_key = None;
        return MenuAction::None;
    }

    apply_capture(menu, bindings, player, action, InputBinding::Key(keycode));
    MenuAction::None
}

/// Apply a captured input binding (keyboard or gamepad).
fn apply_capture(
    menu: &mut MenuState,
    bindings: &mut KeyBindings,
    player: usize,
    action: PlayerAction,
    binding: InputBinding,
) {
    bindings.players[player].set(action, binding);
    menu.capturing_key = None;

    // Update the menu item's current_name to reflect the new binding
    if let Some((items, _)) = menu.stack.last_mut() {
        for item in items.iter_mut() {
            if let MenuItemKind::KeyBind { player: p, action: a, current_name } = &mut item.kind {
                if *p == player && *a == action {
                    *current_name = binding.display_name();
                }
            }
        }
    }

    // Persist
    save_keybindings(bindings);
}

/// Handle a gamepad button press during key capture mode.
/// Returns true if the button was consumed (capture completed).
pub fn menu_gamepad_capture(
    menu: &mut MenuState,
    bindings: &mut KeyBindings,
    btn: GamepadBtn,
) -> bool {
    if let Some((player, action)) = menu.capturing_key {
        apply_capture(menu, bindings, player, action, InputBinding::Pad(btn));
        true
    } else {
        false
    }
}

/// Handle keyboard input while in preset naming mode.
/// Only handles control keys (Enter/Escape/Backspace).
/// Character input comes via menu_text_input (winit text events).
fn handle_naming_input(
    menu: &mut MenuState,
    rules: &mut GameRules,
    keycode: KeyCode,
) -> MenuAction {
    match keycode {
        KeyCode::Enter | KeyCode::NumpadEnter => {
            if let Some(name) = menu.naming.take() {
                let name = name.trim().to_string();
                if !name.is_empty() && name != "Default" {
                    // Sanitize for filesystem safety
                    let safe_name: String = name.chars()
                        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
                        .collect();
                    let safe_name = if safe_name.is_empty() {
                        next_preset_name(&menu.presets)
                    } else {
                        safe_name
                    };
                    // Overwrite if name exists (except Default)
                    if let Some(idx) = menu.presets.iter().position(|p| p.name == safe_name) {
                        if idx > 0 {
                            menu.presets[idx].config = rules.advanced.clone();
                            save_preset(&safe_name, &rules.advanced);
                            menu.active_preset = idx;
                        }
                    } else {
                        save_preset(&safe_name, &rules.advanced);
                        menu.presets.push(Preset {
                            name: safe_name,
                            config: rules.advanced.clone(),
                        });
                        menu.active_preset = menu.presets.len() - 1;
                    }
                    // Stay in presets submenu, rebuild in place
                    rebuild_presets_in_place(menu);
                    update_advanced_in_parent(menu, &rules.advanced);
                } else {
                    menu.naming = None;
                }
            }
        }
        KeyCode::Escape => {
            menu.naming = None;
        }
        KeyCode::Backspace => {
            if let Some(ref mut name) = menu.naming {
                name.pop();
            }
        }
        _ => {} // character input handled by menu_text_input
    }
    MenuAction::None
}

/// Handle special menu actions (presets, reset).
fn handle_special_action(
    menu: &mut MenuState,
    rules: &mut GameRules,
    action: SpecialActionKind,
) {
    match action {
        SpecialActionKind::ResetAdvanced => {
            rules.advanced = AdvancedConfig::default();
            rebuild_advanced_on_stack(menu, &rules.advanced);
        }
        SpecialActionKind::LoadPreset(idx) => {
            if idx < menu.presets.len() {
                rules.advanced = menu.presets[idx].config.clone();
                menu.active_preset = idx;
                // Stay in presets submenu, rebuild it in place to update the marker
                rebuild_presets_in_place(menu);
                // Also update advanced settings in parent game rules level
                update_advanced_in_parent(menu, &rules.advanced);
            }
        }
        SpecialActionKind::SavePreset => {
            menu.naming = Some(String::new());
        }
        SpecialActionKind::DeletePreset(idx) => {
            if idx > 0 && idx < menu.presets.len() {
                let name = menu.presets[idx].name.clone();
                delete_preset(&name);
                menu.presets.remove(idx);
                // If we deleted the active preset, fall back to Default
                if menu.active_preset == idx {
                    menu.active_preset = 0;
                } else if menu.active_preset > idx {
                    menu.active_preset -= 1;
                }
                // Stay in presets submenu, rebuild in place
                rebuild_presets_in_place(menu);
            }
        }
    }
}

/// Rebuild the presets submenu in place (without popping), updating the active marker.
fn rebuild_presets_in_place(menu: &mut MenuState) {
    let new_items = build_presets_menu(&menu.presets, menu.active_preset);
    if let Some((items, active)) = menu.stack.last_mut() {
        *active = (*active).min(new_items.len().saturating_sub(1));
        *items = new_items;
    }
}

/// Update the "advanced settings" and "presets" submenus in the parent game rules level.
fn update_advanced_in_parent(menu: &mut MenuState, adv: &AdvancedConfig) {
    let depth = menu.stack.len();
    if depth >= 2 {
        let parent = &mut menu.stack[depth - 2];
        for item in parent.0.iter_mut() {
            if item.label == "advanced settings" {
                item.kind = MenuItemKind::SubMenu {
                    items: build_advanced_menu(adv),
                };
            }
            if item.label == "presets" {
                item.kind = MenuItemKind::SubMenu {
                    items: build_presets_menu(&menu.presets, menu.active_preset),
                };
            }
        }
    }
}

/// Rebuild the advanced settings menus on the stack after a reset.
fn rebuild_advanced_on_stack(menu: &mut MenuState, adv: &AdvancedConfig) {
    // Pop back one level (from advanced menu)
    if menu.stack.len() > 1 {
        menu.stack.pop();
    }
    // Update the "advanced settings" submenu in game rules
    if let Some((items, _)) = menu.stack.last_mut() {
        for item in items.iter_mut() {
            if item.label == "advanced settings" {
                item.kind = MenuItemKind::SubMenu {
                    items: build_advanced_menu(adv),
                };
            }
        }
    }
}

// ============================================================================
// Rendering
// ============================================================================

/// Draw the menu screen.
pub fn draw_menu(
    r: &mut Renderer,
    menu: &mut MenuState,
    font: &BitmapFont,
    textures: &[u32],
    w: u32,
    h: u32,
) {
    r.clear_color(0.0, 0.0, 0.0, 1.0);
    r.clear(renderer::COLOR_BUFFER_BIT | renderer::DEPTH_BUFFER_BIT);

    render::begin_2d(r, w, h);

    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // Background — cover the screen while preserving texture aspect ratio
    if TEX_GUI < textures.len() && textures[TEX_GUI] != 0 {
        r.set_texture(Some(textures[TEX_GUI]));
        // Query actual texture dimensions to compute correct aspect ratio
        let (tw, th) = r.get_tex_dimensions(textures[TEX_GUI]);
        let tw = tw.max(1);
        let th = th.max(1);
        let tex_aspect = tw as f32 / th as f32;
        let screen_aspect = w as f32 / h as f32;
        let (u0, u1, v0, v1) = if screen_aspect > tex_aspect {
            let vis = tex_aspect / screen_aspect;
            let margin = (1.0 - vis) / 2.0;
            (0.0, 1.0, margin, 1.0 - margin)
        } else {
            let vis = screen_aspect / tex_aspect;
            let margin = (1.0 - vis) / 2.0;
            (margin, 1.0 - margin, 0.0, 1.0)
        };
        let wf = w as f32;
        let hf = h as f32;
        let verts = textured_quad_verts(
            [[0.0, 0.0, 0.0], [wf, 0.0, 0.0], [wf, hf, 0.0], [0.0, hf, 0.0]],
            [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
            [1.0, 1.0, 1.0, 1.0],
        );
        r.draw_triangles(&verts);
    }

    // Logo (upper-right)
    if TEX_LOGO < textures.len() && textures[TEX_LOGO] != 0 {
        r.set_texture(Some(textures[TEX_LOGO]));
        let logo_w = w as f32 * 0.60;
        let logo_h = logo_w * (818.0 / 1500.0);
        let lx = w as f32 - logo_w - w as f32 * 0.05;
        let ly = h as f32 - logo_h + h as f32 * 0.13;
        let verts = textured_quad_verts(
            [[lx, ly, 0.0], [lx + logo_w, ly, 0.0], [lx + logo_w, ly + logo_h, 0.0], [lx, ly + logo_h, 0.0]],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            [1.0, 1.0, 1.0, 0.9],
        );
        r.draw_triangles(&verts);

        // Version number below the logo (right-aligned under "RS")
        let ver_text = format!("v{}", BUILD_VERSION);
        let ver_size = logo_h * 0.045;
        let ver_w = ver_text.len() as f32 * ver_size * 0.72;
        let ver_x = lx + logo_w - ver_w - logo_w * 0.06;
        let ver_y = ly + logo_h * 0.30;
        assets::draw_text(r, font, ver_x, ver_y, ver_size, &ver_text, [0.6, 0.8, 1.0, 0.6]);
    }

    r.set_texture(None);
    r.disable_blend();

    // If credits screen is active, draw it instead of menu items
    if menu.credits.is_some() {
        draw_credits_content(r, menu, font, w, h);
        render::end_2d(r);
        return;
    }

    // Menu text layout (matching original: 8% from left, 40% from top)
    let (items, active_idx) = menu.stack.last().unwrap();
    let menu_x = w as f32 * 0.08;
    let menu_top = h as f32 * 0.60;
    let menu_height = h as f32 * 0.45;
    let n = items.len().max(1);
    let item_height = (menu_height / n as f32).min(h as f32 * 0.08);
    let font_size = item_height * 0.75;

    // Animated highlight: oscillate between green (0.5, 1, 0) and red (1, 0, 0)
    let time = menu_time(menu.elapsed_ms);
    let t = (time * std::f32::consts::PI / 2048.0).sin() * 0.5 + 0.5;
    let active_color = [
        0.5 * t + 1.0 * (1.0 - t),
        1.0 * t + 0.0 * (1.0 - t),
        0.0,
        1.0,
    ];

    for (i, item) in items.iter().enumerate() {
        let y = menu_top - i as f32 * item_height * 1.3;
        let color = if i == *active_idx { active_color } else { [1.0, 1.0, 1.0, 1.0] };

        let text = format_item(item);
        if matches!(item.kind, MenuItemKind::Label(_)) {
            // Hint/label items: smaller, dimmer text
            let hint_size = font_size * 0.6;
            let hint_color = [0.5, 0.5, 0.5, 0.7];
            assets::draw_text(r, font, menu_x, y, hint_size, &text, hint_color);
        } else {
            assets::draw_text_shadowed(r, font, menu_x, y, font_size, &text, color);
        }

        // Draw slider bar for slider items (below the text)
        if let MenuItemKind::Slider { field_idx, value } = &item.kind {
            draw_slider_bar(r, menu_x, y, font_size, w, *field_idx, *value, i == *active_idx, active_color);
        }

        // Draw player toggle slots with colors
        if let MenuItemKind::PlayerToggle { enabled, cursor } = &item.kind {
            draw_player_toggle(r, font, menu_x, y, font_size, enabled, *cursor, i == *active_idx);
        }

        // Draw controls selector P1-P4 (only when focused)
        if let MenuItemKind::ControlsSelect { cursor } = &item.kind {
            if i == *active_idx {
                draw_controls_select(r, font, menu_x, y, font_size, *cursor, true);
            }
        }
    }

    // Draw naming overlay if active
    if let Some(ref name) = menu.naming {
        draw_naming_overlay(r, font, w, h, name, menu.elapsed_ms);
    }

    // Draw key capture overlay if active
    if menu.capturing_key.is_some() {
        draw_capture_overlay(r, font, w, h, menu.elapsed_ms);
    }

    render::end_2d(r);
}

/// Draw the credits screen content (called from draw_menu when credits are active).
fn draw_credits_content(r: &mut Renderer, menu: &mut MenuState, font: &BitmapFont, w: u32, h: u32) {
    let wf = w as f32;
    let hf = h as f32;

    // Semi-transparent overlay to darken the background
    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);
    let dim = quad_verts(
        [[0.0, 0.0, 0.0], [wf, 0.0, 0.0], [wf, hf, 0.0], [0.0, hf, 0.0]],
        [0.0, 0.0, 0.05, 0.75],
    );
    r.draw_triangles(&dim);

    let credits = menu.credits.as_mut().unwrap();

    let margin_x = wf * 0.08;
    let content_top = hf * 0.88;
    let content_bottom = hf * 0.08;

    let mut y = content_top + credits.scroll_offset;
    let link_color: [f32; 4] = [0.3, 0.55, 1.0, 1.0];

    // Collect URL regions this frame
    let mut url_regions = Vec::new();

    for line in &credits.lines {
        let (font_sz, line_spacing, base_color): (f32, f32, [f32; 4]) = match line.style {
            CreditsStyle::Header => (hf * 0.050, 1.8, [0.5, 0.9, 1.0, 1.0]),
            CreditsStyle::SubHeader => (hf * 0.035, 1.6, [0.3, 1.0, 0.6, 1.0]),
            CreditsStyle::Normal => (hf * 0.026, 1.4, [0.85, 0.85, 0.9, 0.9]),
            CreditsStyle::Blank => {
                y -= hf * 0.018;
                continue;
            }
            CreditsStyle::Separator => {
                // Draw a thin horizontal line.  Text below extends upward by
                // its font_sz, so the line must sit in the upper portion of
                // the gap to appear visually centred between content.
                let gap = hf * 0.12;
                let sep_y = y - gap * 0.24;
                if sep_y < content_top + hf * 0.05 && sep_y > content_bottom {
                    let sep_color = [0.3, 0.5, 0.6, 0.3];
                    let line_w = wf * 0.45;
                    let thick = (hf * 0.0015).max(1.0);
                    let verts = quad_verts(
                        [[margin_x, sep_y, 0.0], [margin_x + line_w, sep_y, 0.0],
                         [margin_x + line_w, sep_y + thick, 0.0], [margin_x, sep_y + thick, 0.0]],
                        sep_color,
                    );
                    r.draw_triangles(&verts);
                }
                y -= gap;
                continue;
            }
        };

        let has_url = line.url.is_some();

        // Skip lines outside visible area
        if y > content_top + font_sz * 2.0 || y < content_bottom - font_sz {
            y -= font_sz * line_spacing;
            continue;
        }

        // Render spans
        let mut cx = margin_x;
        for span in &line.spans {
            let color = if span.is_url {
                link_color
            } else if span.highlighted {
                [0.3, 1.0, 0.85, 1.0]  // teal
            } else if span.accent {
                [1.0, 0.85, 0.3, 1.0]  // gold
            } else {
                base_color
            };
            // URLs: draw twice with slight offset for faux-bold effect
            if span.is_url {
                let bold_offset = (font_sz * 0.04).max(0.5);
                assets::draw_text_shadowed(r, font, cx + bold_offset, y, font_sz, &span.text, color);
            }
            assets::draw_text_shadowed(r, font, cx, y, font_sz, &span.text, color);
            let span_w = span.text.len() as f32 * font_sz * 0.72;

            // Track URL hit region for clickable links
            if span.is_url {
                url_regions.push(UrlHitRegion {
                    url: span.text.clone(),
                    x: cx,
                    y,
                    w: span_w,
                    h: font_sz,
                });
            }
            cx += span_w;
        }

        y -= font_sz * line_spacing;
    }

    // Store URL regions for click detection
    credits.url_regions = url_regions;

    // Track total content height for scroll clamping
    let used_height = content_top + credits.scroll_offset - y;
    let visible_height = content_top - content_bottom;
    if used_height > visible_height {
        credits.total_height = used_height - visible_height;
    }

    // Hint bar at bottom
    let hint_size = hf * 0.025;
    let hint = "up/down: scroll | click links to open | esc: back";
    let hint_w = hint.len() as f32 * hint_size * 0.72;
    let hint_x = (wf - hint_w) / 2.0;
    let hint_y = hf * 0.02;
    assets::draw_text_shadowed(r, font, hint_x, hint_y, hint_size, hint, [0.6, 0.7, 0.8, 0.85]);

    r.disable_blend();
}

/// Draw the preset naming overlay (centered text input box).
fn draw_naming_overlay(r: &mut Renderer, font: &BitmapFont, w: u32, h: u32, name: &str, elapsed_ms: u32) {
    let wf = w as f32;
    let hf = h as f32;

    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // Dim background overlay
    let dim = quad_verts(
        [[0.0, 0.0, 0.0], [wf, 0.0, 0.0], [wf, hf, 0.0], [0.0, hf, 0.0]],
        [0.0, 0.0, 0.0, 0.6],
    );
    r.draw_triangles(&dim);

    // Input box
    let box_w = wf * 0.5;
    let box_h = hf * 0.12;
    let box_x = (wf - box_w) / 2.0;
    let box_y = hf * 0.45;

    // Box background
    let bg = quad_verts(
        [[box_x, box_y, 0.0], [box_x + box_w, box_y, 0.0],
         [box_x + box_w, box_y + box_h, 0.0], [box_x, box_y + box_h, 0.0]],
        [0.08, 0.08, 0.18, 0.9],
    );
    r.draw_triangles(&bg);

    // Box border
    r.set_line_smooth(true);
    r.line_width(1.5);
    let border_color = [0.4, 0.5, 0.8, 0.7];
    let border_pts = [
        Vertex::pos_color_2d(box_x, box_y, border_color),
        Vertex::pos_color_2d(box_x + box_w, box_y, border_color),
        Vertex::pos_color_2d(box_x + box_w, box_y + box_h, border_color),
        Vertex::pos_color_2d(box_x, box_y + box_h, border_color),
    ];
    r.draw_lines(&line_loop_verts(&border_pts));
    r.set_line_smooth(false);
    r.disable_blend();

    // Prompt text
    let prompt_size = hf * 0.028;
    let prompt = "enter preset name (enter to save, esc to cancel)";
    let prompt_w = prompt.len() as f32 * prompt_size * 0.72;
    let px = (wf - prompt_w) / 2.0;
    let py = hf * 0.45 + hf * 0.12 + prompt_size * 0.5;
    assets::draw_text_shadowed(r, font, px, py, prompt_size, prompt, [0.6, 0.6, 0.8, 0.8]);

    // Name text with blinking cursor
    let name_size = hf * 0.045;
    let blink = (elapsed_ms / 400) % 2 == 0;
    let display = if blink {
        format!("{}|", name)
    } else {
        format!("{} ", name)
    };
    let name_w = display.len() as f32 * name_size * 0.72;
    let nx = (wf - name_w) / 2.0;
    let ny = hf * 0.45 + hf * 0.04;
    assets::draw_text_shadowed(r, font, nx, ny, name_size, &display, [1.0, 1.0, 1.0, 1.0]);
}

/// Player slot labels (short names for the toggle display).
const PLAYER_SLOT_LABELS: [&str; 4] = ["P1", "P2", "P3", "P4"];

/// Draw the [P1] [P2] [P3] [P4] toggle display with player colors (text only).
fn draw_player_toggle(
    r: &mut Renderer,
    font: &BitmapFont,
    menu_x: f32,
    y: f32,
    font_size: f32,
    enabled: &[bool; 4],
    cursor: usize,
    is_active_item: bool,
) {
    let char_w = font_size * 0.72;
    // Position slots after the label text ("human players" = 13 chars + some padding)
    let label_len = 15.0;
    let slot_start_x = menu_x + label_len * char_w;
    let slot_w = char_w * 5.0;

    for slot in 0..4 {
        let sx = slot_start_x + slot as f32 * slot_w;
        let pc = MODEL_DIFFUSE[slot];

        let is_cursor = is_active_item && cursor == slot;

        let text_color = if enabled[slot] {
            if is_cursor {
                // Brighten the player color to indicate cursor
                let brighten = |c: f32| (c * 1.4).min(1.0);
                [brighten(pc[0]), brighten(pc[1]), brighten(pc[2]), 1.0]
            } else {
                [pc[0], pc[1], pc[2], 1.0]
            }
        } else if is_cursor {
            [0.5, 0.5, 0.5, 0.9]
        } else {
            [0.25, 0.25, 0.25, 0.45]
        };

        let label = PLAYER_SLOT_LABELS[slot];
        let bracket_text = format!("[{}]", label);
        assets::draw_text(r, font, sx, y, font_size, &bracket_text, text_color);
    }
}

/// Draw the [P1] [P2] [P3] [P4] controls selector with player colors.
fn draw_controls_select(
    r: &mut Renderer,
    font: &BitmapFont,
    menu_x: f32,
    y: f32,
    font_size: f32,
    cursor: usize,
    is_active_item: bool,
) {
    let char_w = font_size * 0.72;
    // Position slots after "controls" label (8 chars + padding)
    let label_len = 15.0;
    let slot_start_x = menu_x + label_len * char_w;
    let slot_w = char_w * 5.0;

    for slot in 0..4 {
        let sx = slot_start_x + slot as f32 * slot_w;
        let pc = MODEL_DIFFUSE[slot];
        let is_cursor = is_active_item && cursor == slot;

        let text_color = if is_cursor {
            let brighten = |c: f32| (c * 1.4).min(1.0);
            [brighten(pc[0]), brighten(pc[1]), brighten(pc[2]), 1.0]
        } else {
            [pc[0], pc[1], pc[2], 0.7]
        };

        let label = PLAYER_SLOT_LABELS[slot];
        let bracket_text = format!("[{}]", label);
        assets::draw_text(r, font, sx, y, font_size, &bracket_text, text_color);
    }
}

/// Draw the "press a key..." capture overlay.
fn draw_capture_overlay(r: &mut Renderer, font: &BitmapFont, w: u32, h: u32, elapsed_ms: u32) {
    let wf = w as f32;
    let hf = h as f32;

    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // Dim background overlay
    let dim = quad_verts(
        [[0.0, 0.0, 0.0], [wf, 0.0, 0.0], [wf, hf, 0.0], [0.0, hf, 0.0]],
        [0.0, 0.0, 0.0, 0.6],
    );
    r.draw_triangles(&dim);

    // Box
    let box_w = wf * 0.4;
    let box_h = hf * 0.10;
    let box_x = (wf - box_w) / 2.0;
    let box_y = hf * 0.45;

    let bg = quad_verts(
        [[box_x, box_y, 0.0], [box_x + box_w, box_y, 0.0],
         [box_x + box_w, box_y + box_h, 0.0], [box_x, box_y + box_h, 0.0]],
        [0.08, 0.08, 0.18, 0.9],
    );
    r.draw_triangles(&bg);

    // Border
    r.set_line_smooth(true);
    r.line_width(1.5);
    let bc = [0.4, 0.5, 0.8, 0.7];
    let border_pts = [
        Vertex::pos_color_2d(box_x, box_y, bc),
        Vertex::pos_color_2d(box_x + box_w, box_y, bc),
        Vertex::pos_color_2d(box_x + box_w, box_y + box_h, bc),
        Vertex::pos_color_2d(box_x, box_y + box_h, bc),
    ];
    r.draw_lines(&line_loop_verts(&border_pts));
    r.set_line_smooth(false);
    r.disable_blend();

    // Prompt text with blinking dots
    let text_size = hf * 0.04;
    let blink = (elapsed_ms / 500) % 2 == 0;
    let prompt = if blink { "press a key..." } else { "press a key" };
    let tw = prompt.len() as f32 * text_size * 0.72;
    let tx = (wf - tw) / 2.0;
    let ty = box_y + box_h * 0.3;
    assets::draw_text_shadowed(r, font, tx, ty, text_size, prompt, [1.0, 1.0, 1.0, 1.0]);

    // Hint
    let hint_size = hf * 0.025;
    let hint = "esc to cancel";
    let hw = hint.len() as f32 * hint_size * 0.72;
    let hx = (wf - hw) / 2.0;
    let hy = box_y - hint_size * 1.5;
    assets::draw_text_shadowed(r, font, hx, hy, hint_size, hint, [0.6, 0.6, 0.8, 0.8]);
}

/// Draw a slider bar directly below the label text.
fn draw_slider_bar(r: &mut Renderer, menu_x: f32, y: f32, font_size: f32, w: u32, field_idx: usize, value: f32, active: bool, active_color: [f32; 4]) {
    let field = &ADVANCED_FIELDS[field_idx];
    let frac = ((value - field.min) / (field.max - field.min)).clamp(0.0, 1.0);

    // Position: below the text, same left edge, spanning to ~85% of screen
    let bar_x = menu_x;
    let bar_w = w as f32 * 0.85 - menu_x;
    let bar_h = font_size * 0.25;
    let bar_y = y - font_size * 0.35;

    r.set_blend(renderer::SRC_ALPHA, renderer::ONE_MINUS_SRC_ALPHA);

    // Background track
    let track = quad_verts(
        [[bar_x, bar_y, 0.0], [bar_x + bar_w, bar_y, 0.0],
         [bar_x + bar_w, bar_y + bar_h, 0.0], [bar_x, bar_y + bar_h, 0.0]],
        [0.2, 0.2, 0.3, 0.5],
    );
    r.draw_triangles(&track);

    // Fill
    let fill_w = bar_w * frac;
    if fill_w > 0.5 {
        let col = if active { active_color } else { [0.4, 0.7, 0.4, 0.7] };
        let fill_color = [col[0], col[1], col[2], col[3] * 0.7];
        let fill = quad_verts(
            [[bar_x, bar_y, 0.0], [bar_x + fill_w, bar_y, 0.0],
             [bar_x + fill_w, bar_y + bar_h, 0.0], [bar_x, bar_y + bar_h, 0.0]],
            fill_color,
        );
        r.draw_triangles(&fill);
    }

    // Border
    r.set_line_smooth(true);
    r.line_width(1.0);
    let bc = [0.5, 0.5, 0.6, 0.5];
    let border_pts = [
        Vertex::pos_color_2d(bar_x, bar_y, bc),
        Vertex::pos_color_2d(bar_x + bar_w, bar_y, bc),
        Vertex::pos_color_2d(bar_x + bar_w, bar_y + bar_h, bc),
        Vertex::pos_color_2d(bar_x, bar_y + bar_h, bc),
    ];
    r.draw_lines(&line_loop_verts(&border_pts));
    r.set_line_smooth(false);
    r.disable_blend();
}

fn menu_time(elapsed_ms: u32) -> f32 {
    (elapsed_ms & 4095) as f32
}

fn format_item(item: &MenuItem) -> String {
    match &item.kind {
        MenuItemKind::List { options, current, .. } => {
            format!("{}: {}", item.label, options[*current])
        }
        MenuItemKind::Slider { field_idx, value } => {
            let field = &ADVANCED_FIELDS[*field_idx];
            match field.decimals {
                0 => format!("{}: {:.0}", item.label, value),
                1 => format!("{}: {:.1}", item.label, value),
                _ => format!("{}: {:.2}", item.label, value),
            }
        }
        MenuItemKind::KeyBind { current_name, .. } => {
            format!("{}: [{}]", item.label, current_name)
        }
        MenuItemKind::PlayerToggle { .. } => {
            // Label only — colored slots drawn separately by draw_menu
            item.label.clone()
        }
        MenuItemKind::ControlsSelect { .. } => {
            // Label only — P1-P4 slots drawn separately when expanded
            item.label.clone()
        }
        _ => item.label.clone(),
    }
}
