use crate::types::*;
use crate::game;
use crate::input::{GLANCE_ANGLE, InputBinding, KeyBindings, PlayerAction};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

const STICK_DEADZONE: f32 = 0.5;

// ============================================================================
// GamepadBtn — platform-agnostic button identifier
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadBtn {
    South,         // A / Cross
    East,          // B / Circle
    West,          // X / Square
    North,         // Y / Triangle
    ShoulderLeft,  // LB / L1
    ShoulderRight, // RB / R1
    TriggerLeft,   // LT / L2
    TriggerRight,  // RT / R2
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Start,
}

impl GamepadBtn {
    pub fn display_name(self) -> &'static str {
        match self {
            GamepadBtn::South => "gp:A",
            GamepadBtn::East => "gp:B",
            GamepadBtn::West => "gp:X",
            GamepadBtn::North => "gp:Y",
            GamepadBtn::ShoulderLeft => "gp:LB",
            GamepadBtn::ShoulderRight => "gp:RB",
            GamepadBtn::TriggerLeft => "gp:LT",
            GamepadBtn::TriggerRight => "gp:RT",
            GamepadBtn::DpadUp => "gp:up",
            GamepadBtn::DpadDown => "gp:down",
            GamepadBtn::DpadLeft => "gp:left",
            GamepadBtn::DpadRight => "gp:right",
            GamepadBtn::Start => "gp:start",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "gp:A" => Some(GamepadBtn::South),
            "gp:B" => Some(GamepadBtn::East),
            "gp:X" => Some(GamepadBtn::West),
            "gp:Y" => Some(GamepadBtn::North),
            "gp:LB" => Some(GamepadBtn::ShoulderLeft),
            "gp:RB" => Some(GamepadBtn::ShoulderRight),
            "gp:LT" => Some(GamepadBtn::TriggerLeft),
            "gp:RT" => Some(GamepadBtn::TriggerRight),
            "gp:up" => Some(GamepadBtn::DpadUp),
            "gp:down" => Some(GamepadBtn::DpadDown),
            "gp:left" => Some(GamepadBtn::DpadLeft),
            "gp:right" => Some(GamepadBtn::DpadRight),
            "gp:start" => Some(GamepadBtn::Start),
            _ => None,
        }
    }

    /// Check if this button is pressed in the given gamepad state.
    pub fn is_pressed(self, gp: &GamepadState) -> bool {
        match self {
            GamepadBtn::South => gp.button_south,
            GamepadBtn::East => gp.button_east,
            GamepadBtn::West => gp.button_west,
            GamepadBtn::North => gp.button_north,
            GamepadBtn::ShoulderLeft => gp.shoulder_left,
            GamepadBtn::ShoulderRight => gp.shoulder_right,
            GamepadBtn::TriggerLeft => gp.trigger_left,
            GamepadBtn::TriggerRight => gp.trigger_right,
            GamepadBtn::DpadUp => gp.dpad_up,
            GamepadBtn::DpadDown => gp.dpad_down,
            GamepadBtn::DpadLeft => gp.dpad_left,
            GamepadBtn::DpadRight => gp.dpad_right,
            GamepadBtn::Start => gp.start,
        }
    }
}

const ALL_BUTTONS: [GamepadBtn; 13] = [
    GamepadBtn::South, GamepadBtn::East, GamepadBtn::West, GamepadBtn::North,
    GamepadBtn::ShoulderLeft, GamepadBtn::ShoulderRight,
    GamepadBtn::TriggerLeft, GamepadBtn::TriggerRight,
    GamepadBtn::DpadUp, GamepadBtn::DpadDown, GamepadBtn::DpadLeft, GamepadBtn::DpadRight,
    GamepadBtn::Start,
];

/// Return the first button that is newly pressed (rising edge) this frame.
pub fn newly_pressed_button(current: &GamepadState, prev: &GamepadState) -> Option<GamepadBtn> {
    for btn in ALL_BUTTONS {
        if btn.is_pressed(current) && !btn.is_pressed(prev) {
            return Some(btn);
        }
    }
    None
}

/// Platform-agnostic gamepad state snapshot.
#[derive(Debug, Clone, Default)]
pub struct GamepadState {
    pub axis_x: f32,
    pub axis_y: f32,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub button_south: bool,   // A / Cross      -> boost
    pub button_east: bool,    // B / Circle
    pub button_west: bool,    // X / Square
    pub button_north: bool,   // Y / Triangle   -> scoreboard
    pub shoulder_left: bool,  // LB / L1        -> glance left
    pub shoulder_right: bool, // RB / R1        -> glance right
    pub trigger_left: bool,   // LT / L2
    pub trigger_right: bool,  // RT / R2
    pub start: bool,          // Start          -> pause
}

