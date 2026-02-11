use glam::Vec2;
use crate::types::*;
use crate::physics;

// === Action tables from original GLTron computer.c ===

const AGGRESSIVE: [[u8; 4]; 8] = [
    [2, 0, 2, 2], [0, 1, 1, 2], [0, 1, 1, 2], [0, 1, 1, 2],
    [0, 2, 2, 1], [0, 2, 2, 1], [0, 2, 2, 1], [1, 1, 1, 0],
];

const EVASIVE: [[u8; 4]; 8] = [
    [1, 1, 2, 2], [1, 1, 2, 0], [1, 1, 2, 0], [1, 1, 2, 0],
    [2, 0, 1, 1], [2, 0, 1, 1], [2, 0, 1, 1], [2, 2, 1, 1],
];

const SAVE_T_DIFF: f32 = 0.500;
const SAVE_SPEED_DIFF: f32 = 1.0;
const HOPELESS_T: f32 = 0.80;

// === Space estimation ===

/// Cast rays in 5 directions and estimate how much open space is on the left vs right side.
/// Returns (left_space, right_space) — higher = more room on that side.
fn estimate_space(
    player_idx: usize,
    players: &[Player],
    walls: &[Segment2; 4],
    grid_size: f32,
    erase_crashed: bool,
) -> (f32, f32) {
    let data = &players[player_idx].data;
    let pos = physics::current_position(data);
    let fwd = data.dir.vec();
    let left = data.dir.turn_left().vec();
    let right = data.dir.turn_right().vec();

    let max_d = grid_size;
    let rc = |dir: Vec2| physics::ray_cast(pos, dir, players, player_idx, walls, max_d, erase_crashed);

    let d_front = rc(fwd);
    let d_left = rc(left);
    let d_right = rc(right);
    // Diagonal rays at ~45° between forward and left/right
    let d_fwd_left = rc((fwd + left).normalize_or_zero());
    let d_fwd_right = rc((fwd + right).normalize_or_zero());

    // Weight: perpendicular side ray matters most, diagonal less, forward shared
    let left_space = d_left * 1.0 + d_fwd_left * 0.7 + d_front * 0.3;
    let right_space = d_right * 1.0 + d_fwd_right * 0.7 + d_front * 0.3;

    (left_space, right_space)
}

// === Distance calculation ===

fn get_distances(
    player_idx: usize,
    players: &[Player],
    walls: &[Segment2; 4],
    grid_size: f32,
    erase_crashed: bool,
) -> AiDistances {
    let data = &players[player_idx].data;
    let pos = physics::current_position(data);
    let front = data.dir.vec();
    let left_dir = data.dir.turn_left().vec();
    let right_dir = data.dir.turn_right().vec();
    let back_left_dir = (left_dir - front).normalize_or_zero();

    let max_d = grid_size;
    AiDistances {
        front: physics::ray_cast(pos, front, players, player_idx, walls, max_d, erase_crashed),
        left: physics::ray_cast(pos, left_dir, players, player_idx, walls, max_d, erase_crashed),
        right: physics::ray_cast(pos, right_dir, players, player_idx, walls, max_d, erase_crashed),
        back_left: physics::ray_cast(pos, back_left_dir, players, player_idx, walls, max_d, erase_crashed),
    }
}

// === Opponent tracking ===

