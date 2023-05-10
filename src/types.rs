use crate::error::BevyGstError;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

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

impl Display for FrameFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameFormat::MJPEG => {
                write!(f, "MJPG")
            }
            FrameFormat::YUYV => {
                write!(f, "YUYV")
            }
            FrameFormat::GRAY => {
                write!(f, "GRAY")
            }
            FrameFormat::RAWRGB => {
                write!(f, "RAWRGB")
            }
            FrameFormat::NV12 => {
                write!(f, "NV12")
            }
        }
    }
}

impl FromStr for FrameFormat {
    type Err = BevyGstError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MJPEG" => Ok(FrameFormat::MJPEG),
            "YUYV" => Ok(FrameFormat::YUYV),
            "GRAY" => Ok(FrameFormat::GRAY),
            "RAWRGB" => Ok(FrameFormat::RAWRGB),
            "NV12" => Ok(FrameFormat::NV12),
            _ => Err(BevyGstError::StructureError {
                structure: "FrameFormat".to_string(),
                error: format!("No match for {s}"),
            }),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct CameraFormat {
    resolution: Resolution,
    format: FrameFormat,
    frame_rate: u32,
}

impl Default for CameraFormat {
    fn default() -> Self {
        Self {
            resolution: Resolution::new(640, 480),
            format: FrameFormat::MJPEG,
            frame_rate: 30,
        }
    }
}

impl CameraFormat {
    /// create a new CameraFormat
    pub fn new(resolution: Resolution, format: FrameFormat, frame_rate: u32) -> Self {
        Self {
            resolution,
            format,
            frame_rate,
        }
    }

    /// create a new CameraFormat from a resolution and a format
    pub fn new_from(res_x: u32, res_y: u32, format: FrameFormat, fps: u32) -> Self {
        CameraFormat {
            resolution: Resolution {
                width_x: res_x,
                height_y: res_y,
            },
            format,
            frame_rate: fps,
        }
    }

    /// get camera resolution width
    pub fn width(&self) -> u32 {
        self.resolution.width_x
    }

    /// get camera resolution height
    pub fn height(&self) -> u32 {
        self.resolution.height_y
    }

