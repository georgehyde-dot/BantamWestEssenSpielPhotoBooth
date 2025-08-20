// Camera functionality module

#[cfg(target_os = "linux")]
use bytes::Bytes;
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use tokio::sync::mpsc;
#[cfg(target_os = "linux")]
use v4l::buffer::Type;
#[cfg(target_os = "linux")]
use v4l::device::Device;
#[cfg(target_os = "linux")]
use v4l::io::traits::CaptureStream;
#[cfg(target_os = "linux")]
use v4l::io::userptr;
#[cfg(target_os = "linux")]
use v4l::prelude::*;
#[cfg(target_os = "linux")]
use v4l::video::Capture;
#[cfg(target_os = "linux")]
use v4l::{Format, FourCC};

// Use the camera config from the main configuration module
#[cfg(target_os = "linux")]
use crate::config::CameraConfig;

#[cfg(target_os = "linux")]
impl CameraConfig {
    pub fn from_env() -> Self {
        // This method is kept for backward compatibility
        // but delegates to the main config
        crate::config::Config::from_env()
            .map(|c| c.camera)
            .unwrap_or_else(|_| CameraConfig {
                device: "/dev/video0".to_string(),
                width: 1920,
                height: 1080,
                format: "MJPG".to_string(),
            })
    }
}

#[cfg(target_os = "linux")]
pub struct Camera {
    config: CameraConfig,
}

#[cfg(target_os = "linux")]
impl Camera {
    pub fn new(config: CameraConfig) -> Self {
        Camera { config }
    }

    pub async fn start_preview_stream(
        &self,
        frame_sink: mpsc::Sender<Vec<u8>>,
        last_frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<(), String> {
        let config = self.config.clone();
        tokio::task::spawn_blocking(move || {
            preview_loop(
                config.device,
                config.width,
                config.height,
                frame_sink,
                last_frame_buffer,
            )
        })
        .await
        .map_err(|e| format!("Preview task failed: {}", e))?
    }

    pub fn capture_frame(&self, last_frame_buffer: Arc<Mutex<Option<Vec<u8>>>>) -> Option<Vec<u8>> {
        last_frame_buffer.lock().unwrap().clone()
    }
}

// Internal implementation details
#[cfg(target_os = "linux")]
fn configure_device(dev: &mut Device, width: u32, height: u32) -> Result<Format, String> {
    // Capture trait in scope provides format() and set_format()
    let mut fmt = dev.format().map_err(|e| format!("format(): {e}"))?;
    fmt.width = width;
    fmt.height = height;

    // Try MJPEG first, fall back to YUYV
    fmt.fourcc = FourCC::new(b"MJPG");
    let fmt = dev
        .set_format(&fmt)
        .map_err(|e| format!("set_format(): {e}"))?;

    if fmt.fourcc == FourCC::new(b"MJPG") {
        return Ok(fmt);
    }

    Err(format!(
        "Device does not support MJPEG, got {}. Only MJPEG is supported.",
        fmt.fourcc
    ))
}

#[cfg(target_os = "linux")]
pub fn preview_loop(
    path: String,
    width: u32,
    height: u32,
    mut tx: mpsc::Sender<Vec<u8>>,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<(), String> {
    let mut dev = Device::with_path(path).map_err(|e| format!("open device: {e}"))?;
    let fmt = configure_device(&mut dev, width, height)?;

    let is_mjpeg = fmt.fourcc == FourCC::new(b"MJPG");
    let mut frame_count = 0;

    // Try userptr streaming first (better for HDMI capture devices)
    match try_userptr_streaming(
        &mut dev,
        &fmt,
        is_mjpeg,
        &mut tx,
        &mut frame_count,
        &last_frame,
    ) {
        Ok(()) => return Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(target_os = "linux")]
fn try_userptr_streaming(
    dev: &mut Device,
    fmt: &Format,
    is_mjpeg: bool,
    tx: &mut mpsc::Sender<Vec<u8>>,
    frame_count: &mut usize,
    last_frame: &Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<(), String> {
    let mut stream = userptr::Stream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create UserptrStream: {e}"))?;

    loop {
        match stream.next() {
            Ok((buffer, _meta)) => {
                *frame_count += 1;

                let jpeg_data = if is_mjpeg {
                    buffer.to_vec()
                } else {
                    continue;
                };

                {
                    let mut lf = last_frame.lock().unwrap();
                    *lf = Some(jpeg_data.clone());
                }
                if tx.blocking_send(jpeg_data).is_err() {
                    break;
                }
            }
            Err(e) => {
                return Err(format!("Userptr stream error: {e}"));
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn video_settings() -> (String, u32, u32) {
    let dev = std::env::var("VIDEO_DEVICE").unwrap_or_else(|_| "/dev/video0".to_string());
    let width = std::env::var("VIDEO_WIDTH")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1920);
    let height = std::env::var("VIDEO_HEIGHT")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1080);
    (dev, width, height)
}

// Non-Linux stubs
#[cfg(not(target_os = "linux"))]
#[derive(Clone)]
#[allow(dead_code)]
pub struct CameraConfig {
    pub device: String,
    pub width: u32,
    pub height: u32,
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
impl CameraConfig {
    pub fn from_env() -> Self {
        CameraConfig {
            device: "/dev/video0".to_string(),
            width: 1920,
            height: 1080,
        }
    }
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub struct Camera {
    config: CameraConfig,
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
impl Camera {
    pub fn new(config: CameraConfig) -> Self {
        Camera { config }
    }

    pub async fn start_preview_stream(
        &self,
        _frame_sink: tokio::sync::mpsc::Sender<Vec<u8>>,
        _last_frame_buffer: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    ) -> Result<(), String> {
        Err("Camera functionality not supported on this platform".to_string())
    }

    pub fn capture_frame(
        &self,
        _last_frame_buffer: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    ) -> Option<Vec<u8>> {
        None
    }
}
