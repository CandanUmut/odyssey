use std::{collections::HashMap, net::UdpSocket, time::SystemTime};

use bevy::prelude::*;
use bevy_renet::netcode::{
    NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig,
};
use bevy_renet::renet::{ConnectionConfig, RenetServer, ServerEvent};
use bevy_renet::RenetServerPlugin;
use rand::Rng;
use shared::*;

const PORT: u16 = 5000;

#[derive(Resource)]
struct RoomState {
    code: String,
    players: HashMap<u64, PlayerState>,
    phase: RoomPhase,
    countdown: u32,
    tick: u32,
}

#[derive(Debug)]
struct PlayerState {
    name: String,
    ready: bool,
    is_host: bool,
    kin: PlayerKinematics,
    last_input: InputFrame,
    finished_tick: Option<u32>,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(RenetServerPlugin)
        .add_plugins(NetcodeServerPlugin)
        .insert_resource(new_server())
        .insert_resource(new_transport())
        .insert_resource(RoomState {
            code: random_room_code(),
            players: HashMap::new(),
            phase: RoomPhase::Lobby,
            countdown: 3_000,
            tick: 0,
        })
        .add_systems(
            Update,
            (handle_events, network_receive_system, broadcast_room_state),
        )
        .add_systems(
            FixedUpdate,
            (
                apply_inputs,
                physics_step,
                race_state_system,
                snapshot_broadcast_system,
            )
                .chain(),
        )
        .insert_resource(Time::<Fixed>::from_hz(TICK_RATE as f64))
        .run();
}

fn new_transport() -> NetcodeServerTransport {
    let public_addr = format!("0.0.0.0:{}", PORT).parse().unwrap();
    let socket = UdpSocket::bind(public_addr).unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server_config = ServerConfig {
        current_time,
        max_clients: MAX_PLAYERS,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    NetcodeServerTransport::new(server_config, socket).unwrap()
}

fn new_server() -> RenetServer {
    let server_config = ConnectionConfig::default();
    RenetServer::new(server_config)
}

fn handle_events(mut events: MessageReader<ServerEvent>, mut room: ResMut<RoomState>) {
    for event in events.read() {
        match event {
            ServerEvent::ClientConnected { client_id, .. } => {
                let is_host = room.players.is_empty();
                room.players.insert(
                    *client_id,
                    PlayerState {
                        name: format!("Guest-{}", client_id),
                        ready: false,
                        is_host,
                        kin: PlayerKinematics::spawn(start_position()),
                        last_input: InputFrame::default(),
                        finished_tick: None,
                    },
                );
                info!("Client {client_id} connected");
            }
            ServerEvent::ClientDisconnected { client_id, .. } => {
                room.players.remove(client_id);
                info!("Client {client_id} disconnected");
                if room.players.is_empty() {
                    room.phase = RoomPhase::Lobby;
                }
            }
        }
    }
}

fn network_receive_system(mut server: ResMut<RenetServer>, mut room: ResMut<RoomState>) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, 0) {
            if let Ok(msg) = bincode::deserialize::<ClientMessage>(&message) {
                match msg {
                    ClientMessage::JoinRoom { name, .. } => {
                        if let Some(player) = room.players.get_mut(&client_id) {
                            player.name = name;
                        }
                    }
                    ClientMessage::SetReady { ready } => {
                        if let Some(player) = room.players.get_mut(&client_id) {
                            player.ready = ready;
                        }
                        if room.players.values().all(|p| p.ready)
                            && matches!(room.phase, RoomPhase::Lobby)
                        {
                            room.phase = RoomPhase::Countdown;
                            room.countdown = 3_000;
                        }
                    }
                    ClientMessage::InputFrame(input) => {
                        if let Some(player) = room.players.get_mut(&client_id) {
                            player.last_input = input;
                        }
                    }
                    ClientMessage::StartRace => {
                        if let Some(player) = room.players.get(&client_id) {
                            if player.is_host && matches!(room.phase, RoomPhase::Lobby) {
                                room.phase = RoomPhase::Countdown;
                                room.countdown = 3_000;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn apply_inputs(mut room: ResMut<RoomState>) {
    if !matches!(room.phase, RoomPhase::Countdown | RoomPhase::Racing) {
        return;
    }
    let dt = 1.0 / TICK_RATE as f32;
    for player in room.players.values_mut() {
        let mut kin = integrate_input(player.kin.clone(), &player.last_input, dt);
        let region = region_for_position(kin.position);
        let radius = tube_radius(region);
        kin.position = clamp_to_radius(kin.position, radius);
        player.kin = kin;
    }
}

fn physics_step(mut room: ResMut<RoomState>) {
    if matches!(room.phase, RoomPhase::Countdown) {
        if room.countdown > 0 {
            room.countdown = room.countdown.saturating_sub(1000 / TICK_RATE);
            if room.countdown == 0 {
                room.phase = RoomPhase::Racing;
            }
        }
        return;
    }

    if !matches!(room.phase, RoomPhase::Racing) {
        return;
    }

    room.tick = room.tick.wrapping_add(1);
    let current_tick = room.tick;
    for player in room.players.values_mut() {
        if region_for_position(player.kin.position) == RegionId::Ampulla
            && player.finished_tick.is_none()
        {
            player.finished_tick = Some(current_tick);
        }
    }
}

fn race_state_system(mut room: ResMut<RoomState>) {
    if matches!(room.phase, RoomPhase::Racing)
        && room.players.values().any(|p| p.finished_tick.is_some())
    {
        room.phase = RoomPhase::Finished;
    }
}

fn snapshot_broadcast_system(mut server: ResMut<RenetServer>, room: Res<RoomState>) {
    if !matches!(room.phase, RoomPhase::Racing | RoomPhase::Countdown) {
        return;
    }

    let entities = room
        .players
        .iter()
        .map(|(id, player)| EntitySnapshot {
            id: *id,
            position: player.kin.position.to_array(),
            velocity: player.kin.velocity.to_array(),
            stamina: player.kin.stamina,
            region: region_for_position(player.kin.position),
        })
        .collect();

    let snapshot = ServerMessage::Snapshot {
        tick: room.tick,
        entities,
    };
    let payload = bincode::serialize(&snapshot).unwrap();
    for client_id in server.clients_id() {
        server.send_message(client_id, 0, payload.clone());
    }
}

fn broadcast_room_state(mut server: ResMut<RenetServer>, room: Res<RoomState>) {
    let msg = ServerMessage::RoomState {
        room_code: room.code.clone(),
        players: room
            .players
            .iter()
            .map(|(id, p)| PlayerSummary {
                id: *id,
                name: p.name.clone(),
                ready: p.ready,
                is_host: p.is_host,
            })
            .collect(),
        state: room.phase.clone(),
    };

    let payload = bincode::serialize(&msg).unwrap();
    for client_id in server.clients_id() {
        server.send_message(client_id, 0, payload.clone());
    }
}

fn random_room_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx] as char
        })
        .collect()
}
