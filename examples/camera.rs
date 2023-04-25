use bevy::prelude::*;
use bevy_gstreamer::camera::GstCamera;
use bevy_gstreamer::types::{CameraFormat, FrameFormat, Resolution};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_camera)
        .run()
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(GstCamera::new(
        0,
        CameraFormat::new(Resolution::new(640, 480), FrameFormat::MJPEG, 30),
    ));
}
