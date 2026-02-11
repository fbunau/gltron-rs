use glam::Vec2;
use crate::types::*;

/// Test movement segment against all trails of all players.
/// Returns the closest collision point as (t_param, player_index).
/// If erase_crashed is true, skip trails of dead players (invisible walls).
fn crash_test_trails(movement: &Segment2, players: &[Player], self_idx: usize, erase_crashed: bool) -> Option<(f32, usize)> {
    players.iter().enumerate()
        .filter(|(_, p)| !erase_crashed || p.data.speed > 0.0)
        .flat_map(|(pi, p)| {
            let skip_last = pi == self_idx;
            let count = if skip_last {
                p.data.trail_offset.saturating_sub(1)
            } else {
                p.data.trail_offset + 1
            };
            p.data.trails[..count.min(p.data.trails.len())].iter()
                .filter_map(move |seg| {
                    movement.intersect(seg)
                        .filter(|&(_, t1, t2)| t1 > 1e-6 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0)
                        .map(|(_, t1, _)| (t1, pi))
                })
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
}

/// Test movement segment against arena walls.
fn crash_test_walls(movement: &Segment2, walls: &[Segment2; 4]) -> Option<f32> {
    walls.iter()
        .filter_map(|wall| {
            movement.intersect(wall)
                .filter(|&(_, t1, t2)| t1 > 1e-6 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0)
                .map(|(_, t1, _)| t1)
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
}

/// Compute speed factor with oscillation.
pub fn speed_factor(time: u32, adv: &AdvancedConfig) -> f32 {
    let t = time as f32 * std::f32::consts::TAU / adv.speed_oz_freq;
    1.0 - adv.speed_oz_factor + adv.speed_oz_factor * t.cos()
}

/// Apply booster logic to a player. Updates speed_mult smoothly.
/// Boost consumption scales by speed zone.
/// Returns current speed_mult (1.0 = base speed, up to max_speed_mult at full boost).
pub fn apply_booster(data: &mut PlayerData, enabled: bool, adv: &AdvancedConfig, dt: f32) -> f32 {
    if !enabled {
        data.speed_mult = 1.0;
        return 1.0;
    }
    apply_booster_no_reset(data, adv, dt)
}

/// Like apply_booster but doesn't reset speed_mult when boost key isn't held.
/// Used when wall_accel is also active so wall accel can maintain its own speed_mult.
fn apply_booster_no_reset(data: &mut PlayerData, adv: &AdvancedConfig, dt: f32) -> f32 {
    let dt_s = dt / 1000.0;
    let boost_min = 1.0f32;

    if data.boost_enabled && data.booster > boost_min {
        let zone_mult = if data.speed_mult >= adv.zone_red_mult {
            adv.zone_red_drain
        } else if data.speed_mult >= adv.zone_yellow_mult {
            adv.zone_yellow_drain
        } else {
            1.0
        };

        data.booster = (data.booster - adv.boost_use_rate * zone_mult * dt_s).max(boost_min);
        data.speed_mult = (data.speed_mult + adv.boost_accel_rate * dt_s).min(adv.max_speed_mult);
    } else {
        data.boost_enabled = false;
        data.booster = (data.booster + adv.boost_regen_rate * dt_s).min(adv.boost_max);
        // Only decel from boost if wall_accel isn't currently pushing speed up
        if !data.wall_accel_active {
            data.speed_mult = (data.speed_mult - adv.boost_decel_rate * dt_s).max(1.0);
        }
    }

    data.speed_mult
}

/// Process a single turn event: creates a new trail segment.
pub fn do_turn(player: &mut Player, direction: Dir) {
    let data = &mut player.data;
    let pos = current_position(data);
    // Finalize current segment
    if data.trail_offset < data.trails.len() {
        data.trails[data.trail_offset].direction = pos - data.trails[data.trail_offset].start;
    }
    // Start new segment
    data.last_dir = data.dir;
    data.dir = direction;
    data.turn_time = 0; // will be set to current game time by caller
    data.trail_offset += 1;
    if data.trail_offset < data.trails.len() {
        data.trails[data.trail_offset] = Segment2::new(pos, Vec2::ZERO);
    }
}

/// Get the current position (end of current trail segment).
pub fn current_position(data: &PlayerData) -> Vec2 {
    if data.trail_offset < data.trails.len() {
        data.trails[data.trail_offset].end()
    } else {
        Vec2::ZERO
    }
}

// Wall acceleration constants removed — now read from AdvancedConfig

/// Measure perpendicular distance to nearest enemy trail on left and right.
/// Returns (left_dist, right_dist).
fn wall_accel_distances(
    player_idx: usize,
    players: &[Player],
    _walls: &[Segment2; 4],
    grid_size: f32,
) -> (f32, f32) {
    let data = &players[player_idx].data;
    let pos = current_position(data);
    let left_dir = data.dir.turn_left().vec();
    let right_dir = data.dir.turn_right().vec();

    // Only test against OTHER players' trails (not own, not arena walls)
    let left_ray = Segment2::new(pos, left_dir * grid_size);
    let right_ray = Segment2::new(pos, right_dir * grid_size);

    let mut left_dist = f32::MAX;
    let mut right_dist = f32::MAX;

    for (pi, p) in players.iter().enumerate() {
        if pi == player_idx || p.data.speed <= 0.0 {
            continue;
        }
        let count = (p.data.trail_offset + 1).min(p.data.trails.len());
        for seg in &p.data.trails[..count] {
            if let Some((_, t1, t2)) = left_ray.intersect(seg) {
                if t1 >= 0.0 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0 {
                    let d = t1 * grid_size;
                    if d < left_dist { left_dist = d; }
                }
            }
            if let Some((_, t1, t2)) = right_ray.intersect(seg) {
                if t1 >= 0.0 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0 {
                    let d = t1 * grid_size;
                    if d < right_dist { right_dist = d; }
                }
            }
        }
    }

    (left_dist, right_dist)
}

/// Main movement simulation for one physics step.
/// Returns list of crash events.
pub fn do_movement(
    players: &mut Vec<Player>,
    rules: &GameRules,
    time: &GameTime,
    walls: &[Segment2; 4],
) -> Vec<GameEvent> {
    let dt = time.dt as f32;
    let mut events = Vec::new();

    // Compute new positions first, then check collisions
    for i in 0..players.len() {
        if players[i].data.speed <= 0.0 {
            continue;
        }

        // Apply wall acceleration and/or booster (both can be active simultaneously)
        let adv = &rules.advanced;
        if rules.wall_accel {
            let (left_d, right_d) = wall_accel_distances(i, players, walls, rules.grid_size);
            let dt_s = dt / 1000.0;
            let near_wall = left_d < adv.wall_accel_limit || right_d < adv.wall_accel_limit;
            players[i].data.wall_accel_active = near_wall;
            if near_wall {
                players[i].data.speed_mult = (players[i].data.speed_mult + adv.wall_accel_use * dt_s).min(adv.max_speed_mult);
            } else if !players[i].data.boost_enabled || !rules.booster.enabled {
                // Only decel from wall accel if not actively boosting
                players[i].data.speed_mult = (players[i].data.speed_mult - adv.wall_accel_decrease * dt_s).max(1.0);
            }
        }
        if rules.booster.enabled {
            // When wall_accel is also on, skip the "not enabled" reset to 1.0
            apply_booster_no_reset(&mut players[i].data, adv, dt);
        } else if !rules.wall_accel {
            players[i].data.speed_mult = 1.0;
        }

        let speed_mult = players[i].data.speed_mult;
        let base_speed = rules.speed * speed_factor(time.current, adv);
        let speed = base_speed * speed_mult;

        let distance = dt / 100.0 * speed;
        let dir_vec = players[i].data.dir.vec();
        let movement_vec = dir_vec * distance;

        let data = &players[i].data;
        let seg_start = current_position(data);
        let movement = Segment2::new(seg_start, movement_vec);

        // Test collisions
        let trail_hit = crash_test_trails(&movement, players, i, rules.erase_crashed);
        let wall_hit = crash_test_walls(&movement, walls);

        // Determine closest hit and who caused it
        let (closest, killer) = match (trail_hit, wall_hit) {
            (Some((t1, ki)), Some(t2)) => {
                if t1 <= t2 {
                    // Trail hit is closer — killer is the trail owner (None if self)
                    (Some(t1), if ki != i { Some(ki) } else { None })
                } else {
                    (Some(t2), None) // wall hit
                }
            }
            (Some((t1, ki)), None) => (Some(t1), if ki != i { Some(ki) } else { None }),
            (None, Some(t2)) => (Some(t2), None),
            (None, None) => (None, None),
        };

        if let Some(t) = closest {
            // Crash: truncate movement
            let crash_pos = seg_start + movement_vec * (t - 0.01).max(0.0);
            let offset = players[i].data.trail_offset;
            let start = players[i].data.trails[offset].start;
            players[i].data.trails[offset].direction = crash_pos - start;
            events.push(GameEvent::Crash {
                player: i,
                killed_by: killer,
                x: crash_pos.x,
                y: crash_pos.y,
                timestamp: time.current,
            });
        } else {
            // Normal movement: extend current trail
            let offset = players[i].data.trail_offset;
            players[i].data.trails[offset].direction += movement_vec;
        }
    }

    events
}

/// Process crash: kill player, start trail fade. Returns number of alive players.
pub fn do_crash(players: &mut [Player], visuals: &mut [PlayerVisual], player_idx: usize, trail_fade_duration: f32) -> usize {
    let pos = current_position(&players[player_idx].data);
    players[player_idx].data.speed = -1.0;
    // Don't zero trail_height immediately — start fade animation instead
    players[player_idx].data.trail_fade = trail_fade_duration;
    visuals[player_idx].impact_radius = 0.0;
    visuals[player_idx].exp_radius = 0.0;
    visuals[player_idx].crash_pos = Some(pos);
    visuals[player_idx].crash_dir = Some(players[player_idx].data.dir);
    // Store cycle rotation angle at crash time (dir angle + model offset)
    visuals[player_idx].crash_angle = players[player_idx].data.dir.angle() + std::f32::consts::FRAC_PI_2;

    let alive = players.iter()
        .filter(|p| p.data.speed > 0.0)
        .count();
    alive
}

/// Ray cast from a position in a direction, testing against all trails and walls.
/// Returns distance to nearest obstacle.
pub fn ray_cast(
    from: Vec2,
    dir: Vec2,
    players: &[Player],
    self_idx: usize,
    walls: &[Segment2; 4],
    max_dist: f32,
    erase_crashed: bool,
) -> f32 {
    let ray = Segment2::new(from, dir * max_dist);

    let trail_dist = players.iter().enumerate()
        .filter(|(_, p)| !erase_crashed || p.data.speed > 0.0)
        .flat_map(|(pi, p)| {
            let count = if pi == self_idx {
                p.data.trail_offset.saturating_sub(1)
            } else {
                p.data.trail_offset + 1
            };
            p.data.trails[..count.min(p.data.trails.len())].iter()
                .filter_map(move |seg| {
                    ray.intersect(seg)
                        .filter(|&(_, t1, t2)| t1 >= 0.0 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0)
                        .map(|(_, t1, _)| t1 * max_dist)
                })
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap());

    let wall_dist = walls.iter()
        .filter_map(|wall| {
            ray.intersect(wall)
                .filter(|&(_, t1, t2)| t1 >= 0.0 && t1 <= 1.0 && t2 >= 0.0 && t2 <= 1.0)
                .map(|(_, t1, _)| t1 * max_dist)
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap());

    match (trail_dist, wall_dist) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => max_dist,
    }
}
