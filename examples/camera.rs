use bevy::prelude::*;

use bevy_gstreamer::camera::{BackgroundImageMarker, GstCamera};
use bevy_gstreamer::types::{CameraFormat, FrameFormat};
use bevy_gstreamer::GstreamerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "gstreamer capture".into(),
                resolution: (640., 480.).into(),

                ..default()
            }),
            ..default()
        }))
        .add_plugins(GstreamerPlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Update, camera_control)
        .run();
}

fn setup_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut camera = GstCamera::new(
        0,
        Some(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)),
    )
    .expect("cannot find any camera");

    camera.open_stream().unwrap();
    commands.spawn((camera, BackgroundImageMarker));

    commands.spawn(Camera3dBundle {
        camera_3d: Camera3d {
            // clear_color: ClearColorConfig::None,
            ..default()
        },
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial {
            base_color: bevy::color::palettes::css::SEA_GREEN.into(),
            unlit: true,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
}

fn camera_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut q_camera: Query<&mut GstCamera, With<BackgroundImageMarker>>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        if let Ok(mut cam) = q_camera.get_single_mut() {
            info!("start stream");
            cam.open_stream().unwrap();
        }
    }

    if keyboard_input.just_pressed(KeyCode::Digit2) {
        if let Ok(mut cam) = q_camera.get_single_mut() {
            info!("stop stream");
            cam.stop_stream().unwrap();
        }
    }

    if keyboard_input.just_pressed(KeyCode::Digit3) {
        if let Ok(mut cam) = q_camera.get_single_mut() {
            info!("change capture resolution to 1920x1080 30fps");
            cam.set_camera_format(CameraFormat::new_from(1920, 1080, FrameFormat::MJPEG, 30))
                .unwrap();
        }
    }

    if keyboard_input.just_pressed(KeyCode::Digit4) {
        if let Ok(mut cam) = q_camera.get_single_mut() {
            info!("change capture resolution to 640x480 30fps");
            cam.set_camera_format(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30))
                .unwrap();
        }
    }
}
