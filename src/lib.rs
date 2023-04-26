#![feature(portable_simd)]

use bevy::app::{App, Plugin};

pub mod camera;
pub mod error;
pub mod types;

pub struct GstreamerPlugin;

impl Plugin for GstreamerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(camera::WebCameraPlugin);
    }
}
