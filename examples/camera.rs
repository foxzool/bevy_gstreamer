use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::prelude::*;
use bevy_gstreamer::camera::{BackgroundImageMarker, GstCamera};
use bevy_gstreamer::GstreamerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GstreamerPlugin)
        .add_startup_system(setup_camera)
        .add_system(camera_control)
        .run()
}

fn setup_camera(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut camera = GstCamera::new(0, None).unwrap();
    camera.open_stream().unwrap();
    commands
        .spawn(Camera3dBundle {
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::None,
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert((camera, BackgroundImageMarker));

    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::SEA_GREEN,
            unlit: true,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
}

fn camera_control(
    keyboard_input: Res<Input<KeyCode>>,
    mut q_camera: Query<&mut GstCamera, With<BackgroundImageMarker>>,
) {
    if keyboard_input.just_pressed(KeyCode::Key1) {
        if let Ok(mut cam) = q_camera.get_single_mut() {
            info!("start stream");
            cam.open_stream().unwrap();
        }
    }

    if keyboard_input.just_pressed(KeyCode::Key2) {
        if let Ok(mut cam) = q_camera.get_single_mut() {
            info!("stop stream");
            cam.stop_stream().unwrap();
        }
    }
}
