#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Resolution {
    pub width_x: u32,
    pub height_y: u32,
}

impl Resolution {
    pub fn new(x: u32, y: u32) -> Self {
        Self {
            width_x: x,
            height_y: y,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum FrameFormat {
    MJPEG,
    YUYV,
    NV12,
    GRAY,
    RAWRGB,
}


#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct CameraFormat {
    resolution: Resolution,
    format: FrameFormat,
    frame_rate: u32,
}

impl CameraFormat {
    pub fn new(resolution: Resolution, format: FrameFormat, frame_rate: u32) -> Self {
        Self {
            resolution,
            format,
            frame_rate,
        }
    }
}