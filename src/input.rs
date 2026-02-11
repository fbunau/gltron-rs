use winit::keyboard::KeyCode;
use crate::types::*;
use crate::game;
use crate::camera;
use crate::gamepad::GamepadBtn;

/// Glance angle in radians (90 degrees).
pub const GLANCE_ANGLE: f32 = std::f32::consts::FRAC_PI_2;

pub const MAX_KEY_PLAYERS: usize = 4;

// ============================================================================
// InputBinding — keyboard key or gamepad button
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBinding {
    Key(KeyCode),
    Pad(GamepadBtn),
}

impl InputBinding {
    pub fn display_name(self) -> String {
        match self {
            InputBinding::Key(kc) => keycode_display_name(kc),
            InputBinding::Pad(btn) => btn.display_name().into(),
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        if s.starts_with("gp:") {
            GamepadBtn::from_name(s).map(InputBinding::Pad)
        } else {
            keycode_from_name(s).map(InputBinding::Key)
        }
    }
}

// ============================================================================
// Player actions & key bindings
// ============================================================================

/// The six rebindable actions per player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    TurnLeft,
    TurnRight,
    GlanceLeft,
    GlanceRight,
    Boost,
    Scoreboard,
}

pub const ALL_ACTIONS: [PlayerAction; 6] = [
    PlayerAction::TurnLeft,
    PlayerAction::TurnRight,
    PlayerAction::GlanceLeft,
    PlayerAction::GlanceRight,
    PlayerAction::Boost,
    PlayerAction::Scoreboard,
];

impl PlayerAction {
    pub fn label(self) -> &'static str {
        match self {
            PlayerAction::TurnLeft => "turn left",
            PlayerAction::TurnRight => "turn right",
            PlayerAction::GlanceLeft => "glance left",
            PlayerAction::GlanceRight => "glance right",
            PlayerAction::Boost => "booster",
            PlayerAction::Scoreboard => "scoreboard",
        }
    }

    fn key_name(self) -> &'static str {
        match self {
            PlayerAction::TurnLeft => "turn_left",
            PlayerAction::TurnRight => "turn_right",
            PlayerAction::GlanceLeft => "glance_left",
            PlayerAction::GlanceRight => "glance_right",
            PlayerAction::Boost => "boost",
            PlayerAction::Scoreboard => "scoreboard",
        }
    }

    fn from_key_name(s: &str) -> Option<Self> {
        match s {
            "turn_left" => Some(PlayerAction::TurnLeft),
            "turn_right" => Some(PlayerAction::TurnRight),
            "glance_left" => Some(PlayerAction::GlanceLeft),
            "glance_right" => Some(PlayerAction::GlanceRight),
            "boost" => Some(PlayerAction::Boost),
            "scoreboard" => Some(PlayerAction::Scoreboard),
            _ => None,
        }
    }
}

/// Bindings for a single player (keyboard keys or gamepad buttons).
#[derive(Debug, Clone)]
pub struct PlayerKeys {
    pub left: InputBinding,
    pub right: InputBinding,
    pub glance_left: InputBinding,
    pub glance_right: InputBinding,
    pub boost: InputBinding,
    pub scoreboard: InputBinding,
}

impl PlayerKeys {
    pub fn get(&self, action: PlayerAction) -> InputBinding {
        match action {
            PlayerAction::TurnLeft => self.left,
            PlayerAction::TurnRight => self.right,
            PlayerAction::GlanceLeft => self.glance_left,
            PlayerAction::GlanceRight => self.glance_right,
            PlayerAction::Boost => self.boost,
            PlayerAction::Scoreboard => self.scoreboard,
        }
    }

    pub fn set(&mut self, action: PlayerAction, binding: InputBinding) {
        match action {
            PlayerAction::TurnLeft => self.left = binding,
            PlayerAction::TurnRight => self.right = binding,
            PlayerAction::GlanceLeft => self.glance_left = binding,
            PlayerAction::GlanceRight => self.glance_right = binding,
            PlayerAction::Boost => self.boost = binding,
            PlayerAction::Scoreboard => self.scoreboard = binding,
        }
    }
}

/// All key bindings for up to 4 players.
#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub players: [PlayerKeys; MAX_KEY_PLAYERS],
}