// ============================================================================
// GamepadManager — platform-specific implementations
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub struct GamepadManager {
    gilrs: gilrs::Gilrs,
}

#[cfg(not(target_arch = "wasm32"))]
impl GamepadManager {
    pub fn new() -> Self {
        let gilrs = match gilrs::Gilrs::new() {
            Ok(g) => g,
            Err(gilrs::Error::NotImplemented(g)) => {
                log::warn!("Gamepad backend not implemented on this platform");
                g
            }
            Err(e) => {
                log::warn!("Gamepad init failed: {e}");
                // Try to get the inner Gilrs anyway
                panic!("Cannot initialize gamepad library: {e}");
            }
        };
        Self { gilrs }
    }

    pub fn poll(&mut self) -> Vec<GamepadState> {
        // Consume buffered events
        while let Some(_event) = self.gilrs.next_event() {}

        let mut states = Vec::new();
        for (_id, gamepad) in self.gilrs.gamepads() {
            if !gamepad.is_connected() { continue; }
            states.push(GamepadState {
                axis_x: gamepad.value(gilrs::Axis::LeftStickX),
                axis_y: gamepad.value(gilrs::Axis::LeftStickY),
                dpad_left: gamepad.is_pressed(gilrs::Button::DPadLeft),
                dpad_right: gamepad.is_pressed(gilrs::Button::DPadRight),
                dpad_up: gamepad.is_pressed(gilrs::Button::DPadUp),
                dpad_down: gamepad.is_pressed(gilrs::Button::DPadDown),
                button_south: gamepad.is_pressed(gilrs::Button::South),
                button_east: gamepad.is_pressed(gilrs::Button::East),
                button_west: gamepad.is_pressed(gilrs::Button::West),
                button_north: gamepad.is_pressed(gilrs::Button::North),
                shoulder_left: gamepad.is_pressed(gilrs::Button::LeftTrigger),
                shoulder_right: gamepad.is_pressed(gilrs::Button::RightTrigger),
                trigger_left: gamepad.is_pressed(gilrs::Button::LeftTrigger2),
                trigger_right: gamepad.is_pressed(gilrs::Button::RightTrigger2),
                start: gamepad.is_pressed(gilrs::Button::Start),
            });
            if states.len() >= 4 { break; }
        }
        states
    }
}

#[cfg(target_arch = "wasm32")]
pub struct GamepadManager;

#[cfg(target_arch = "wasm32")]
impl GamepadManager {
    pub fn new() -> Self { Self }

    pub fn poll(&mut self) -> Vec<GamepadState> {
        let Some(window) = web_sys::window() else { return Vec::new() };
        let navigator = window.navigator();
        let Ok(gamepads_js) = navigator.get_gamepads() else { return Vec::new() };

        let mut states = Vec::new();
        for i in 0..gamepads_js.length() {
            let val = gamepads_js.get(i);
            if val.is_null() || val.is_undefined() { continue; }
            let gp: web_sys::Gamepad = val.unchecked_into();
            if !gp.connected() { continue; }

            let buttons = gp.buttons();
            let axes = gp.axes();

            let btn = |idx: u32| -> bool {
                let b = buttons.get(idx);
                if b.is_undefined() || b.is_null() { return false; }
                let b: web_sys::GamepadButton = b.unchecked_into();
                b.pressed()
            };
            let axis = |idx: u32| -> f32 {
                axes.get(idx).as_f64().unwrap_or(0.0) as f32
            };

            // Standard mapping: axes 0,1 = left stick; buttons 0-3 = face ABXY;
            // 4,5 = shoulders; 6,7 = triggers; 9 = start; 12-15 = dpad
            states.push(GamepadState {
                axis_x: axis(0),
                axis_y: axis(1),
                button_south: btn(0),
                button_east: btn(1),
                button_west: btn(2),
                button_north: btn(3),
                shoulder_left: btn(4),
                shoulder_right: btn(5),
                trigger_left: btn(6),
                trigger_right: btn(7),
                start: btn(9),
                dpad_up: btn(12),
                dpad_down: btn(13),
                dpad_left: btn(14),
                dpad_right: btn(15),
            });
            if states.len() >= 4 { break; }
        }
        states
    }
}

// ============================================================================
// Gamepad → menu navigation
// ============================================================================

use winit::keyboard::KeyCode;

