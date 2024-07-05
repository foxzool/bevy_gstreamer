use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use bevy::core_pipeline;
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResourcePlugin;
use bevy::render::render_graph::RenderGraph;
use bevy::render::RenderApp;
use glib::Quark;
use gstreamer::prelude::*;
use gstreamer::{
    element_error,
    prelude::{DeviceExt, DeviceMonitorExt, DeviceMonitorExtManual, ElementExt, GstBinExt},
    Bin, Caps, ClockTime, DeviceMonitor, Element, FlowError, FlowSuccess, MessageView,
    ResourceError, State,
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_video::{VideoFormat, VideoInfo};
use image::ImageBuffer;
use image::{Rgb, RgbaImage};
use regex::Regex;

use crate::camera::background::*;
use crate::error::BevyGstError;
use crate::types::{mjpeg_to_rgb24, CameraFormat, CameraInfo, FrameFormat};
use crate::types::{yuyv422_to_rgb, Resolution};

type PipelineGenRet = (Element, AppSink, Arc<Mutex<ImageBuffer<Rgb<u8>, Vec<u8>>>>);

mod background;

pub struct WebCameraPlugin;

impl Plugin for WebCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BackgroundImage(RgbaImage::new(640, 480)))
            .add_plugins(ExtractResourcePlugin::<BackgroundImage>::default())
            .add_systems(Update, handle_background_image);

        let render_app = app.sub_app_mut(RenderApp);
        let background_node_2d = BackgroundNode::new(render_app.world_mut());
        let background_node_3d = BackgroundNode::new(render_app.world_mut());
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();

        if let Some(graph_2d) =
            render_graph.get_sub_graph_mut(core_pipeline::core_2d::graph::Core2d)
        {
            graph_2d.add_node(BackgroundNodeLabel, background_node_2d);

            graph_2d.add_node_edge(
                BackgroundNodeLabel,
                core_pipeline::core_2d::graph::Node2d::StartMainPass,
            );
        }

        if let Some(graph_3d) =
            render_graph.get_sub_graph_mut(core_pipeline::core_3d::graph::Core3d)
        {
            graph_3d.add_node(BackgroundNodeLabel, background_node_3d);

            graph_3d.add_node_edge(
                BackgroundNodeLabel,
                core_pipeline::core_3d::graph::Node3d::MainTransparentPass,
            );
        }
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<BackgroundPipeline>();
    }
}

#[derive(Component)]
pub struct BackgroundImageMarker;

/// A camera from gstreamer pipeline
#[derive(Component)]
#[allow(dead_code)]
pub struct GstCamera {
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
        let camera_format = format.unwrap_or_default();

        if let Err(why) = gstreamer::init() {
            return Err(BevyGstError::InitializeError(why.to_string()));
        }

        let (camera_info, caps) = search_device(index)?;

        let (pipeline, app_sink, receiver) = generate_pipeline(camera_format, index)?;

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

    /// open capture stream
    pub fn open_stream(&mut self) -> Result<(), BevyGstError> {
        if let Err(why) = self.pipeline.set_state(State::Playing) {
            return Err(BevyGstError::OpenStreamError(format!(
                "Failed to set appsink to playing: {}",
                why
            )));
        }
        Ok(())
    }

    /// check device stream is opening
    pub fn is_stream_open(&self) -> bool {
        self.pipeline.current_state() == State::Playing
    }

    /// get rgb image from device
    pub fn frame(&mut self) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, BevyGstError> {
        let cam_fmt = self.camera_format;
        let image_data = self.frame_raw()?;
        let imagebuf =
            match ImageBuffer::from_vec(cam_fmt.width(), cam_fmt.height(), image_data.to_vec()) {
                Some(buf) => {
                    let rgb: ImageBuffer<Rgb<u8>, Vec<u8>> = buf;
                    rgb
                }
                None => return Err(BevyGstError::ReadFrameError(
                    "Imagebuffer is not large enough! This is probably a bug, please report it!"
                        .to_string(),
                )),
            };
        Ok(imagebuf)
    }