impl Default for KeyBindings {
    fn default() -> Self {
        use InputBinding::Key;
        Self {
            players: [
                PlayerKeys { left: Key(KeyCode::ArrowLeft), right: Key(KeyCode::ArrowRight), glance_left: Key(KeyCode::Delete), glance_right: Key(KeyCode::End), boost: Key(KeyCode::Space), scoreboard: Key(KeyCode::Tab) },
                PlayerKeys { left: Key(KeyCode::KeyJ), right: Key(KeyCode::KeyK), glance_left: Key(KeyCode::KeyU), glance_right: Key(KeyCode::KeyI), boost: Key(KeyCode::KeyL), scoreboard: Key(KeyCode::KeyO) },
                PlayerKeys { left: Key(KeyCode::KeyA), right: Key(KeyCode::KeyS), glance_left: Key(KeyCode::KeyQ), glance_right: Key(KeyCode::KeyW), boost: Key(KeyCode::KeyE), scoreboard: Key(KeyCode::PageDown) },
                PlayerKeys { left: Key(KeyCode::Numpad4), right: Key(KeyCode::Numpad6), glance_left: Key(KeyCode::Numpad7), glance_right: Key(KeyCode::Numpad9), boost: Key(KeyCode::Numpad5), scoreboard: Key(KeyCode::Numpad0) },
            ],
        }
    }
}

// ============================================================================
// KeyCode display name <-> parse
// ============================================================================

