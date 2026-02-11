use glam::Vec2;
use crate::types::*;
use crate::physics;

/// Generate start positions for N players arranged in a circle.
fn start_positions(n: usize, grid_size: f32) -> Vec<(Vec2, Dir)> {
    let g = grid_size / 2.0;
    (0..n).map(|i| {
        let angle = std::f32::consts::TAU * i as f32 / n as f32;
        let pos = Vec2::new(angle.cos() * g * 0.5, angle.sin() * g * 0.5);
        // Pick facing direction: opposite of spawn angle (toward center)
        // Map to nearest cardinal using Dir movement vectors
        let dirs = [Dir::Up, Dir::Right, Dir::Down, Dir::Left];
        let best = dirs.iter().copied().max_by(|a, b| {
            let dot_a = (-pos).normalize().dot(a.vec());
            let dot_b = (-pos).normalize().dot(b.vec());
            dot_a.partial_cmp(&dot_b).unwrap()
        }).unwrap();
        (pos, best)
    }).collect()
}

/// Initialize all players at starting positions around the grid.
pub fn init_players(rules: &GameRules, ai_kinds: &[AiKind]) -> (Vec<Player>, Vec<PlayerVisual>) {
    let n = ai_kinds.len().min(MAX_PLAYERS);
    let positions = start_positions(n, rules.grid_size);

    let players = (0..n).map(|i| {
        let (pos, dir) = positions[i];
        let mut trails = vec![Segment2::new(Vec2::ZERO, Vec2::ZERO); MAX_TRAIL];
        trails[0] = Segment2::new(pos, Vec2::ZERO);

        let (cam_r, cam_chi, cam_phi) = CameraKind::Follow.defaults();
        Player {
            data: PlayerData {
                dir,
                last_dir: dir,
                speed: 1.0,
                kills: 0,
                deaths: 0,
                wins: 0,
                booster: rules.advanced.boost_max,
                boost_enabled: false,
                wall_accel_active: false,
                speed_mult: 1.0,
                trail_height: rules.advanced.trail_height,
                trail_fade: 0.0,
                turn_time: 0,
                trails,
                trail_offset: 0,
            },
            ai: AiState {
                kind: ai_kinds[i],
                tdiff: 0,
                last_time: 0,
                distances: AiDistances::default(),
            },
            camera: Camera {
                pos: glam::Vec3::ZERO,
                target: glam::Vec3::ZERO,
                r: cam_r,
                chi: cam_chi,
                phi: cam_phi,
                phi_offset: 0.0,
                kind: CameraKind::Follow,
                interpolated_cam: true,
                interpolated_target: true,
                coupled: true,
                freedom: CameraFreedom { r: true, phi: false, chi: true },
            },
            show_scoreboard: false,
            spectate_target: None,
            death_time: None,
        }
    }).collect();

    let visuals = (0..n).map(|i| PlayerVisual {
        diffuse: MODEL_DIFFUSE[i % MODEL_DIFFUSE.len()],
        specular: MODEL_SPECULAR[i % MODEL_SPECULAR.len()],
        trail_color: TRAIL_DIFFUSE[i % TRAIL_DIFFUSE.len()],
        impact_radius: 0.0,
        exp_radius: 0.0,
        crash_pos: None,
        crash_dir: None,
        crash_angle: 0.0,
        shatter_particles: Vec::new(),
    }).collect();

    (players, visuals)
}

/// Create initial game state.
pub fn new_game(rules: GameRules, _settings: &Settings) -> GameState {
    let (mut players, visuals) = init_players(&rules, &rules.ai_players);

    // Disable None players immediately
    for (i, p) in players.iter_mut().enumerate() {
        if rules.ai_players[i] == AiKind::None {
            p.data.speed = -2.0;
            p.data.trail_height = 0.0;
        }
    }
    let running = rules.ai_players.iter().filter(|k| **k != AiKind::None).count();

    GameState {
        running,
        players,
        visuals,
        rules,
        time: GameTime::default(),
        winner: None,
        pause: PauseState::Running,
        pause_modal: false,
        pause_modal_selection: 0,
        events: Vec::new(),
        recognizer_alpha: 0.0,
        fast_forward: false,
    }
}

/// Reset game for a new round, preserving stats (K/D/W).
pub fn reset_game(state: &mut GameState) {
    let stats: Vec<(i32, i32, i32)> = state.players.iter()
        .map(|p| (p.data.kills, p.data.deaths, p.data.wins))
        .collect();

    let (mut players, visuals) = init_players(&state.rules, &state.rules.ai_players);
    for (i, (k, d, w)) in stats.into_iter().enumerate() {
        if i < players.len() {
            players[i].data.kills = k;
            players[i].data.deaths = d;
            players[i].data.wins = w;
        }
    }
    for (i, p) in players.iter_mut().enumerate() {
        if state.rules.ai_players[i] == AiKind::None {
            p.data.speed = -2.0;
            p.data.trail_height = 0.0;
        }
    }
    let running = state.rules.ai_players.iter().filter(|k| **k != AiKind::None).count();

    state.players = players;
    state.visuals = visuals;
    state.running = running;
    state.winner = None;
    state.pause = PauseState::Running;
    state.pause_modal = false;
    state.pause_modal_selection = 0;
    state.time = GameTime::default();
    state.events.clear();
    state.recognizer_alpha = 0.0;
    for p in &mut state.players { p.spectate_target = None; p.death_time = None; }
}