    /// raw data from device
    pub fn frame_raw(&mut self) -> Result<Cow<[u8]>, BevyGstError> {
        let bus = match self.pipeline.bus() {
            Some(bus) => bus,
            None => {
                return Err(BevyGstError::ReadFrameError(
                    "The pipeline has no bus!".to_string(),
                ))
            }
        };

        if let Some(message) = bus.timed_pop(ClockTime::from_seconds(0)) {
            match message.view() {
                MessageView::Eos(..) => {
                    return Err(BevyGstError::ReadFrameError("Stream is ended!".to_string()))
                }
                MessageView::Error(err) => {
                    return Err(BevyGstError::ReadFrameError(format!(
                        "Bus error: {}",
                        err.error()
                    )));
                }
                _ => {}
            }
        }

        Ok(Cow::from(self.image_lock.lock().unwrap().to_vec()))
    }

    /// stop device stream
    pub fn stop_stream(&mut self) -> Result<(), BevyGstError> {
        if let Err(why) = self.pipeline.set_state(State::Null) {
            return Err(BevyGstError::StreamShutdownError(format!(
                "Could not change state: {}",
                why
            )));
        }
        Ok(())
    }

    /// get camera index
    pub fn index(&self) -> usize {
        self.index
    }

    /// get camera infomation
    pub fn camera_info(&self) -> &CameraInfo {
        &self.camera_info
    }

    /// get camera format
    pub fn camera_format(&self) -> CameraFormat {
        self.camera_format
    }

