use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::prelude::*;
use bevy_gstreamer::camera::{BackgroundImageMarker, GstCamera};
use bevy_gstreamer::GstreamerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GstreamerPlugin)
        .add_startup_system(setup_camera)
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
