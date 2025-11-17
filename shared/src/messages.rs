use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSummary {
    pub id: u64,
    pub name: String,
    pub ready: bool,
    pub is_host: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: u64,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub stamina: f32,
    pub region: RegionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub name: String,
    pub ticks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    JoinRoom {
        name: String,
        room_code: Option<String>,
    },
    SetReady {
        ready: bool,
    },
    InputFrame(InputFrame),
    StartRace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    RoomState {
        room_code: String,
        players: Vec<PlayerSummary>,
        state: RoomPhase,
    },
    Countdown {
        millis_left: u32,
    },
    Snapshot {
        tick: u32,
        entities: Vec<EntitySnapshot>,
    },
    RaceFinished {
        leaderboard: Vec<LeaderboardEntry>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputFrame {
    pub tick: u32,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoomPhase {
    Lobby,
    Countdown,
    Racing,
    Finished,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegionId {
    Vagina,
    Cervix,
    Uterus,
    Utj,
    Tube,
    Ampulla,
}
