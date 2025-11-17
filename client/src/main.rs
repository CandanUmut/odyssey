use std::{net::UdpSocket, time::SystemTime};

use bevy::math::primitives::{Capsule3d, Cylinder, Sphere};
use bevy::prelude::*;
use bevy::time::Fixed;
use bevy::window::WindowResolution;
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
    client_id: u64,
}

#[derive(Resource, Default)]
struct SnapshotTick(u32);

#[derive(Component)]
struct PlayerAvatar {
    id: u64,
}

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec3);

#[derive(Component)]
struct FollowCamera {
    target: u64,
    distance: f32,
    height: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Odyssey: Race to the Egg".into(),
                resolution: WindowResolution::new(1280, 720),
                resizable: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(RenetClientPlugin)
        .add_plugins(NetcodeClientPlugin)
        .insert_resource(SnapshotTick::default())
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.08)))
        .add_systems(Startup, (setup_scene, start_connection))
        .add_systems(
            Update,
            (
                poll_connection_status,
                apply_snapshots,
                assign_follow_target,
                camera_follow_target,
            ),
        )
        .insert_resource(Time::<Fixed>::from_hz(TICK_RATE as f64))
        .add_systems(FixedUpdate, send_inputs)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-200.0, 60.0, 160.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        FollowCamera {
            target: 0,
            distance: 140.0,
            height: 32.0,
        },
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 25_000.0,
            ..Default::default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.5, -0.6, 0.0)),
    ));

    for i in 0..4 {
        let phase = i as f32 * 0.7;
        commands.spawn((
            PointLight {
                intensity: 2_200.0,
                range: 320.0,
                color: Color::srgb(0.8 - 0.1 * phase, 0.3 + 0.1 * phase, 0.9),
                shadows_enabled: false,
                ..Default::default()
            },
            Transform::from_xyz(-200.0 + i as f32 * 320.0, 40.0, 90.0),
        ));
    }

    spawn_tunnel(&mut commands, &mut meshes, &mut materials);
    spawn_egg(&mut commands, &mut meshes, &mut materials);
}

fn spawn_tunnel(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let regions = [
        RegionId::Vagina,
        RegionId::Cervix,
        RegionId::Uterus,
        RegionId::Utj,
        RegionId::Tube,
        RegionId::Ampulla,
    ];

    for window in REGION_MARKERS.windows(2).enumerate() {
        let (idx, segment) = window;
        let start = segment[0];
        let end = segment[1];
        let length = end - start;
        let center = start + length * 0.5;
        let region = regions[idx].clone();
        let radius = tube_radius(region);

        let mesh_handle = meshes.add(Cylinder::new(radius, length));
        let mut material = StandardMaterial::from(color_for_region(region.clone()));
        material.perceptual_roughness = 0.6;
        material.metallic = 0.05;
        material.emissive = color_for_region(region).into();
        let material_handle = materials.add(material);

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_xyz(center, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
        ));
    }
}

fn spawn_egg(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let egg_pos = Vec3::new(TRACK_LENGTH + 120.0, 0.0, 0.0);
    let egg_mesh = meshes.add(Mesh::from(Sphere::new(48.0)));
    let egg_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.9, 0.8),
        emissive: Color::srgb(1.0, 0.9, 0.7).into(),
        perceptual_roughness: 0.3,
        metallic: 0.05,
        ..Default::default()
    });

    commands.spawn((
        Mesh3d(egg_mesh),
        MeshMaterial3d(egg_mat),
        Transform::from_translation(egg_pos),
    ));
}

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
        client_id,
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

fn apply_snapshots(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut avatars: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &PlayerAvatar,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Without<Camera>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !client.is_connected() {
        return;
    }

    while let Some(message) = client.receive_message(0) {
        if let Ok(msg) = bincode::deserialize::<ServerMessage>(&message) {
            if let ServerMessage::Snapshot { entities, .. } = msg {
                let live_ids: Vec<u64> = entities.iter().map(|e| e.id).collect();

                for (entity, _, _, avatar, _) in avatars.iter_mut() {
                    if !live_ids.iter().any(|id| *id == avatar.id) {
                        commands.entity(entity).despawn();
                    }
                }

                for snapshot in entities {
                    let pos = Vec3::from(snapshot.position);
                    let vel = Vec3::from(snapshot.velocity);
                    let region = snapshot.region.clone();
                    if let Some((_, mut transform, mut velocity, _, material)) = avatars
                        .iter_mut()
                        .find(|(_, _, _, avatar, _)| avatar.id == snapshot.id)
                    {
                        transform.translation = pos;
                        **velocity = vel;
                        if let Some(mat) = materials.get_mut(&material.0) {
                            mat.base_color = color_for_region(region.clone());
                            mat.emissive = color_for_region(region).into();
                        }
                    } else {
                        spawn_avatar(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            snapshot.id,
                            pos,
                            vel,
                            region,
                        );
                    }
                }
            }
        }
    }
}

fn spawn_avatar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    id: u64,
    pos: Vec3,
    velocity: Vec3,
    region: RegionId,
) {
    let body = meshes.add(Mesh::from(Capsule3d::new(
        PLAYER_RADIUS * 0.75,
        PLAYER_RADIUS * 2.0,
    )));
    let mut material = StandardMaterial::from(color_for_region(region));
    material.perceptual_roughness = 0.4;
    let material_handle = materials.add(material);

    commands.spawn((
        Mesh3d(body),
        MeshMaterial3d(material_handle),
        Transform::from_translation(pos),
        PlayerAvatar { id },
        Velocity(velocity),
    ));
}

fn assign_follow_target(player: Option<Res<LocalPlayer>>, mut cameras: Query<&mut FollowCamera>) {
    let Some(player) = player else { return };
    let Ok(mut follow) = cameras.single_mut() else {
        return;
    };
    if follow.target == 0 {
        follow.target = player.client_id;
    }
}

fn camera_follow_target(
    mut cameras: Query<(&mut Transform, &FollowCamera)>,
    avatars: Query<(&PlayerAvatar, &Transform, &Velocity)>,
) {
    let Ok((mut cam_tf, follow)) = cameras.single_mut() else {
        return;
    };

    let Some((_, target_tf, vel)) = avatars
        .iter()
        .find(|(avatar, _, _)| avatar.id == follow.target)
    else {
        return;
    };

    let forward = if vel.length_squared() > 1.0 {
        vel.normalize()
    } else {
        Vec3::X
    };

    let desired = target_tf.translation - forward * follow.distance + Vec3::Y * follow.height;
    cam_tf.translation = cam_tf.translation.lerp(desired, 0.08);
    cam_tf.look_at(target_tf.translation + forward * 20.0, Vec3::Y);
}

fn color_for_region(region: RegionId) -> Color {
    match region {
        RegionId::Vagina => Color::srgb(0.45, 0.14, 0.30),
        RegionId::Cervix => Color::srgb(0.55, 0.18, 0.34),
        RegionId::Uterus => Color::srgb(0.62, 0.22, 0.40),
        RegionId::Utj => Color::srgb(0.66, 0.26, 0.44),
        RegionId::Tube => Color::srgb(0.58, 0.30, 0.60),
        RegionId::Ampulla => Color::srgb(0.72, 0.36, 0.70),
    }
}
