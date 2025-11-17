use glam::{vec3, Vec3};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::TICK_RATE;
use crate::{BASE_SPEED, BOOST_COST, BOOST_REGEN, BOOST_SPEED, PLAYER_RADIUS};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerKinematics {
    pub position: Vec3,
    pub velocity: Vec3,
    pub stamina: f32,
}

impl PlayerKinematics {
    pub fn spawn(start: Vec3) -> Self {
        Self {
            position: start,
            velocity: Vec3::ZERO,
            stamina: 100.0,
        }
    }
}

pub fn integrate_input(
    mut kin: PlayerKinematics,
    input: &crate::InputFrame,
    dt: f32,
) -> PlayerKinematics {
    let mut dir = Vec3::ZERO;
    if input.up {
        dir.x += 1.0;
    }
    if input.down {
        dir.x -= 1.0;
    }
    if input.left {
        dir.z -= 1.0;
    }
    if input.right {
        dir.z += 1.0;
    }

    let speed = if input.boost && kin.stamina > 0.1 {
        kin.stamina = (kin.stamina - BOOST_COST * dt).max(0.0);
        BOOST_SPEED
    } else {
        kin.stamina = (kin.stamina + BOOST_REGEN * dt).min(100.0);
        BASE_SPEED
    };

    let accel = if dir.length_squared() > 0.01 {
        dir.normalize() * speed
    } else {
        vec3(0.0, 0.0, 0.0)
    };

    kin.velocity = accel;
    kin.position =
        integrate_3d_position(kin.position.to_array(), kin.velocity.to_array(), dt).into();
    kin
}

pub fn integrate_3d_position(pos: [f32; 3], vel: [f32; 3], dt: f32) -> [f32; 3] {
    let position = Vec3::from(pos) + Vec3::from(vel) * dt;
    position.to_array()
}

pub fn clamp_to_radius(mut pos: Vec3, radius: f32) -> Vec3 {
    let radial = Vec3::new(0.0, pos.y, pos.z);
    let len = radial.length();
    if len > radius {
        let corrected = radial.normalize_or_zero() * radius;
        pos.y = corrected.y;
        pos.z = corrected.z;
    }
    pos
}

pub fn jitter_color(seed: u64) -> [f32; 3] {
    let mut rng = StdRng::seed_from_u64(seed);
    [
        rng.gen_range(0.3..1.0),
        rng.gen_range(0.3..1.0),
        rng.gen_range(0.5..1.0),
    ]
}

pub fn distance(a: Vec3, b: Vec3) -> f32 {
    a.distance(b)
}

pub fn overlaps(a: Vec3, b: Vec3) -> bool {
    distance(a, b) < PLAYER_RADIUS * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrates_upward_motion() {
        let kin = PlayerKinematics::spawn(Vec3::ZERO);
        let input = crate::InputFrame {
            up: true,
            ..Default::default()
        };
        let result = integrate_input(kin, &input, 1.0 / TICK_RATE as f32);
        assert!(result.position.x > 0.0);
    }

    #[test]
    fn clamps_to_radius() {
        let pos = Vec3::new(0.0, 500.0, 0.0);
        let clamped = clamp_to_radius(pos, 100.0);
        assert!((clamped.length() - 100.0).abs() < 0.01);
    }
}
