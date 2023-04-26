use crate::error::BevyGstError;
use crate::types::yuyv422_to_rgb;
use crate::types::{mjpeg_to_rgb24, CameraFormat, CameraInfo, FrameFormat};
use bevy::prelude::*;
use gstreamer::{
    element_error,
    glib::Cast,
    prelude::{DeviceExt, DeviceMonitorExt, DeviceMonitorExtManual, ElementExt, GstBinExt},
    Bin, Caps, DeviceMonitor, Element, FlowError, FlowSuccess, ResourceError, State,
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_video::{VideoFormat, VideoInfo};
use image::ImageBuffer;
use image::Rgb;
use std::sync::{Arc, Mutex};
type PipelineGenRet = (Element, AppSink, Arc<Mutex<ImageBuffer<Rgb<u8>, Vec<u8>>>>);

pub struct WebCameraPlugin;

impl Plugin for WebCameraPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component)]
pub struct GstCamera {
    /// camera index
    index: usize,
    pipeline: Element,
    app_sink: AppSink,
    camera_format: CameraFormat,
    camera_info: CameraInfo,
    image_lock: Arc<Mutex<ImageBuffer<Rgb<u8>, Vec<u8>>>>,
    caps: Option<Caps>,
}

impl GstCamera {
    pub fn new(index: usize, format: Option<CameraFormat>) -> Result<Self, BevyGstError> {
        let camera_format = match format {
            None => CameraFormat::default(),
            Some(fmt) => fmt,
        };

        if let Err(why) = gstreamer::init() {
            return Err(BevyGstError::InitializeError(why.to_string()));
        }

        let (camera_info, caps) = search_device(0).unwrap();

        let (pipeline, app_sink, receiver) = generate_pipeline(camera_format, index as usize)?;

        Ok(Self {
            index,
            pipeline,
            app_sink,
            camera_format,
            camera_info,
            image_lock: receiver,
            caps,
        })
    }

    pub fn open_stream(&mut self) -> Result<(), BevyGstError> {
        if let Err(why) = self.pipeline.set_state(State::Playing) {
            return Err(BevyGstError::OpenStreamError(format!(
                "Failed to set appsink to playing: {}",
                why
            )));
        }
        Ok(())
    }
}

