use bevy::prelude::*;
use bevy_gstreamer::camera::GstCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_camera)
        .run()
}

fn setup_camera(mut commands: Commands) {
    let mut camera = GstCamera::new(0, None).unwrap();
    camera.open_stream().unwrap();
    commands.spawn(camera);
}