fn closest_opponent(player_idx: usize, players: &[Player]) -> Option<(usize, f32)> {
    let pos = physics::current_position(&players[player_idx].data);
    players.iter().enumerate()
        .filter(|(i, p)| *i != player_idx && p.data.speed > 0.0)
        .map(|(i, p)| {
            let opos = physics::current_position(&p.data);
            let dist = (opos.x - pos.x).abs() + (opos.y - pos.y).abs();
            (i, dist)
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
}

/// Sector computation from original ai_getConfig (cross product + acos + binning).
fn opponent_sector(player: &Player, opponent: &Player) -> usize {
    let pos = physics::current_position(&player.data);
    let opos = physics::current_position(&opponent.data);
    let diff = pos - opos;

    let v1 = diff.normalize_or_zero();
    let v2 = opponent.data.dir.vec().normalize_or_zero();

    let cross_z = v1.x * v2.y - v1.y * v2.x;
    let cos_phi = v1.dot(v2).clamp(-1.0, 1.0);
    let mut phi = cos_phi.acos();
    if cross_z > 0.0 {
        phi = std::f32::consts::TAU - phi;
    }

    let quarter = std::f32::consts::FRAC_PI_4;
    for i in 0..8 {
        phi -= quarter;
        if phi < 0.0 { return i; }
    }
    7
}

fn dir_diff(player: &Player, opponent: &Player) -> usize {
    ((player.data.dir as i32 - opponent.data.dir as i32 + 4) % 4) as usize
}

fn safe_turn_distance(level: usize, speed: f32) -> f32 {
    let min_time = AI_PARAMS.min_turn_time[level] as f32;
    min_time * speed / 1000.0 + 20.0
}

// === Intercept computation (from original computer_utilities.c) ===

fn get_config(player: &Player, opponent: &Player) -> (f32, f32, usize) {
    let pos = physics::current_position(&player.data);
    let opos = physics::current_position(&opponent.data);

    let p_speed = player.data.speed * player.data.speed_mult;
    let o_speed = opponent.data.speed * opponent.data.speed_mult;

    let o_dir = opponent.data.dir.vec();

    let seg1 = Segment2::new(opos, o_dir * o_speed);
    let ortho = Vec2::new(-o_dir.y, o_dir.x).normalize_or_zero() * p_speed;
    let seg2 = Segment2::new(pos, ortho);

    let (mut t_opponent, mut t_player) = if let Some((_, t1, t2)) = seg1.intersect(&seg2) {
        (t1, t2)
    } else {
        (f32::MAX, f32::MAX)
    };

    if t_player < 0.0 { t_player = -t_player; }
    if t_opponent < 0.0 { t_opponent = -t_opponent; }

    (t_player, t_opponent, opponent_sector(player, opponent))
}

// === Simple mode: survive and claim space ===

/// Core survival AI. Key principles:
/// 1. Keep going straight when safe — long runs claim more territory
/// 2. When turning, ALWAYS turn toward the side with more open space (multi-ray estimate)
/// 3. Break spirals early to avoid self-boxing
/// 4. Never turn into a tight space when a wider option exists
fn do_simple(
    player_idx: usize,
    players: &[Player],
    walls: &[Segment2; 4],
    distances: &AiDistances,
    level: usize,
    tdiff: &mut i32,
    seg_length: f32,
    grid_size: f32,
    speed: f32,
    erase_crashed: bool,
    boost_enabled: &mut bool,
) -> Option<bool> {
    let critical = AI_PARAMS.critical_frac[level] * grid_size;
    let max_seg = AI_PARAMS.max_seg_frac[level] * grid_size;
    let spiral_limit = AI_PARAMS.spiral[level];

    *boost_enabled = false;

    // Keep going straight if safe and segment not too long
    if distances.front > critical && seg_length < max_seg {
        return None;
    }

    // Front is the best direction? Keep going.
    if distances.front > distances.right && distances.front > distances.left {
        return None;
    }

    // We need to turn — use multi-ray space estimation to pick the best side
    let (left_space, right_space) = estimate_space(
        player_idx, players, walls, grid_size, erase_crashed,
    );

    let safe_dist = safe_turn_distance(level, speed);

    // Can we safely turn each way?
    let left_ok = distances.left > safe_dist;
    let right_ok = distances.right > safe_dist;

    if left_ok && right_ok {
        // Both sides open — pick the one with more space, respect spiral limit
        if left_space > right_space * 1.1 && *tdiff < spiral_limit {
            *tdiff += 1;
            Some(true)
        } else if right_space > left_space * 1.1 && *tdiff > -spiral_limit {
            *tdiff -= 1;
            Some(false)
        } else {
            // Very similar — use spiral counter to alternate
            if *tdiff > 0 {
                *tdiff -= 1;
                Some(false)
            } else {
                *tdiff += 1;
                Some(true)
            }
        }
    } else if left_ok {
        *tdiff += 1;
        Some(true)
    } else if right_ok {
        *tdiff -= 1;
        Some(false)
    } else {
        // Both sides blocked — desperate, pick the one with more room
        if left_space > right_space {
            *tdiff += 1;
            Some(true)
        } else {
            *tdiff -= 1;
            Some(false)
        }
    }
}

// === Active mode: fight opponents ===

fn ai_left(distances: &AiDistances, safe_dist: f32, tdiff: &mut i32) -> Option<bool> {
    if distances.left > safe_dist {
        *tdiff += 1;
        Some(true)
    } else {
        None
    }
}

fn ai_right(distances: &AiDistances, safe_dist: f32, tdiff: &mut i32) -> Option<bool> {
    if distances.right > safe_dist {
        *tdiff -= 1;
        Some(false)
    } else {
        None
    }
}

fn apply_action_table(
    table: &[[u8; 4]; 8],
    sector: usize,
    ddiff: usize,
    distances: &AiDistances,
    safe_dist: f32,
    tdiff: &mut i32,
) -> Option<bool> {
    match table[sector][ddiff] {
        1 => ai_left(distances, safe_dist, tdiff),
        2 => ai_right(distances, safe_dist, tdiff),
        _ => None,
    }
}

/// Active fight mode. Engages opponents with aggressive cut-offs or evasive maneuvers.
/// When not on direct collision course, actively steers toward the opponent to cut them off.
fn do_active(
    player_idx: usize,
    opponent_idx: usize,
    players: &[Player],
    walls: &[Segment2; 4],
    distances: &AiDistances,
    level: usize,
    tdiff: &mut i32,
    seg_length: f32,
    grid_size: f32,
    speed: f32,
    erase_crashed: bool,
    boost_enabled: &mut bool,
    current_time: u32,
    last_ai_time: u32,
) -> Option<bool> {
    // Active mode: very fast reactions (150ms)
    if current_time.wrapping_sub(last_ai_time) < 150 {
        return None;
    }

    let (t_player, t_opponent, sector) = get_config(
        &players[player_idx],
        &players[opponent_idx],
    );

    let safe_dist = safe_turn_distance(level, speed);

    match sector {
        0 | 1 | 6 | 7 => {
            // On collision course — fight!
            let ddiff = dir_diff(&players[player_idx], &players[opponent_idx]);

            // Always boost during collision course — be aggressive
            *boost_enabled = true;

            let result = if t_player < t_opponent {
                // We arrive first — cut them off
                apply_action_table(&AGGRESSIVE, sector, ddiff, distances, safe_dist, tdiff)
            } else if t_opponent < HOPELESS_T {
                // They're too close — evade!
                apply_action_table(&EVASIVE, sector, ddiff, distances, safe_dist, tdiff)
            } else if t_opponent - t_player < SAVE_T_DIFF {
                // Close race — aggressive
                apply_action_table(&AGGRESSIVE, sector, ddiff, distances, safe_dist, tdiff)
            } else {
                // Behind — evade
                apply_action_table(&EVASIVE, sector, ddiff, distances, safe_dist, tdiff)
            };

            result.or_else(|| do_simple(
                player_idx, players, walls, distances, level,
                tdiff, seg_length, grid_size, speed, erase_crashed, boost_enabled,
            ))
        }
        _ => {
            // Not on collision course — actively pursue!
            // Steer toward the opponent to get into a cut-off position.
            let pos = physics::current_position(&players[player_idx].data);
            let opos = physics::current_position(&players[opponent_idx].data);
            let to_opp = opos - pos;
            let fwd = players[player_idx].data.dir.vec();
            let left_dir = players[player_idx].data.dir.turn_left().vec();

            // Is opponent to our left or right?
            let cross = fwd.x * to_opp.y - fwd.y * to_opp.x;
            // Also check: is opponent roughly ahead? (dot > 0)
            let dot = fwd.dot(to_opp);

            // Boost while pursuing
            *boost_enabled = true;

            if dot > 0.0 {
                // Opponent is ahead — steer toward them
                if cross > 0.0 && distances.left > safe_dist && *tdiff < AI_PARAMS.spiral[level] {
                    *tdiff += 1;
                    Some(true) // turn left toward opponent
                } else if cross < 0.0 && distances.right > safe_dist && *tdiff > -AI_PARAMS.spiral[level] {
                    *tdiff -= 1;
                    Some(false) // turn right toward opponent
                } else {
                    // Can't turn toward them safely — survive
                    do_simple(
                        player_idx, players, walls, distances, level,
                        tdiff, seg_length, grid_size, speed, erase_crashed, boost_enabled,
                    )
                }
            } else {
                // Opponent is behind — just survive smartly, we'll loop back
                do_simple(
                    player_idx, players, walls, distances, level,
                    tdiff, seg_length, grid_size, speed, erase_crashed, boost_enabled,
                )
            }
        }
    }
}

// === Main entry point ===

pub fn do_computer(
    player_idx: usize,
    players: &[Player],
    walls: &[Segment2; 4],
    rules: &GameRules,
    current_time: u32,
    last_ai_time: u32,
    tdiff: &mut i32,
    boost_enabled: &mut bool,
) -> Option<bool> {
    let level = rules.ai_level.min(3);

    if current_time.wrapping_sub(last_ai_time) < AI_PARAMS.min_turn_time[level] {
        return None;
    }

    let distances = get_distances(player_idx, players, walls, rules.grid_size, rules.erase_crashed);
    let seg_length = if players[player_idx].data.trail_offset < players[player_idx].data.trails.len() {
        players[player_idx].data.trails[players[player_idx].data.trail_offset].length()
    } else {
        0.0
    };

    let speed = rules.speed * players[player_idx].data.speed_mult;

    // Engagement: fight ANY nearby opponent (AI vs AI and AI vs human both)
    // This creates dramatic fights between all players
    let opp_max_dist = 0.5 * rules.grid_size; // large engagement range — always looking for a fight
    if level > 0 {
        if let Some((opp_idx, dist)) = closest_opponent(player_idx, players) {
            if dist < opp_max_dist && distances.front >= dist {
                return do_active(
                    player_idx, opp_idx, players, walls, &distances, level,
                    tdiff, seg_length, rules.grid_size, speed, rules.erase_crashed,
                    boost_enabled, current_time, last_ai_time,
                );
            }
        }
    }

    do_simple(
        player_idx, players, walls, &distances, level,
        tdiff, seg_length, rules.grid_size, speed, rules.erase_crashed, boost_enabled,
    )
}

/// Run AI for all computer players.
pub fn run_ai(state: &mut GameState) {
    let walls = arena_walls(state.rules.grid_size);

    for i in 0..state.players.len() {
        if state.players[i].ai.kind != AiKind::Computer || state.players[i].data.speed <= 0.0 {
            continue;
        }

        let last_time = state.players[i].ai.last_time;
        let mut tdiff = state.players[i].ai.tdiff;
        let mut boost_enabled = state.players[i].data.boost_enabled;

        if let Some(turn_left) = do_computer(
            i,
            &state.players,
            &walls,
            &state.rules,
            state.time.current,
            last_time,
            &mut tdiff,
            &mut boost_enabled,
        ) {
            let pos = physics::current_position(&state.players[i].data);
            let event = if turn_left {
                GameEvent::TurnLeft { player: i, x: pos.x, y: pos.y, timestamp: state.time.current }
            } else {
                GameEvent::TurnRight { player: i, x: pos.x, y: pos.y, timestamp: state.time.current }
            };
            state.events.push(event);
            state.players[i].ai.last_time = state.time.current;
        }

        state.players[i].ai.tdiff = tdiff;
        state.players[i].data.boost_enabled = boost_enabled;
    }
}
