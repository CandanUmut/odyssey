use glam::{vec2, Vec2};
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::TICK_RATE;
use crate::{BASE_SPEED, BOOST_COST, BOOST_REGEN, BOOST_SPEED, PLAYER_RADIUS};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerKinematics {
    pub position: Vec2,
    pub velocity: Vec2,
    pub stamina: f32,
}

impl PlayerKinematics {
    pub fn spawn(start: Vec2) -> Self {
        Self {
            position: start,
            velocity: Vec2::ZERO,
            stamina: 100.0,
        }
    }
}

pub fn integrate_input(
    mut kin: PlayerKinematics,
    input: &crate::InputFrame,
    dt: f32,
) -> PlayerKinematics {
    let mut dir = Vec2::ZERO;
    if input.up {
        dir.y += 1.0;
    }
    if input.down {
        dir.y -= 1.0;
    }
    if input.left {
        dir.x -= 1.0;
    }
    if input.right {
        dir.x += 1.0;
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
        vec2(0.0, 0.0)
    };

    kin.velocity = accel;
    kin.position += kin.velocity * dt;
    kin
}

pub fn jitter_color(seed: u64) -> [f32; 3] {
    let mut rng = StdRng::seed_from_u64(seed);
    [
        rng.gen_range(0.3..1.0),
        rng.gen_range(0.3..1.0),
        rng.gen_range(0.5..1.0),
    ]
}

pub fn distance(a: Vec2, b: Vec2) -> f32 {
    a.distance(b)
}

pub fn overlaps(a: Vec2, b: Vec2) -> bool {
    distance(a, b) < PLAYER_RADIUS * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrates_upward_motion() {
        let kin = PlayerKinematics::spawn(Vec2::ZERO);
        let input = crate::InputFrame {
            up: true,
            ..Default::default()
        };
        let result = integrate_input(kin, &input, 1.0 / TICK_RATE as f32);
        assert!(result.position.y > 0.0);
    }
}
