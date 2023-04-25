use crate::types::CameraFormat;
use bevy::prelude::*;

pub struct WebCameraPlugin;

impl Plugin for WebCameraPlugin {
    fn build(&self, app: &mut App) {}
}

#[derive(Component)]
pub struct GstCamera {
    /// camera index
    index: usize,
    format: CameraFormat,
}

impl GstCamera {
    pub fn new(index: usize, format: CameraFormat) -> Self {
        Self { index, format }
    }
}