/// Main game update tick. Called with fixed timestep.
pub fn game_idle(state: &mut GameState) {
    if state.pause != PauseState::Running {
        return;
    }

    let walls = arena_walls(state.rules.grid_size);

    // Process pending turn events
    let events = std::mem::take(&mut state.events);
    for event in &events {
        match *event {
            GameEvent::TurnLeft { player, .. } => {
                let new_dir = state.players[player].data.dir.turn_left();
                physics::do_turn(&mut state.players[player], new_dir);
                state.players[player].data.turn_time = state.time.current;
            }
            GameEvent::TurnRight { player, .. } => {
                let new_dir = state.players[player].data.dir.turn_right();
                physics::do_turn(&mut state.players[player], new_dir);
                state.players[player].data.turn_time = state.time.current;
            }
            _ => {}
        }
    }

    // Physics / movement
    let crash_events = physics::do_movement(
        &mut state.players,
        &state.rules,
        &state.time,
        &walls,
    );

    let free_run = state.players.len() == 1;

    // Process crashes
    for event in &crash_events {
        if let GameEvent::Crash { player, killed_by, .. } = event {
            if !free_run {
                state.players[*player].data.deaths += 1;
                if let Some(killer) = killed_by {
                    state.players[*killer].data.kills += 1;
                }
                if state.players[*player].ai.kind == AiKind::Human {
                    // Spectate killer if alive, otherwise find any alive player
                    let target = killed_by
                        .filter(|&k| state.players[k].data.speed > 0.0)
                        .or_else(|| state.players.iter().position(|p| p.data.speed > 0.0));
                    state.players[*player].spectate_target = target;
                    state.players[*player].death_time = Some(state.time.current);
                }
                // Reassign spectators who were watching this player
                let next_alive = state.players.iter().position(|p| p.data.speed > 0.0);
                for pi in 0..state.players.len() {
                    if state.players[pi].spectate_target == Some(*player) {
                        state.players[pi].spectate_target = next_alive;
                    }
                }
            }
            state.running = physics::do_crash(
                &mut state.players,
                &mut state.visuals,
                *player,
                state.rules.advanced.trail_fade_duration,
            );
        }
    }

    // Tick trail fade animations for crashed players
    let dt = state.time.dt as f32;
    let adv_trail_h = state.rules.advanced.trail_height;
    let adv_fade_dur = state.rules.advanced.trail_fade_duration;
    for (i, p) in state.players.iter_mut().enumerate() {
        if p.data.trail_fade > 0.0 {
            p.data.trail_fade = (p.data.trail_fade - dt).max(0.0);
            let t = p.data.trail_fade / adv_fade_dur;
            p.data.trail_height = adv_trail_h * t;
            state.visuals[i].trail_color[3] = TRAIL_DIFFUSE[i % TRAIL_DIFFUSE.len()][3] * t;
        }
    }

    if free_run {
        // Single player: auto-reset after crash + trail fade
        if state.players[0].data.speed <= 0.0 && state.players[0].data.trail_fade <= 0.0 {
            reset_game(state);
        }
    } else {
        // Check for game end: need running <= 1 AND all trail fades complete
        if state.running <= 1 {
            let all_faded = state.players.iter()
                .all(|p| p.data.speed > 0.0 || p.data.trail_fade <= 0.0);
            if all_faded && state.pause == PauseState::Running {
                let winner_idx = state.players.iter().position(|p| p.data.speed > 0.0);
                state.winner = winner_idx;
                if let Some(wi) = winner_idx {
                    state.players[wi].data.wins += 1;
                }
                state.pause = PauseState::Finished;
            }
        }
    }

    // Update recognizer
    state.recognizer_alpha += state.time.dt as f32 / 2000.0;
}

/// Queue a turn event for a player.
pub fn queue_turn(state: &mut GameState, player: usize, left: bool) {
    if player >= state.players.len() || state.players[player].data.speed <= 0.0 {
        return;
    }
    let pos = physics::current_position(&state.players[player].data);
    let event = if left {
        GameEvent::TurnLeft { player, x: pos.x, y: pos.y, timestamp: state.time.current }
    } else {
        GameEvent::TurnRight { player, x: pos.x, y: pos.y, timestamp: state.time.current }
    };
    state.events.push(event);
}