    /// get camera resolution
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// get camera frame rate
    pub fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    /// get camera frame format
    pub fn format(&self) -> FrameFormat {
        self.format
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd)]
pub struct CameraInfo {
    human_name: String,
    description: String,
    misc: String,
    index: usize,
}

impl CameraInfo {
    pub fn new(human_name: &str, description: &str, misc: &str, index: usize) -> Self {
        CameraInfo {
            human_name: human_name.to_string(),
            description: description.to_string(),
            misc: misc.to_string(),
            index,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum CameraIndex {
    Index(u32),
    String(String),
}

pub fn mjpeg_to_rgb24(in_buf: &[u8]) -> Result<Vec<u8>, BevyGstError> {
    let mut decoder = jpeg_decoder::Decoder::new(in_buf);

    let d = match decoder.decode() {
        Ok(d) => d,
        Err(err) => {
            return Err(BevyGstError::ProcessFrameError {
                src: FrameFormat::MJPEG,
                destination: "RGB888".to_string(),
                error: format!("Could not decode MJPEG: {}", err),
            });
        }
    };

    Ok(d)
}
pub fn yuyv422_to_rgb(data: &[u8], rgba: bool) -> Result<Vec<u8>, BevyGstError> {
    if data.len() % 4 != 0 {
        return Err(BevyGstError::ProcessFrameError {
            src: FrameFormat::YUYV,
            destination: "RGB888".to_string(),
            error: "Assertion failure, the YUV stream isn't 4:2:2! (wrong number of bytes)"
                .to_string(),
        });
    }

    let pixel_size = if rgba { 4 } else { 3 };
    // yuyv yields 2 3-byte pixels per yuyv chunk
    let rgb_buf_size = (data.len() / 4) * (2 * pixel_size);

    let mut dest = vec![0; rgb_buf_size];
    buf_yuyv422_to_rgb(data, &mut dest, rgba)?;

    Ok(dest)
}

/// Same as [`yuyv422_to_rgb`] but with a destination buffer instead of a return `Vec<u8>`
/// # Errors
/// If the stream is invalid YUYV, or the destination buffer is not large enough, this will error.
pub fn buf_yuyv422_to_rgb(data: &[u8], dest: &mut [u8], rgba: bool) -> Result<(), BevyGstError> {
    if data.len() % 4 != 0 {
        return Err(BevyGstError::ProcessFrameError {
            src: FrameFormat::YUYV,
            destination: "RGB888".to_string(),
            error: "Assertion failure, the YUV stream isn't 4:2:2! (wrong number of bytes)"
                .to_string(),
        });
    }

    let pixel_size = if rgba { 4 } else { 3 };
    // yuyv yields 2 3-byte pixels per yuyv chunk
    let rgb_buf_size = (data.len() / 4) * (2 * pixel_size);

    if dest.len() != rgb_buf_size {
        return Err(BevyGstError::ProcessFrameError {
            src: FrameFormat::YUYV,
            destination: "RGB888".to_string(),
            error: format!("Assertion failure, the destination RGB buffer is of the wrong size! [expected: {rgb_buf_size}, actual: {}]", dest.len()),
        });
    }

    let iter = data.chunks_exact(4);

    if rgba {
        let mut iter = iter
            .flat_map(|yuyv| {
                let y1 = i32::from(yuyv[0]);
                let u = i32::from(yuyv[1]);
                let y2 = i32::from(yuyv[2]);
                let v = i32::from(yuyv[3]);
                let pixel1 = yuyv444_to_rgba(y1, u, v);
                let pixel2 = yuyv444_to_rgba(y2, u, v);
                [pixel1, pixel2]
            })
            .flatten();
        for i in dest.iter_mut().take(rgb_buf_size) {
            *i = match iter.next() {
                Some(v) => v,
                None => {
                    return Err(BevyGstError::ProcessFrameError {
                        src: FrameFormat::YUYV,
                        destination: "RGBA8888".to_string(),
                        error: "Ran out of RGBA YUYV values! (this should not happen, please file an issue: l1npengtul/nokhwa)".to_string()
                    })
                }
            }
        }
    } else {
        let mut iter = iter
            .flat_map(|yuyv| {
                let y1 = i32::from(yuyv[0]);
                let u = i32::from(yuyv[1]);
                let y2 = i32::from(yuyv[2]);
                let v = i32::from(yuyv[3]);
                let pixel1 = yuyv444_to_rgb(y1, u, v);
                let pixel2 = yuyv444_to_rgb(y2, u, v);
                [pixel1, pixel2]
            })
            .flatten();

        for i in dest.iter_mut().take(rgb_buf_size) {
            *i = match iter.next() {
                Some(v) => v,
                None => {
                    return Err(BevyGstError::ProcessFrameError {
                        src: FrameFormat::YUYV,
                        destination: "RGB888".to_string(),
                        error: "Ran out of RGB YUYV values! (this should not happen, please file an issue: l1npengtul/nokhwa)".to_string()
                    })
                }
            }
        }
    }

    Ok(())
}

// equation from https://en.wikipedia.org/wiki/YUV#Converting_between_Y%E2%80%B2UV_and_RGB
/// Convert `YCbCr` 4:4:4 to a RGB888. [For further reading](https://en.wikipedia.org/wiki/YUV#Converting_between_Y%E2%80%B2UV_and_RGB)
#[allow(clippy::many_single_char_names)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
#[must_use]
#[inline]
pub fn yuyv444_to_rgb(y: i32, u: i32, v: i32) -> [u8; 3] {
    let c298 = (y - 16) * 298;
    let d = u - 128;
    let e = v - 128;
    let r = ((c298 + 409 * e + 128) >> 8) as u8;
    let g = ((c298 - 100 * d - 208 * e + 128) >> 8) as u8;
    let b = ((c298 + 516 * d + 128) >> 8) as u8;
    [r, g, b]
}

// equation from https://en.wikipedia.org/wiki/YUV#Converting_between_Y%E2%80%B2UV_and_RGB
/// Convert `YCbCr` 4:4:4 to a RGBA8888. [For further reading](https://en.wikipedia.org/wiki/YUV#Converting_between_Y%E2%80%B2UV_and_RGB)
///
/// Equivalent to [`yuyv444_to_rgb`] but with an alpha channel attached.
#[allow(clippy::many_single_char_names)]
#[must_use]
#[inline]
pub fn yuyv444_to_rgba(y: i32, u: i32, v: i32) -> [u8; 4] {
    let [r, g, b] = yuyv444_to_rgb(y, u, v);
    [r, g, b, 255]
}