/// Convert gamepad state to menu key presses (edge-detected).
/// Uses the first gamepad only. Returns a list of KeyCode equivalents
/// that were "pressed" this frame (rising edge).
pub fn gamepad_menu_keys(
    current: &[GamepadState],
    prev: &[GamepadState],
) -> Vec<KeyCode> {
    let Some(gp) = current.first() else { return Vec::new() };
    let prev_gp = prev.first();

    let mut keys = Vec::new();

    // D-pad / stick → arrow keys (edge detect)
    let up_now = gp.dpad_up || gp.axis_y < -STICK_DEADZONE;
    let down_now = gp.dpad_down || gp.axis_y > STICK_DEADZONE;
    let left_now = gp.dpad_left || gp.axis_x < -STICK_DEADZONE;
    let right_now = gp.dpad_right || gp.axis_x > STICK_DEADZONE;

    let up_prev = prev_gp.map_or(false, |p| p.dpad_up || p.axis_y < -STICK_DEADZONE);
    let down_prev = prev_gp.map_or(false, |p| p.dpad_down || p.axis_y > STICK_DEADZONE);
    let left_prev = prev_gp.map_or(false, |p| p.dpad_left || p.axis_x < -STICK_DEADZONE);
    let right_prev = prev_gp.map_or(false, |p| p.dpad_right || p.axis_x > STICK_DEADZONE);

    if up_now && !up_prev { keys.push(KeyCode::ArrowUp); }
    if down_now && !down_prev { keys.push(KeyCode::ArrowDown); }
    if left_now && !left_prev { keys.push(KeyCode::ArrowLeft); }
    if right_now && !right_prev { keys.push(KeyCode::ArrowRight); }

    // A/South → Enter (confirm/select)
    if gp.button_south && !prev_gp.map_or(false, |p| p.button_south) {
        keys.push(KeyCode::Enter);
    }
    // B/East → Escape (back)
    if gp.button_east && !prev_gp.map_or(false, |p| p.button_east) {
        keys.push(KeyCode::Escape);
    }

    keys
}

// ============================================================================
// Gamepad → game input
// ============================================================================

/// Check if a binding is "pressed" on the given gamepad state.
fn binding_pressed(binding: InputBinding, gp: &GamepadState) -> bool {
    match binding {
        InputBinding::Pad(btn) => btn.is_pressed(gp),
        // Also treat stick as turn left/right for d-pad bindings
        InputBinding::Key(_) => false,
    }
}

/// Check if a binding had a rising edge (newly pressed this frame).
fn binding_edge(binding: InputBinding, gp: &GamepadState, prev: &GamepadState) -> bool {
    binding_pressed(binding, gp) && !binding_pressed(binding, prev)
}

/// Result of gamepad input processing for the main loop to act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadAction {
    None,
    Pause,            // Start button while running -> stop engine sound
    Resume,           // Any button while suspended (not in pause modal)
    QuitToMenu,       // Confirm quit in pause modal
    NextRound,        // Any button while finished (non-match-over)
    EndMatch,         // Boost while finished + match over
}