fn search_device(index: usize) -> Result<(CameraInfo, Option<Caps>), BevyGstError> {
    let device_monitor = DeviceMonitor::new();

    if let Err(why) = device_monitor.start() {
        return Err(BevyGstError::StructureError {
            structure: "Device Monitor".to_string(),
            error: format!("Failed to start device monitor: {}", why),
        });
    }

    let device = match device_monitor
        .devices()
        .iter()
        .enumerate()
        .find(|(i, _device)| *i == index)
    {
        Some((_, dev)) => dev.clone(),
        None => {
            return Err(BevyGstError::OpenDeviceError(
                index.to_string(),
                "No device".to_string(),
            ))
        }
    };
    device_monitor.stop();

    let caps = device.caps();
    Ok((
        CameraInfo::new(
            &DeviceExt::display_name(&device),
            &DeviceExt::device_class(&device),
            &"",
            index,
        ),
        caps,
    ))
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::let_and_return)]
fn generate_pipeline(fmt: CameraFormat, index: usize) -> Result<PipelineGenRet, BevyGstError> {
    let pipeline =
        match gstreamer::parse_launch(webcam_pipeline(format!("{}", index).as_str(), fmt).as_str())
        {
            Ok(p) => p,
            Err(why) => {
                return Err(BevyGstError::OpenDeviceError(
                    index.to_string(),
                    format!(
                        "Failed to open pipeline with args {}: {}",
                        webcam_pipeline(format!("{}", index).as_str(), fmt),
                        why
                    ),
                ))
            }
        };

    let sink = match pipeline
        .clone()
        .dynamic_cast::<Bin>()
        .unwrap()
        .by_name("appsink")
    {
        Some(s) => s,
        None => {
            return Err(BevyGstError::OpenDeviceError(
                index.to_string(),
                "Failed to get sink element!".to_string(),
            ))
        }
    };

    let appsink = match sink.dynamic_cast::<AppSink>() {
        Ok(aps) => aps,
        Err(_) => {
            return Err(BevyGstError::OpenDeviceError(
                index.to_string(),
                "Failed to get sink element as appsink".to_string(),
            ))
        }
    };

    pipeline.set_state(State::Playing).unwrap();

    let image_lock = Arc::new(Mutex::new(ImageBuffer::default()));
    let img_lck_clone = image_lock.clone();

    appsink.set_callbacks(
        AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample = appsink.pull_sample().map_err(|_| FlowError::Eos)?;
                let sample_caps = if let Some(c) = sample.caps() {
                    c
                } else {
                    element_error!(
                        appsink,
                        ResourceError::Failed,
                        ("Failed to get caps from sample")
                    );
                    return Err(FlowError::Error);
                };

                let video_info = match VideoInfo::from_caps(sample_caps) {
                    Ok(vi) => vi,
                    Err(why) => {
                        element_error!(
                            appsink,
                            ResourceError::Failed,
                            (format!("Failed to get videoinfo from caps: {}", why).as_str())
                        );

                        return Err(FlowError::Error);
                    }
                };

                let buffer = if let Some(buf) = sample.buffer() {
                    buf
                } else {
                    element_error!(
                        appsink,
                        ResourceError::Failed,
                        ("Failed to get buffer from sample")
                    );
                    return Err(FlowError::Error);
                };

                let buffer_map = match buffer.map_readable() {
                    Ok(m) => m,
                    Err(why) => {
                        element_error!(
                            appsink,
                            ResourceError::Failed,
                            (format!("Failed to map buffer to readablemap: {}", why).as_str())
                        );

                        return Err(FlowError::Error);
                    }
                };

                let channels = if video_info.has_alpha() { 4 } else { 3 };

                let image_buffer = match video_info.format() {
                    VideoFormat::Yuy2 => {
                        let mut decoded_buffer = match yuyv422_to_rgb(&buffer_map, false) {
                            Ok(buf) => buf,
                            Err(why) => {
                                element_error!(
                                    appsink,
                                    ResourceError::Failed,
                                    (format!("Failed to make yuy2 into rgb888: {}", why).as_str())
                                );

                                return Err(FlowError::Error);
                            }
                        };

                        decoded_buffer.resize(
                            (video_info.width() * video_info.height() * channels) as usize,
                            0_u8,
                        );

                        let image = if let Some(i) = ImageBuffer::from_vec(
                            video_info.width(),
                            video_info.height(),
                            decoded_buffer,
                        ) {
                            let rgb: ImageBuffer<Rgb<u8>, Vec<u8>> = i;
                            rgb
                        } else {
                            element_error!(
                                appsink,
                                ResourceError::Failed,
                                ("Failed to make rgb buffer into imagebuffer")
                            );

                            return Err(FlowError::Error);
                        };
                        image
                    }
                    VideoFormat::Rgb => {
                        let mut decoded_buffer = buffer_map.as_slice().to_vec();
                        decoded_buffer.resize(
                            (video_info.width() * video_info.height() * channels) as usize,
                            0_u8,
                        );
                        let image = if let Some(i) = ImageBuffer::from_vec(
                            video_info.width(),
                            video_info.height(),
                            decoded_buffer,
                        ) {
                            let rgb: ImageBuffer<Rgb<u8>, Vec<u8>> = i;
                            rgb
                        } else {
                            element_error!(
                                appsink,
                                ResourceError::Failed,
                                ("Failed to make rgb buffer into imagebuffer")
                            );

                            return Err(FlowError::Error);
                        };
                        image
                    }
                    // MJPEG
                    VideoFormat::Encoded => {
                        let mut decoded_buffer = match mjpeg_to_rgb24(&buffer_map) {
                            Ok(buf) => buf,
                            Err(why) => {
                                element_error!(
                                    appsink,
                                    ResourceError::Failed,
                                    (format!("Failed to make yuy2 into rgb888: {}", why).as_str())
                                );

                                return Err(FlowError::Error);
                            }
                        };

                        decoded_buffer.resize(
                            (video_info.width() * video_info.height() * channels) as usize,
                            0_u8,
                        );

                        let image = if let Some(i) = ImageBuffer::from_vec(
                            video_info.width(),
                            video_info.height(),
                            decoded_buffer,
                        ) {
                            let rgb: ImageBuffer<Rgb<u8>, Vec<u8>> = i;
                            rgb
                        } else {
                            element_error!(
                                appsink,
                                ResourceError::Failed,
                                ("Failed to make rgb buffer into imagebuffer")
                            );

                            return Err(FlowError::Error);
                        };
                        image
                    }
                    _ => {
                        element_error!(
                            appsink,
                            ResourceError::Failed,
                            ("Unsupported video format")
                        );
                        return Err(FlowError::Error);
                    }
                };

                *img_lck_clone.lock().unwrap() = image_buffer;

                Ok(FlowSuccess::Ok)
            })
            .build(),
    );
    Ok((pipeline, appsink, image_lock))
}

#[cfg(target_os = "macos")]
fn webcam_pipeline(device: &str, camera_format: CameraFormat) -> String {
    match camera_format.format() {
        FrameFormat::MJPEG => {
            format!("autovideosrc location=/dev/video{} ! image/jpeg,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        FrameFormat::YUYV => {
            format!("autovideosrc location=/dev/video{} ! video/x-raw,format=YUY2,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        _ => {
            format!("unsupproted! if you see this, switch to something else!")
        }
    }
}

#[cfg(target_os = "linux")]
fn webcam_pipeline(device: &str, camera_format: CameraFormat) -> String {
    match camera_format.format() {
        FrameFormat::MJPEG => {
            format!("v4l2src device=/dev/video{} ! image/jpeg, width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        FrameFormat::YUYV => {
            format!("v4l2src device=/dev/video{} ! video/x-raw,format=YUY2,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        _ => {
            format!("unsupproted! if you see this, switch to something else!")
        }
    }
}

#[cfg(target_os = "windows")]
fn webcam_pipeline(device: &str, camera_format: CameraFormat) -> String {
    match camera_format.format() {
        FrameFormat::MJPEG => {
            format!("ksvideosrc device_index={} ! image/jpeg, width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        FrameFormat::YUYV => {
            format!("ksvideosrc device_index={} ! video/x-raw,format=YUY2,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        _ => {
            format!("unsupproted! if you see this, switch to something else!")
        }
    }
}