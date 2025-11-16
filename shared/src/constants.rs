use glam::Vec2;

pub const TICK_RATE: u32 = 60;
pub const SNAPSHOT_RATE: u32 = 20;
pub const PROTOCOL_ID: u64 = 7_812_345_678_901;
pub const MAX_PLAYERS: usize = 8;
pub const TRACK_LENGTH: f32 = 3600.0;
pub const BOOST_COST: f32 = 35.0;
pub const BOOST_REGEN: f32 = 15.0;
pub const BASE_SPEED: f32 = 250.0;
pub const BOOST_SPEED: f32 = 420.0;
pub const PLAYER_RADIUS: f32 = 14.0;

pub const REGION_MARKERS: [f32; 6] = [0.0, 400.0, 1100.0, 1700.0, 2600.0, TRACK_LENGTH];
pub const REGION_NAMES: [&str; 6] = ["Vagina", "Cervix", "Uterus", "UTJ", "Tube", "Ampulla"];

pub fn start_position() -> Vec2 {
    Vec2::new(-100.0, 0.0)
}