/// Apply gamepad input to game state using player bindings + edge detection.
/// `prev` is updated at the end of each call for next-frame comparison.
/// Returns an action for the main loop to handle (resume, quit, etc.)
pub fn apply_gamepad_input(
    state: &mut GameState,
    bindings: &KeyBindings,
    current: &[GamepadState],
    prev: &mut Vec<GamepadState>,
) -> GamepadAction {
    let mut action = GamepadAction::None;

    // Use first gamepad for pause/resume/modal controls
    let default_prev = GamepadState::default();
    if let Some(gp) = current.first() {
        let prev_gp = prev.first().unwrap_or(&default_prev);
        let any_pressed = newly_pressed_button(gp, prev_gp).is_some();

        match state.pause {
            PauseState::Suspended => {
                if state.pause_modal {
                    // Pause modal: d-pad/stick up/down to navigate, South to confirm, East to cancel
                    let up_edge = (gp.dpad_up && !prev_gp.dpad_up)
                        || (gp.axis_y < -STICK_DEADZONE && !(prev_gp.axis_y < -STICK_DEADZONE));
                    let down_edge = (gp.dpad_down && !prev_gp.dpad_down)
                        || (gp.axis_y > STICK_DEADZONE && !(prev_gp.axis_y > STICK_DEADZONE));
                    if up_edge || down_edge {
                        state.pause_modal_selection = 1 - state.pause_modal_selection;
                    }
                    if gp.button_south && !prev_gp.button_south {
                        if state.pause_modal_selection == 0 {
                            state.pause_modal = false;
                        } else {
                            action = GamepadAction::QuitToMenu;
                        }
                    }
                    if gp.button_east && !prev_gp.button_east {
                        state.pause_modal = false;
                    }
                } else if any_pressed {
                    // Any button resumes
                    action = GamepadAction::Resume;
                }
            }
            PauseState::Finished => {
                let match_over = state.players.iter().any(|p| p.data.wins >= state.rules.rounds_to_win);
                if match_over {
                    // Need boost button to end match
                    let boost_edge = gp.button_south && !prev_gp.button_south;
                    if boost_edge {
                        action = GamepadAction::EndMatch;
                    }
                } else if any_pressed {
                    action = GamepadAction::NextRound;
                }
            }
            PauseState::Running => {
                // Start button -> pause (edge detect)
                if gp.start && !prev_gp.start {
                    state.pause = PauseState::Suspended;
                    state.pause_modal = true;
                    state.pause_modal_selection = 0;
                    action = GamepadAction::Pause;
                }
            }
            PauseState::NoGame => {}
        }
    }

    // Only process gameplay input when running
    if state.pause == PauseState::Running {
        let human_indices: Vec<usize> = state.players.iter().enumerate()
            .filter(|(_, p)| p.ai.kind == AiKind::Human)
            .map(|(i, _)| i)
            .collect();

        for (gp_idx, gp) in current.iter().enumerate() {
            let Some(&player_idx) = human_indices.get(gp_idx) else { continue };
            let prev_gp = prev.get(gp_idx).unwrap_or(&default_prev);

            // Get this player's bindings (if they have a keyboard slot)
            let keys = if player_idx < bindings.players.len() {
                Some(&bindings.players[player_idx])
            } else {
                None
            };

            // Stick/d-pad turns (always available as natural gamepad turn controls)
            let left_now = gp.dpad_left || gp.axis_x < -STICK_DEADZONE;
            let right_now = gp.dpad_right || gp.axis_x > STICK_DEADZONE;
            let left_prev = prev_gp.dpad_left || prev_gp.axis_x < -STICK_DEADZONE;
            let right_prev = prev_gp.dpad_right || prev_gp.axis_x > STICK_DEADZONE;

            // Also check if any binding triggers turn
            let turn_left_edge = (left_now && !left_prev)
                || keys.is_some_and(|k| binding_edge(k.left, gp, prev_gp));
            let turn_right_edge = (right_now && !right_prev)
                || keys.is_some_and(|k| binding_edge(k.right, gp, prev_gp));

            // Boost: check binding, or fallback to South button
            let boost_held = keys.map_or(gp.button_south, |k| {
                binding_pressed(k.boost, gp) || gp.button_south
            });
            let boost_edge = keys.map_or(
                gp.button_south && !prev_gp.button_south,
                |k| binding_edge(k.boost, gp, prev_gp) || (gp.button_south && !prev_gp.button_south),
            );

            // Glance: check bindings, or fallback to shoulders
            let glance_left_held = keys.map_or(gp.shoulder_left, |k| {
                binding_pressed(k.glance_left, gp) || gp.shoulder_left
            });
            let glance_right_held = keys.map_or(gp.shoulder_right, |k| {
                binding_pressed(k.glance_right, gp) || gp.shoulder_right
            });
            let prev_glance_left = keys.map_or(prev_gp.shoulder_left, |k| {
                binding_pressed(k.glance_left, prev_gp) || prev_gp.shoulder_left
            });
            let prev_glance_right = keys.map_or(prev_gp.shoulder_right, |k| {
                binding_pressed(k.glance_right, prev_gp) || prev_gp.shoulder_right
            });

            // Scoreboard: check binding, or fallback to North button
            let scoreboard_held = keys.map_or(gp.button_north, |k| {
                binding_pressed(k.scoreboard, gp) || gp.button_north
            });

            if state.players[player_idx].data.speed > 0.0 {
                // Alive: turns, boost, glance
                if turn_left_edge {
                    game::queue_turn(state, player_idx, true);
                }
                if turn_right_edge {
                    game::queue_turn(state, player_idx, false);
                }

                // Boost (continuous)
                state.players[player_idx].data.boost_enabled = boost_held;

                // Glance (hold)
                if glance_left_held {
                    state.players[player_idx].camera.phi_offset = GLANCE_ANGLE;
                } else if glance_right_held {
                    state.players[player_idx].camera.phi_offset = -GLANCE_ANGLE;
                } else if prev_glance_left || prev_glance_right {
                    state.players[player_idx].camera.phi_offset = 0.0;
                }
            } else {
                // Dead: boost = fast forward (if all humans dead)
                if boost_edge {
                    let all_humans_dead = state.players.iter()
                        .filter(|p| p.ai.kind == AiKind::Human)
                        .all(|p| p.data.speed <= 0.0);
                    if all_humans_dead {
                        state.fast_forward = true;
                    }
                }
            }

            // Scoreboard (hold)
            state.players[player_idx].show_scoreboard = scoreboard_held;
        }
    }

    // Update previous state
    *prev = current.to_vec();

    action
}
