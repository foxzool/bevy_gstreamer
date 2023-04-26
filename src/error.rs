use crate::types::FrameFormat;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum BevyGstError {
    #[error("Could not initialize gstreamer: {0}")]
    InitializeError(String),
    #[error("Error: {0}")]
    GeneralError(String),
    #[error("Could not generate required structure {structure}: {error}")]
    StructureError { structure: String, error: String },
    #[error("Could not open device {0}: {1}")]
    OpenDeviceError(String, String),
    #[error("Could not process frame {src} to {destination}: {error}")]
    ProcessFrameError {
        src: FrameFormat,
        destination: String,
        error: String,
    },
    #[error("Could not open device stream: {0}")]
    OpenStreamError(String),
}