pub fn keycode_display_name(kc: KeyCode) -> String {
    match kc {
        KeyCode::KeyA => "a".into(),
        KeyCode::KeyB => "b".into(),
        KeyCode::KeyC => "c".into(),
        KeyCode::KeyD => "d".into(),
        KeyCode::KeyE => "e".into(),
        KeyCode::KeyF => "f".into(),
        KeyCode::KeyG => "g".into(),
        KeyCode::KeyH => "h".into(),
        KeyCode::KeyI => "i".into(),
        KeyCode::KeyJ => "j".into(),
        KeyCode::KeyK => "k".into(),
        KeyCode::KeyL => "l".into(),
        KeyCode::KeyM => "m".into(),
        KeyCode::KeyN => "n".into(),
        KeyCode::KeyO => "o".into(),
        KeyCode::KeyP => "p".into(),
        KeyCode::KeyQ => "q".into(),
        KeyCode::KeyR => "r".into(),
        KeyCode::KeyS => "s".into(),
        KeyCode::KeyT => "t".into(),
        KeyCode::KeyU => "u".into(),
        KeyCode::KeyV => "v".into(),
        KeyCode::KeyW => "w".into(),
        KeyCode::KeyX => "x".into(),
        KeyCode::KeyY => "y".into(),
        KeyCode::KeyZ => "z".into(),
        KeyCode::Digit0 => "0".into(),
        KeyCode::Digit1 => "1".into(),
        KeyCode::Digit2 => "2".into(),
        KeyCode::Digit3 => "3".into(),
        KeyCode::Digit4 => "4".into(),
        KeyCode::Digit5 => "5".into(),
        KeyCode::Digit6 => "6".into(),
        KeyCode::Digit7 => "7".into(),
        KeyCode::Digit8 => "8".into(),
        KeyCode::Digit9 => "9".into(),
        KeyCode::ArrowLeft => "left".into(),
        KeyCode::ArrowRight => "right".into(),
        KeyCode::ArrowUp => "up".into(),
        KeyCode::ArrowDown => "down".into(),
        KeyCode::Space => "space".into(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Backspace => "bksp".into(),
        KeyCode::Delete => "del".into(),
        KeyCode::Insert => "ins".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pgup".into(),
        KeyCode::PageDown => "pgdn".into(),
        KeyCode::Escape => "esc".into(),
        KeyCode::ShiftLeft => "lshift".into(),
        KeyCode::ShiftRight => "rshift".into(),
        KeyCode::ControlLeft => "lctrl".into(),
        KeyCode::ControlRight => "rctrl".into(),
        KeyCode::AltLeft => "lalt".into(),
        KeyCode::AltRight => "ralt".into(),
        KeyCode::Numpad0 => "kp0".into(),
        KeyCode::Numpad1 => "kp1".into(),
        KeyCode::Numpad2 => "kp2".into(),
        KeyCode::Numpad3 => "kp3".into(),
        KeyCode::Numpad4 => "kp4".into(),
        KeyCode::Numpad5 => "kp5".into(),
        KeyCode::Numpad6 => "kp6".into(),
        KeyCode::Numpad7 => "kp7".into(),
        KeyCode::Numpad8 => "kp8".into(),
        KeyCode::Numpad9 => "kp9".into(),
        KeyCode::NumpadAdd => "kp+".into(),
        KeyCode::NumpadSubtract => "kp-".into(),
        KeyCode::NumpadMultiply => "kp*".into(),
        KeyCode::NumpadDivide => "kp/".into(),
        KeyCode::NumpadEnter => "kpenter".into(),
        KeyCode::NumpadDecimal => "kp.".into(),
        KeyCode::F1 => "f1".into(),
        KeyCode::F2 => "f2".into(),
        KeyCode::F3 => "f3".into(),
        KeyCode::F4 => "f4".into(),
        KeyCode::F5 => "f5".into(),
        KeyCode::F6 => "f6".into(),
        KeyCode::F7 => "f7".into(),
        KeyCode::F8 => "f8".into(),
        KeyCode::F9 => "f9".into(),
        KeyCode::F10 => "f10".into(),
        KeyCode::F11 => "f11".into(),
        KeyCode::F12 => "f12".into(),
        KeyCode::Comma => ",".into(),
        KeyCode::Period => ".".into(),
        KeyCode::Slash => "/".into(),
        KeyCode::Backslash => "\\".into(),
        KeyCode::Semicolon => ";".into(),
        KeyCode::Quote => "'".into(),
        KeyCode::BracketLeft => "[".into(),
        KeyCode::BracketRight => "]".into(),
        KeyCode::Minus => "-".into(),
        KeyCode::Equal => "=".into(),
        KeyCode::Backquote => "`".into(),
        KeyCode::CapsLock => "caps".into(),
        other => format!("{:?}", other),
    }
}

/// Parse a key name (as produced by keycode_display_name) back to a KeyCode.
/// Also accepts the Debug format (e.g. "KeyA") as fallback.
pub fn keycode_from_name(s: &str) -> Option<KeyCode> {
    match s {
        "a" => Some(KeyCode::KeyA),
        "b" => Some(KeyCode::KeyB),
        "c" => Some(KeyCode::KeyC),
        "d" => Some(KeyCode::KeyD),
        "e" => Some(KeyCode::KeyE),
        "f" => Some(KeyCode::KeyF),
        "g" => Some(KeyCode::KeyG),
        "h" => Some(KeyCode::KeyH),
        "i" => Some(KeyCode::KeyI),
        "j" => Some(KeyCode::KeyJ),
        "k" => Some(KeyCode::KeyK),
        "l" => Some(KeyCode::KeyL),
        "m" => Some(KeyCode::KeyM),
        "n" => Some(KeyCode::KeyN),
        "o" => Some(KeyCode::KeyO),
        "p" => Some(KeyCode::KeyP),
        "q" => Some(KeyCode::KeyQ),
        "r" => Some(KeyCode::KeyR),
        "s" => Some(KeyCode::KeyS),
        "t" => Some(KeyCode::KeyT),
        "u" => Some(KeyCode::KeyU),
        "v" => Some(KeyCode::KeyV),
        "w" => Some(KeyCode::KeyW),
        "x" => Some(KeyCode::KeyX),
        "y" => Some(KeyCode::KeyY),
        "z" => Some(KeyCode::KeyZ),
        "0" => Some(KeyCode::Digit0),
        "1" => Some(KeyCode::Digit1),
        "2" => Some(KeyCode::Digit2),
        "3" => Some(KeyCode::Digit3),
        "4" => Some(KeyCode::Digit4),
        "5" => Some(KeyCode::Digit5),
        "6" => Some(KeyCode::Digit6),
        "7" => Some(KeyCode::Digit7),
        "8" => Some(KeyCode::Digit8),
        "9" => Some(KeyCode::Digit9),
        "left" => Some(KeyCode::ArrowLeft),
        "right" => Some(KeyCode::ArrowRight),
        "up" => Some(KeyCode::ArrowUp),
        "down" => Some(KeyCode::ArrowDown),
        "space" => Some(KeyCode::Space),
        "enter" => Some(KeyCode::Enter),
        "tab" => Some(KeyCode::Tab),
        "bksp" => Some(KeyCode::Backspace),
        "del" => Some(KeyCode::Delete),
        "ins" => Some(KeyCode::Insert),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pgup" => Some(KeyCode::PageUp),
        "pgdn" => Some(KeyCode::PageDown),
        "esc" => Some(KeyCode::Escape),
        "lshift" => Some(KeyCode::ShiftLeft),
        "rshift" => Some(KeyCode::ShiftRight),
        "lctrl" => Some(KeyCode::ControlLeft),
        "rctrl" => Some(KeyCode::ControlRight),
        "lalt" => Some(KeyCode::AltLeft),
        "ralt" => Some(KeyCode::AltRight),
        "kp0" => Some(KeyCode::Numpad0),
        "kp1" => Some(KeyCode::Numpad1),
        "kp2" => Some(KeyCode::Numpad2),
        "kp3" => Some(KeyCode::Numpad3),
        "kp4" => Some(KeyCode::Numpad4),
        "kp5" => Some(KeyCode::Numpad5),
        "kp6" => Some(KeyCode::Numpad6),
        "kp7" => Some(KeyCode::Numpad7),
        "kp8" => Some(KeyCode::Numpad8),
        "kp9" => Some(KeyCode::Numpad9),
        "kp+" => Some(KeyCode::NumpadAdd),
        "kp-" => Some(KeyCode::NumpadSubtract),
        "kp*" => Some(KeyCode::NumpadMultiply),
        "kp/" => Some(KeyCode::NumpadDivide),
        "kpenter" => Some(KeyCode::NumpadEnter),
        "kp." => Some(KeyCode::NumpadDecimal),
        "f1" => Some(KeyCode::F1),
        "f2" => Some(KeyCode::F2),
        "f3" => Some(KeyCode::F3),
        "f4" => Some(KeyCode::F4),
        "f5" => Some(KeyCode::F5),
        "f6" => Some(KeyCode::F6),
        "f7" => Some(KeyCode::F7),
        "f8" => Some(KeyCode::F8),
        "f9" => Some(KeyCode::F9),
        "f10" => Some(KeyCode::F10),
        "f11" => Some(KeyCode::F11),
        "f12" => Some(KeyCode::F12),
        "," => Some(KeyCode::Comma),
        "." => Some(KeyCode::Period),
        "/" => Some(KeyCode::Slash),
        "\\" => Some(KeyCode::Backslash),
        ";" => Some(KeyCode::Semicolon),
        "'" => Some(KeyCode::Quote),
        "[" => Some(KeyCode::BracketLeft),
        "]" => Some(KeyCode::BracketRight),
        "-" => Some(KeyCode::Minus),
        "=" => Some(KeyCode::Equal),
        "`" => Some(KeyCode::Backquote),
        "caps" => Some(KeyCode::CapsLock),
        _ => None,
    }
}

// ============================================================================
// Persistence
// ============================================================================

fn serialize_bindings(bindings: &KeyBindings) -> String {
    let mut s = String::new();
    for (p, keys) in bindings.players.iter().enumerate() {
        for &action in &ALL_ACTIONS {
            s.push_str(&format!("player{}.{}={}\n",
                p, action.key_name(), keys.get(action).display_name()));
        }
    }
    s
}

fn parse_bindings(text: &str) -> Option<KeyBindings> {
    let mut bindings = KeyBindings::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let Some((key, val)) = line.split_once('=') else { continue };
        // key = "player0.turn_left", val = "a" or "gp:A"
        let Some((player_part, action_part)) = key.split_once('.') else { continue };
        let player_idx: usize = player_part.strip_prefix("player")?.parse().ok()?;
        if player_idx >= MAX_KEY_PLAYERS { continue; }
        let Some(action) = PlayerAction::from_key_name(action_part) else { continue };
        let Some(binding) = InputBinding::from_name(val) else { continue };
        bindings.players[player_idx].set(action, binding);
    }
    Some(bindings)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_keybindings(bindings: &KeyBindings) {
    let text = serialize_bindings(bindings);
    std::fs::write("keybindings.txt", text).ok();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_keybindings() -> Option<KeyBindings> {
    let text = std::fs::read_to_string("keybindings.txt").ok()?;
    parse_bindings(&text)
}

#[cfg(target_arch = "wasm32")]
fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn save_keybindings(bindings: &KeyBindings) {
    if let Some(storage) = get_local_storage() {
        let text = serialize_bindings(bindings);
        storage.set_item("gltron_keybindings", &text).ok();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_keybindings() -> Option<KeyBindings> {
    let storage = get_local_storage()?;
    let text = storage.get_item("gltron_keybindings").ok()??;
    parse_bindings(&text)
}

// ============================================================================
// Input handling
// ============================================================================

/// Process a key press event.
pub fn handle_key_down(state: &mut GameState, settings: &mut Settings, bindings: &KeyBindings, keycode: KeyCode) -> bool {
    // System keys
    match keycode {
        KeyCode::F10 => {
            // Cycle camera for first human player
            if let Some(hi) = state.first_human() {
                let data = state.players[hi].data.clone();
                camera::next_camera_type(&mut state.players[hi].camera, &data);
            }
            return false;
        }
        _ => {}
    }

    let kb = InputBinding::Key(keycode);

    // Player keys
    for (i, keys) in bindings.players.iter().enumerate() {
        if i >= state.players.len() || state.players[i].ai.kind != AiKind::Human {
            continue;
        }
        // Scoreboard hold-to-show per player (works alive or dead)
        if kb == keys.scoreboard {
            state.players[i].show_scoreboard = true;
            return false;
        }
        if state.players[i].data.speed <= 0.0 {
            // Dead human: glance keys cycle spectate targets
            if kb == keys.glance_left || kb == keys.left {
                cycle_spectate(state, i, false);
            } else if kb == keys.glance_right || kb == keys.right {
                cycle_spectate(state, i, true);
            } else if kb == keys.boost {
                // Fast forward only if ALL humans are dead
                let all_humans_dead = state.players.iter()
                    .filter(|p| p.ai.kind == AiKind::Human)
                    .all(|p| p.data.speed <= 0.0);
                if all_humans_dead {
                    state.fast_forward = true;
                }
            }
        } else {
            // Alive: normal controls
            if kb == keys.left {
                game::queue_turn(state, i, true);
            } else if kb == keys.right {
                game::queue_turn(state, i, false);
            } else if kb == keys.glance_left {
                state.players[i].camera.phi_offset = GLANCE_ANGLE;
            } else if kb == keys.glance_right {
                state.players[i].camera.phi_offset = -GLANCE_ANGLE;
            } else if kb == keys.boost {
                if state.players[i].data.speed > 0.0 {
                    state.players[i].data.boost_enabled = true;
                }
            }
        }
    }
    false
}

/// Process a key release event.
pub fn handle_key_up(state: &mut GameState, bindings: &KeyBindings, keycode: KeyCode) {
    let kb = InputBinding::Key(keycode);
    for (i, keys) in bindings.players.iter().enumerate() {
        if i >= state.players.len() {
            continue;
        }
        if kb == keys.boost {
            state.players[i].data.boost_enabled = false;
            state.fast_forward = false;
        } else if kb == keys.glance_left || kb == keys.glance_right {
            state.players[i].camera.phi_offset = 0.0;
        } else if kb == keys.scoreboard {
            state.players[i].show_scoreboard = false;
        }
    }
}

/// Cycle spectate target: None (recognizer) -> alive players -> back to None.
/// `forward` = true cycles right/next, false cycles left/prev.
fn cycle_spectate(state: &mut GameState, human_idx: usize, forward: bool) {
    // Build list of valid targets: None (recognizer), then alive non-human players
    let mut targets: Vec<Option<usize>> = vec![None];
    for (pi, p) in state.players.iter().enumerate() {
        if pi != human_idx && p.data.speed > 0.0 {
            targets.push(Some(pi));
        }
    }
    if targets.is_empty() {
        return;
    }

    let current = targets.iter().position(|t| *t == state.players[human_idx].spectate_target).unwrap_or(0);
    let next = if forward {
        (current + 1) % targets.len()
    } else {
        (current + targets.len() - 1) % targets.len()
    };
    state.players[human_idx].spectate_target = targets[next];
}

/// Check if a keycode is a scoreboard or glance key for any human player.
pub fn is_passive_key(state: &GameState, bindings: &KeyBindings, keycode: KeyCode) -> bool {
    let kb = InputBinding::Key(keycode);
    for (i, keys) in bindings.players.iter().enumerate() {
        if i < state.players.len() && state.players[i].ai.kind == AiKind::Human {
            if kb == keys.scoreboard || kb == keys.glance_left || kb == keys.glance_right {
                return true;
            }
        }
    }
    false
}

/// Check if a keycode is a boost key for any human player.
pub fn is_boost_key(state: &GameState, bindings: &KeyBindings, keycode: KeyCode) -> bool {
    let kb = InputBinding::Key(keycode);
    for (i, keys) in bindings.players.iter().enumerate() {
        if i < state.players.len() && state.players[i].ai.kind == AiKind::Human && kb == keys.boost {
            return true;
        }
    }
    false
}

/// Process mouse motion for camera control.
pub fn handle_mouse_motion(state: &mut GameState, dx: i32, dy: i32) {
    if let Some(hi) = state.first_human() {
        camera::camera_mouse_input(
            &mut state.players[hi].camera,
            dx as f32, dy as f32,
            false, false,
            0.0, // dt not needed for rotation-only
        );
    }
}
