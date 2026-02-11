use glam::Vec3;
use crate::types::*;
use crate::physics;

/// Initialize camera for a player based on their position and direction.
pub fn init_camera(camera: &mut Camera, data: &PlayerData) {
    let pos = physics::current_position(data);
    let (r, chi, _phi) = camera.kind.defaults();
    camera.r = r;
    camera.chi = chi;
    camera.phi = data.dir.angle() + std::f32::consts::PI;
    camera.phi_offset = 0.0;

    let (interpolated_cam, interpolated_target, coupled, freedom) = match camera.kind {
        CameraKind::Circling => (true, true, false, CameraFreedom { r: false, phi: false, chi: true }),
        CameraKind::Follow => (true, true, true, CameraFreedom { r: true, phi: false, chi: true }),
        CameraKind::Cockpit => (false, false, true, CameraFreedom { r: false, phi: false, chi: false }),
        CameraKind::Free => (true, true, false, CameraFreedom { r: true, phi: true, chi: true }),
    };
    camera.interpolated_cam = interpolated_cam;
    camera.interpolated_target = interpolated_target;
    camera.coupled = coupled;
    camera.freedom = freedom;

    update_camera_pos(camera, pos);
}

/// Compute camera position from spherical coordinates around target.
fn spherical_to_cartesian(target: Vec3, r: f32, phi: f32, chi: f32) -> Vec3 {
    Vec3::new(
        target.x + r * phi.cos() * chi.sin(),
        target.y + r * phi.sin() * chi.sin(),
        target.z + r * chi.cos(),
    )
}

/// Update camera position based on player position.
fn update_camera_pos(camera: &mut Camera, player_pos: glam::Vec2) {
    let target_pos = Vec3::new(player_pos.x, player_pos.y, 0.0);
    camera.target = target_pos;

    let phi = camera.phi + camera.phi_offset;
    camera.pos = match camera.kind {
        CameraKind::Cockpit => Vec3::new(player_pos.x, player_pos.y, CAM_COCKPIT_Z),
        _ => spherical_to_cartesian(target_pos, camera.r, phi, camera.chi),
    };
}

/// Clamp camera parameters within valid ranges.
fn clamp_camera(camera: &mut Camera) {
    camera.r = camera.r.clamp(CAM_R_MIN, CAM_R_MAX);
    camera.chi = camera.chi.clamp(CAM_CHI_MIN, CAM_CHI_MAX);
}

/// Process mouse input for camera.
/// dt_ms is frame time in milliseconds (used for zoom rate).
pub fn camera_mouse_input(camera: &mut Camera, dx: f32, dy: f32, mouse1: bool, mouse2: bool, dt_ms: f32) {
    if camera.freedom.phi {
        camera.phi -= dx * 0.003; // negative: match original (right mouse = look right)
    } else if camera.coupled {
        // In Follow mode: allow mouse to offset from the coupled behind-angle
        camera.phi_offset -= dx * 0.003;
    }
    if camera.freedom.chi {
        camera.chi -= dy * 0.003; // inverted: mouse up = look up (camera towards horizon)
    }
    if camera.freedom.r {
        let zoom_rate = (camera.r - CAM_R_MIN + 1.0) * dt_ms / 300.0;
        if mouse1 { camera.r += zoom_rate; } // left = zoom out (increase R)
        if mouse2 { camera.r -= zoom_rate; } // right = zoom in (decrease R)
    }
    clamp_camera(camera);
}

/// Compute the behind-angle for a coupled camera, with linear interpolation during turns.
fn coupled_behind_phi(data: &PlayerData, current_time: u32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let behind_new = data.dir.angle() + PI;

    if data.turn_time > 0 {
        let elapsed = current_time.saturating_sub(data.turn_time);
        if elapsed < TURN_LENGTH && data.last_dir != data.dir {
            let t = elapsed as f32 / TURN_LENGTH as f32;
            let behind_old = data.last_dir.angle() + PI;
            let mut diff = behind_new - behind_old;
            while diff > PI { diff -= TAU; }
            while diff < -PI { diff += TAU; }
            behind_old + diff * t
        } else {
            behind_new
        }
    } else {
        behind_new
    }
}

/// Update camera for one frame.
pub fn update_camera(camera: &mut Camera, data: &PlayerData, dt: f32, current_time: u32) {
    let pos = physics::current_position(data);
    let dir_angle = data.dir.angle();

    match camera.kind {
        CameraKind::Circling => {
            camera.phi += CAM_SPEED * dt;
        }
        CameraKind::Follow => {
            // Coupled: phi is directly set from player direction
            // Linear interpolation during turns (matches original)
            camera.phi = coupled_behind_phi(data, current_time);
        }
        CameraKind::Cockpit => {
            let look_dist = 10.0;
            let dir_vec = data.dir.vec();
            camera.pos = Vec3::new(pos.x, pos.y, CAM_COCKPIT_Z);
            camera.target = Vec3::new(
                pos.x + dir_vec.x * look_dist,
                pos.y + dir_vec.y * look_dist,
                CAM_COCKPIT_Z - 1.0,
            );
            return; // cockpit sets pos/target directly
        }
        CameraKind::Free => {}
    }

    update_camera_pos(camera, pos);
}

/// Update camera to spectate the recognizer.
/// Follows the recognizer from behind/above using its parametric path.
pub fn update_camera_spectator(camera: &mut Camera, recognizer_alpha: f32, grid_size: f32, dt: f32) {
    let (pos, vel) = crate::effects::recognizer_position(recognizer_alpha, grid_size);
    let height = crate::types::RECOGNIZER_HEIGHT;

    // Target: recognizer position
    let target = Vec3::new(pos.x, pos.y, height * 0.7);

    // Camera: behind and above the recognizer, using velocity for "behind" direction
    let speed = vel.length().max(0.001);
    let fwd = vel / speed;
    let behind_dist = 30.0;
    let above = 15.0;
    let eye = Vec3::new(
        pos.x - fwd.x * behind_dist,
        pos.y - fwd.y * behind_dist,
        height + above,
    );

    // Smooth interpolation
    let factor = (dt / 400.0).min(1.0);
    camera.pos = camera.pos.lerp(eye, factor);
    camera.target = camera.target.lerp(target, factor);
}

/// Update camera to spectate another player (follow from behind).
pub fn update_camera_spectate_player(camera: &mut Camera, target_data: &PlayerData, dt: f32, current_time: u32) {
    let pos = crate::physics::current_position(target_data);
    let dir_angle = target_data.dir.angle() + std::f32::consts::PI; // behind
    let r = 20.0;
    let chi = std::f32::consts::FRAC_PI_4; // 45 degrees above

    let target = Vec3::new(pos.x, pos.y, CYCLE_HEIGHT);
    let eye = Vec3::new(
        pos.x + r * dir_angle.cos() * chi.sin(),
        pos.y + r * dir_angle.sin() * chi.sin(),
        CYCLE_HEIGHT + r * chi.cos(),
    );

    let factor = (dt / 300.0).min(1.0);
    camera.pos = camera.pos.lerp(eye, factor);
    camera.target = camera.target.lerp(target, factor);
}

/// Cycle to next camera type.
pub fn next_camera_type(camera: &mut Camera, data: &PlayerData) {
    camera.kind = camera.kind.next();
    init_camera(camera, data);
}

/// Get the OpenGL-style view matrix components (eye, center, up).
pub fn get_view_params(camera: &Camera) -> (Vec3, Vec3, Vec3) {
    (camera.pos, camera.target, Vec3::Z)
}