    /// set camera a new format
    pub fn set_camera_format(&mut self, new_fmt: CameraFormat) -> Result<(), BevyGstError> {
        let mut reopen = false;
        if self.is_stream_open() {
            println!("is open");

            self.stop_stream()?;
            reopen = true;
        }
        let (pipeline, app_sink, receiver) = generate_pipeline(new_fmt, self.camera_info.index())?;
        self.pipeline = pipeline;
        self.app_sink = app_sink;
        self.image_lock = receiver;
        if reopen {
            self.open_stream()?;
        }

        self.camera_format = new_fmt;
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(dead_code)]
    fn compatible_list_by_resolution(
        &mut self,
        fourcc: FrameFormat,
    ) -> Result<HashMap<Resolution, Vec<u32>>, BevyGstError> {
        let mut resolution_map = HashMap::new();

        let frame_regex = match Regex::new(r"(\d+/1)|((\d+/\d)+(\d/1)*)") {
            Ok(r) => r,
            Err(_why) => return Err(BevyGstError::GeneralError("Regex error".to_string())),
        };

        match self.caps.clone() {
            Some(c) => {
                for capability in c.iter() {
                    match fourcc {
                        FrameFormat::MJPEG => {
                            if capability.name() == "image/jpeg" {
                                let mut fps_vec = vec![];

                                let width = match capability.get::<i32>("width") {
                                    Ok(w) => w,
                                    Err(why) => {
                                        return Err(BevyGstError::GetPropertyError {
                                            property: "Capibilities by Resolution: Width"
                                                .to_string(),
                                            error: why.to_string(),
                                        })
                                    }
                                };
                                let height = match capability.get::<i32>("height") {
                                    Ok(w) => w,
                                    Err(why) => {
                                        return Err(BevyGstError::GetPropertyError {
                                            property: "Capibilities by Resolution: Height"
                                                .to_string(),
                                            error: why.to_string(),
                                        })
                                    }
                                };
                                let value =
                                    match capability.value_by_quark(Quark::from_str("framerate")) {
                                        Ok(v) => match v.transform::<String>() {
                                            Ok(s) => {
                                                format!("{:?}", s)
                                            }
                                            Err(why) => {
                                                return Err(BevyGstError::GetPropertyError {
                                                    property: "Framerates".to_string(),
                                                    error: format!(
                                                        "Failed to make framerates into string: {}",
                                                        why
                                                    ),
                                                });
                                            }
                                        },
                                        Err(_) => {
                                            return Err(BevyGstError::GetPropertyError {
                                                property: "Framerates".to_string(),
                                                error: "Failed to get framerates: doesnt exist!"
                                                    .to_string(),
                                            })
                                        }
                                    };

                                for m in frame_regex.find_iter(&value) {
                                    let fraction_string: Vec<&str> =
                                        m.as_str().split('/').collect();
                                    if fraction_string.len() != 2 {
                                        return Err(BevyGstError::GetPropertyError { property: "Framerates".to_string(), error: format!("Fraction framerate had more than one demoninator: {:?}", fraction_string) });
                                    }

                                    if let Some(v) = fraction_string.get(1) {
                                        if *v != "1" {
                                            continue; // swallow error
                                        }
                                    } else {
                                        return Err(BevyGstError::GetPropertyError { property: "Framerates".to_string(), error: "No framerate denominator? Shouldn't happen, please report!".to_string() });
                                    }

                                    if let Some(numerator) = fraction_string.first() {
                                        match numerator.parse::<u32>() {
                                            Ok(fps) => fps_vec.push(fps),
                                            Err(why) => {
                                                return Err(BevyGstError::GetPropertyError {
                                                    property: "Framerates".to_string(),
                                                    error: format!(
                                                        "Failed to parse numerator: {}",
                                                        why
                                                    ),
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(BevyGstError::GetPropertyError { property: "Framerates".to_string(), error: "No framerate numerator? Shouldn't happen, please report!".to_string() });
                                    }
                                }
                                resolution_map
                                    .insert(Resolution::new(width as u32, height as u32), fps_vec);
                            }
                        }
                        FrameFormat::YUYV => {
                            if capability.name() == "video/x-raw"
                                && capability.get::<String>("format").unwrap_or_default() == *"YUY2"
                            {
                                let mut fps_vec = vec![];

                                let width = match capability.get::<i32>("width") {
                                    Ok(w) => w,
                                    Err(why) => {
                                        return Err(BevyGstError::GetPropertyError {
                                            property: "Capibilities by Resolution: Width"
                                                .to_string(),
                                            error: why.to_string(),
                                        })
                                    }
                                };
                                let height = match capability.get::<i32>("height") {
                                    Ok(w) => w,
                                    Err(why) => {
                                        return Err(BevyGstError::GetPropertyError {
                                            property: "Capibilities by Resolution: Height"
                                                .to_string(),
                                            error: why.to_string(),
                                        })
                                    }
                                };
                                let value =
                                    match capability.value_by_quark(Quark::from_str("framerate")) {
                                        Ok(v) => match v.transform::<String>() {
                                            Ok(s) => {
                                                format!("{:?}", s)
                                            }
                                            Err(why) => {
                                                return Err(BevyGstError::GetPropertyError {
                                                    property: "Framerates".to_string(),
                                                    error: format!(
                                                        "Failed to make framerates into string: {}",
                                                        why
                                                    ),
                                                });
                                            }
                                        },
                                        Err(_) => {
                                            return Err(BevyGstError::GetPropertyError {
                                                property: "Framerates".to_string(),
                                                error: "Failed to get framerates: doesnt exist!"
                                                    .to_string(),
                                            })
                                        }
                                    };

                                for m in frame_regex.find_iter(&value) {
                                    let fraction_string: Vec<&str> =
                                        m.as_str().split('/').collect();
                                    if fraction_string.len() != 2 {
                                        return Err(BevyGstError::GetPropertyError { property: "Framerates".to_string(), error: format!("Fraction framerate had more than one demoninator: {:?}", fraction_string) });
                                    }

                                    if let Some(v) = fraction_string.get(1) {
                                        if *v != "1" {
                                            continue; // swallow error
                                        }
                                    } else {
                                        return Err(BevyGstError::GetPropertyError { property: "Framerates".to_string(), error: "No framerate denominator? Shouldn't happen, please report!".to_string() });
                                    }

                                    if let Some(numerator) = fraction_string.first() {
                                        match numerator.parse::<u32>() {
                                            Ok(fps) => fps_vec.push(fps),
                                            Err(why) => {
                                                return Err(BevyGstError::GetPropertyError {
                                                    property: "Framerates".to_string(),
                                                    error: format!(
                                                        "Failed to parse numerator: {}",
                                                        why
                                                    ),
                                                });
                                            }
                                        }
                                    } else {
                                        return Err(BevyGstError::GetPropertyError { property: "Framerates".to_string(), error: "No framerate numerator? Shouldn't happen, please report!".to_string() });
                                    }
                                }
                                resolution_map
                                    .insert(Resolution::new(width as u32, height as u32), fps_vec);
                            }
                        }
                        unsupported => {
                            return Err(BevyGstError::NotImplementedError(format!(
                                "Not supported frame format {unsupported:?}"
                            )))
                        }
                    }
                }
            }
            None => {
                return Err(BevyGstError::GetPropertyError {
                    property: "Device Caps".to_string(),
                    error: "No device caps!".to_string(),
                })
            }
        }

        Ok(resolution_map)
    }
}

/// search device by index
fn search_device(index: usize) -> Result<(CameraInfo, Option<Caps>), BevyGstError> {
    let device_monitor = DeviceMonitor::new();

    let video_caps = match Caps::from_str("video/x-raw") {
        Ok(cap) => cap,
        Err(why) => {
            return Err(BevyGstError::GeneralError(format!(
                "Failed to generate caps: {}",
                why
            )))
        }
    };

    let _video_filter_id = match device_monitor.add_filter(Some("Video/Source"), Some(&video_caps))
    {
        Some(id) => id,
        None => match device_monitor.add_filter(Some("Source/Video"), Some(&video_caps)) {
            Some(id) => id,
            None => {
                return Err(BevyGstError::StructureError {
                    structure: "Video Filter ID Source/Video".to_string(),
                    error: "Null".to_string(),
                })
            }
        },
    };

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
            "",
            index,
        ),
        caps,
    ))
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::let_and_return)]
fn generate_pipeline(fmt: CameraFormat, index: usize) -> Result<PipelineGenRet, BevyGstError> {
    let appsink_pipeline = webcam_pipeline(format!("{}", index).as_str(), fmt);

    let pipeline = match gstreamer::parse::launch(&appsink_pipeline) {
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
                            (
                                "{}",
                                format!("Failed to get video info from caps: {}", why).as_str()
                            )
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
                            (
                                "{}",
                                format!("Failed to map buffer to readablemap: {}", why).as_str()
                            )
                        );

                        return Err(FlowError::Error);
                    }
                };

                let channels = if video_info.has_alpha() { 4 } else { 3 };

                let image_buffer = match video_info.format() {
                    VideoFormat::Yuy2 => {
                        let mut decoded_buffer = match yuyv422_to_rgb(&buffer_map, true) {
                            Ok(buf) => buf,
                            Err(why) => {
                                element_error!(
                                    appsink,
                                    ResourceError::Failed,
                                    (
                                        "{}",
                                        format!("Failed to make yuy2 into rgb888: {}", why)
                                            .as_str()
                                    )
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
                                    (
                                        "{}",
                                        format!("Failed to make yuy2 into rgb888: {}", why)
                                            .as_str()
                                    )
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

                if let Ok(mut img) = img_lck_clone.lock() {
                    *img = image_buffer;
                }

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
        FrameFormat::GRAY => {
            format!("autovideosrc location=/dev/video{} ! video/x-raw,format=GRAY8,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
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
        FrameFormat::GRAY => {
            format!("v4l2src device=/dev/video{} ! video/x-raw,format=GRAY8,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
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
            format!("mfvideosrc   device_index={} ! image/jpeg, width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        FrameFormat::YUYV => {
            format!("mfvideosrc   device_index={} ! video/x-raw,format=YUY2,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        FrameFormat::GRAY => {
            format!("mfvideosrc   device_index={} ! video/x-raw,format=GRAY8,width={},height={},framerate={}/1 ! appsink name=appsink async=false sync=false", device, camera_format.width(), camera_format.height(), camera_format.frame_rate())
        }
        _ => "unsupproted! if you see this, switch to something else!".to_string(),
    }
}
