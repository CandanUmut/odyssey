use std::{net::UdpSocket, time::SystemTime};

use bevy::prelude::*;
use bevy::time::Fixed;
use bevy::window::WindowResolution;
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport};
use bevy_renet::renet::{ConnectionConfig, RenetClient};
use bevy_renet::RenetClientPlugin;
use rand::Rng;
use shared::*; // PROTOCOL_ID, TICK_RATE, PLAYER_RADIUS, RegionId, ClientMessage, ServerMessage, InputFrame, etc.

/// Info about this client in the room
#[derive(Resource)]
struct LocalPlayer {
    name: String,
    room_code: Option<String>,
    joined: bool,
}

/// Monotonic tick we use when sending InputFrame
#[derive(Resource, Default)]
struct SnapshotTick(u32);

/// Tag for a player’s sperm avatar
#[derive(Component)]
struct PlayerAvatar {
    id: u64,
}

/// Tag for the egg marker
#[derive(Component)]
struct EggMarker;

fn main() {
    App::new()
        // Window + renderer
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Odyssey: Race to the Egg".into(),
                    // Bevy 0.17: u32, not f32
                    resolution: WindowResolution::new(1280, 720),
                    resizable: true,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        // Renet networking client
        .add_plugins(RenetClientPlugin)
        .add_plugins(NetcodeClientPlugin)
        // Background color (slightly purple so we can see contrast)
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.10)))
        .insert_resource(SnapshotTick::default())
        // Startup: scene + connection
        .add_systems(Startup, (setup_scene, start_connection))
        // Frame-based systems
        .add_systems(
            Update,
            (
                poll_connection_status,
                apply_snapshots,
                camera_follow_players,
                sperm_pulse_animation,
            ),
        )
        // Fixed-timestep (network input)
        .insert_resource(Time::<Fixed>::from_hz(TICK_RATE as f64))
        .add_systems(FixedUpdate, send_inputs)
        .run();
}

/// Create camera + a big colored “track” + egg marker + dummy player
fn setup_scene(mut commands: Commands) {
    // 2D camera
    commands.spawn(Camera2d);

    // --- Region bands (so the world actually looks like something) ---

    let band_width = 220.0;
    let full_height = 720.0;

    // Positions from left to right
    let xs = [
        -550.0, // Vagina
        -330.0, // Cervix
        -110.0, // Uterus
        110.0,  // Utj
        330.0,  // Tube
        550.0,  // Ampulla
    ];

    let regions = [
        RegionId::Vagina,
        RegionId::Cervix,
        RegionId::Uterus,
        RegionId::Utj,
        RegionId::Tube,
        RegionId::Ampulla,
    ];

    for (x, region) in xs.into_iter().zip(regions.into_iter()) {
        commands.spawn((
            Sprite::from_color(
                color_for_region(region),
                Vec2::new(band_width, full_height),
            ),
            // background z
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }

    // Egg marker at the far right (bright and big)
    let egg_x = 650.0;
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.95, 0.8), Vec2::splat(80.0)),
        Transform::from_xyz(egg_x, 0.0, 1.0),
        EggMarker,
    ));

    // Soft glow around the egg
    commands.spawn((
        Sprite::from_color(
            Color::srgba(1.0, 0.8, 0.4, 0.18),
            Vec2::new(260.0, 260.0),
        ),
        Transform::from_xyz(egg_x, 0.0, 0.5),
    ));

    // Dummy local player so we always see at least one sperm
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.9, 0.9, 1.0),
            Vec2::splat(PLAYER_RADIUS * 2.0),
        ),
        Transform::from_xyz(-550.0, 0.0, 2.0),
        PlayerAvatar { id: 0 },
    ));
}

/// Creates the Renet client + Netcode transport and inserts LocalPlayer
fn start_connection(mut commands: Commands) {
    let client_id: u64 = rand::thread_rng().gen();
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

/// Once connected, send initial JoinRoom message
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

/// Read keyboard and send InputFrame to server at fixed tick rate
fn send_inputs(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut client: ResMut<RenetClient>,
    mut tick: ResMut<SnapshotTick>,
) {
    let Some(keyboard) = keyboard else { return };
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

/// Apply snapshots from the server: spawn/update/despawn sperm avatars
fn apply_snapshots(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut avatars: Query<(Entity, &mut Transform, &mut Sprite, &PlayerAvatar)>,
) {
    if !client.is_connected() {
        return;
    }

    while let Some(message) = client.receive_message(0) {
        if let Ok(msg) = bincode::deserialize::<ServerMessage>(&message) {
            match msg {
                ServerMessage::Snapshot { entities, .. } => {
                    let live_ids: Vec<u64> = entities.iter().map(|e| e.id).collect();

                    // Despawn avatars that no longer exist in snapshot
                    for (entity, _, _, avatar) in avatars.iter_mut() {
                        if !live_ids.iter().any(|id| *id == avatar.id) {
                            commands.entity(entity).despawn();
                        }
                    }

                    // Update or spawn avatars from snapshot
                    for snapshot in entities {
                        let pos = Vec3::new(snapshot.position.x, snapshot.position.y, 2.0);

                        if let Some((_, mut transform, mut sprite, _)) = avatars
                            .iter_mut()
                            .find(|(_, _, _, avatar)| avatar.id == snapshot.id)
                        {
                            transform.translation = pos;
                            sprite.color = color_for_region(snapshot.region);
                        } else {
                            commands.spawn((
                                Sprite::from_color(
                                    color_for_region(snapshot.region),
                                    Vec2::splat(PLAYER_RADIUS * 2.0),
                                ),
                                Transform::from_translation(pos),
                                PlayerAvatar { id: snapshot.id },
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Camera smoothly follows the center of all sperm
fn camera_follow_players(
    mut cameras: Query<&mut Transform, (With<Camera>, Without<PlayerAvatar>)>,
    avatars: Query<&Transform, (With<PlayerAvatar>, Without<Camera>)>,
) {
    let Ok(mut camera_tf) = cameras.single_mut() else {
        return;
    };

    if avatars.is_empty() {
        return;
    }

    let mut center = Vec3::ZERO;
    let mut count: f32 = 0.0;

    for transform in &avatars {
        center += transform.translation;
        count += 1.0;
    }

    center /= count.max(1.0);

    camera_tf.translation.x = center.x;
    camera_tf.translation.y = center.y;
}

/// Tiny pulse animation so the sperm feel alive
fn sperm_pulse_animation(time: Res<Time>, mut q: Query<(&PlayerAvatar, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (avatar, mut transform) in &mut q {
        let phase = avatar.id as f32 * 0.37;
        let scale = 1.0 + 0.08 * (t * 6.0 + phase).sin();
        transform.scale = Vec3::splat(scale);
    }
}

/// Nice color per region
fn color_for_region(region: RegionId) -> Color {
    match region {
        RegionId::Vagina => Color::srgb(0.20, 0.06, 0.30),
        RegionId::Cervix => Color::srgb(0.30, 0.10, 0.38),
        RegionId::Uterus => Color::srgb(0.40, 0.14, 0.44),
        RegionId::Utj => Color::srgb(0.48, 0.18, 0.48),
        RegionId::Tube => Color::srgb(0.56, 0.22, 0.52),
        RegionId::Ampulla => Color::srgb(0.65, 0.28, 0.56),
    }
}
