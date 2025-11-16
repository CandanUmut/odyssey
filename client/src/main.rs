use std::{net::UdpSocket, time::SystemTime};

use bevy::prelude::*;
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport};
use bevy_renet::renet::{ConnectionConfig, RenetClient};
use bevy_renet::RenetClientPlugin;
use rand::Rng;
use shared::*;

#[derive(Resource)]
struct LocalPlayer {
    name: String,
    room_code: Option<String>,
    joined: bool,
}

#[derive(Component)]
struct PlayerAvatar {
    id: u64,
}

#[derive(Resource, Default)]
struct SnapshotTick(u32);

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(RenetClientPlugin)
        .add_plugins(NetcodeClientPlugin)
        .insert_resource(SnapshotTick::default())
        .add_systems(Startup, start_connection)
        .add_systems(Update, (poll_connection_status, apply_snapshots))
        .add_systems(FixedUpdate, send_inputs)
        .run();
}

fn start_connection(mut commands: Commands) {
    let client_id = rand::thread_rng().gen();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let auth = ClientAuthentication::Unsecure {
        protocol_id: PROTOCOL_ID,
        client_id,
        server_addr: "127.0.0.1:5000".parse().unwrap(),
        user_data: None,
    };
    let transport = NetcodeClientTransport::new(current_time, auth, socket).unwrap();
    let client = RenetClient::new(ConnectionConfig::default());

    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.insert_resource(LocalPlayer {
        name: format!("Explorer-{}", rand::thread_rng().gen_range(100..999)),
        room_code: None,
        joined: false,
    });
}

fn poll_connection_status(mut client: ResMut<RenetClient>, mut player: ResMut<LocalPlayer>) {
    if client.is_disconnected() {
        return;
    }
    if client.is_connected() && !player.joined {
        let join = ClientMessage::JoinRoom {
            name: player.name.clone(),
            room_code: player.room_code.clone(),
        };
        if let Ok(bytes) = bincode::serialize(&join) {
            client.send_message(0, bytes);
        }
        player.joined = true;
    }
}

fn send_inputs(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut client: ResMut<RenetClient>,
    mut tick: ResMut<SnapshotTick>,
) {
    if !client.is_connected() {
        return;
    }
    tick.0 = tick.0.wrapping_add(1);
    let input = InputFrame {
        tick: tick.0,
        up: keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp),
        down: keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown),
        left: keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft),
        right: keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight),
        boost: keyboard.pressed(KeyCode::Space) || keyboard.pressed(KeyCode::ShiftLeft),
    };
    if let Ok(bytes) = bincode::serialize(&ClientMessage::InputFrame(input)) {
        client.send_message(0, bytes);
    }
}

fn apply_snapshots(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut avatar_query: Query<(Entity, &PlayerAvatar)>,
) {
    if !client.is_connected() {
        return;
    }
    while let Some(message) = client.receive_message(0) {
        if let Ok(msg) = bincode::deserialize::<ServerMessage>(&message) {
            if let ServerMessage::Snapshot { entities, .. } = msg {
                for (entity, avatar) in avatar_query.iter_mut() {
                    if !entities.iter().any(|e| e.id == avatar.id) {
                        commands.entity(entity).despawn();
                    }
                }
                for snapshot in entities {
                    if let Some((entity, _)) = avatar_query
                        .iter_mut()
                        .find(|(_, avatar)| avatar.id == snapshot.id)
                    {
                        commands
                            .entity(entity)
                            .insert(Transform::from_translation(Vec3::new(
                                snapshot.position.x,
                                snapshot.position.y,
                                0.0,
                            )));
                    } else {
                        commands.spawn((
                            Transform::from_translation(Vec3::new(
                                snapshot.position.x,
                                snapshot.position.y,
                                0.0,
                            )),
                            PlayerAvatar { id: snapshot.id },
                        ));
                    }
                }
            }
        }
    }
}
