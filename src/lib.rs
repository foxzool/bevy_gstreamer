use bevy::app::{App, Plugin};

pub mod capture;
pub mod camera;
pub mod types;

pub struct GstreamerPlugin;

impl Plugin for GstreamerPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}