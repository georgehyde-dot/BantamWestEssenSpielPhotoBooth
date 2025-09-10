// GPhoto2-based camera implementation for Canon EOS Rebel T7
// Uses gphoto2 CLI for preview streaming and capture operations

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

// Use the camera config from the config module
use crate::config::CameraConfig;

#[cfg(target_os = "linux")]
pub struct GPhotoCamera {
    config: CameraConfig,
    preview_process: Arc<Mutex<Option<Child>>>,
    is_streaming: Arc<Mutex<bool>>,
    preview_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

#[cfg(target_os = "linux")]
impl GPhotoCamera {
    /// Create a new GPhotoCamera instance
    pub fn new(config: CameraConfig) -> Result<Self, String> {
        Ok(GPhotoCamera {
            config,
            preview_process: Arc::new(Mutex::new(None)),
            is_streaming: Arc::new(Mutex::new(false)),
            preview_task: Arc::new(Mutex::new(None)),
        })
    }

    /// Kill any existing gphoto2 processes
    fn kill_gphoto_processes() {
        let _ = Command::new("pkill").args(&["-f", "gphoto2"]).output();
    }

    /// Initialize and connect to the camera
    pub async fn initialize(&self) -> Result<(), String> {
        info!("Initializing Canon EOS camera via USB...");

        // Kill any existing gphoto2 processes
        Self::kill_gphoto_processes();
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Check if camera is connected using gphoto2 --auto-detect
        let output = tokio::process::Command::new("gphoto2")
            .arg("--auto-detect")
            .output()
            .await
            .map_err(|e| format!("Failed to run gphoto2 --auto-detect: {}", e))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        info!("Camera detection output: {}", output_str);

        // Check if a camera was detected (look for lines with USB)
        if !output_str.contains("usb:") {
            return Err(
                "No camera detected. Please ensure camera is connected and turned on.".to_string(),
            );
        }

        info!("Camera initialized successfully");
        Ok(())
    }

    /// Start the camera preview stream using gphoto2 CLI and v4l2loopback
    pub async fn start_preview_stream(
        &self,
        frame_sink: mpsc::Sender<Vec<u8>>,
        last_frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Result<(), String> {
        // Check if already streaming
        {
            let is_streaming = self.is_streaming.lock().unwrap();
            if *is_streaming {
                warn!("Preview stream already running");
                return Ok(());
            }
        }

        info!("Starting camera preview stream...");

        // Stop any existing preview
        self.stop_preview_internal().await;

        // Kill any stray gphoto2 processes
        Self::kill_gphoto_processes();
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Start gphoto2 preview stream to v4l2loopback device
        let v4l2_device = self.config.device.clone(); // e.g., "/dev/video2"

        info!("Starting gphoto2 preview stream to {}", v4l2_device);

        // Use bash to run the piped command
        let preview_cmd = Command::new("bash")
            .args(&[
                "-c",
                &format!(
                    "gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -threads 0 -f v4l2 {}",
                    v4l2_device
                )
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start preview command: {}", e))?;

        // Store the process handle
        *self.preview_process.lock().unwrap() = Some(preview_cmd);

        // Set streaming flag
        *self.is_streaming.lock().unwrap() = true;

        // Give it a moment to start
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Start a task to read frames from the v4l2 device
        let is_streaming = self.is_streaming.clone();
        let device_path = v4l2_device.clone();

        let preview_handle = tokio::spawn(async move {
            // Wait a bit for the stream to stabilize
            tokio::time::sleep(Duration::from_secs(2)).await;

            info!("Starting frame capture from {}", device_path);

            // Now we need to read frames from the v4l2 device
            // We'll use ffmpeg to grab frames periodically
            let mut frame_count = 0;
            let mut last_error_time = std::time::Instant::now();

            while *is_streaming.lock().unwrap() {
                // Capture a single frame from the v4l2 device using ffmpeg
                let output = tokio::process::Command::new("ffmpeg")
                    .args(&[
                        "-f",
                        "v4l2",
                        "-i",
                        &device_path,
                        "-frames:v",
                        "1",
                        "-f",
                        "mjpeg",
                        "-",
                    ])
                    .output()
                    .await;

                match output {
                    Ok(output) if !output.stdout.is_empty() => {
                        frame_count += 1;
                        if frame_count % 30 == 0 {
                            debug!("Captured {} preview frames", frame_count);
                        }

                        let frame_data = output.stdout;

                        // Store in last frame buffer
                        {
                            let mut last_frame = last_frame_buffer.lock().unwrap();
                            *last_frame = Some(frame_data.clone());
                        }

                        // Send to sink (non-blocking)
                        let _ = frame_sink.send(frame_data).await;
                    }
                    Ok(_) => {
                        if last_error_time.elapsed() > Duration::from_secs(5) {
                            warn!("Empty frame captured from {}", device_path);
                            last_error_time = std::time::Instant::now();
                        }
                    }
                    Err(e) => {
                        if last_error_time.elapsed() > Duration::from_secs(5) {
                            warn!("Failed to capture frame: {}", e);
                            last_error_time = std::time::Instant::now();
                        }
                    }
                }

                // Control frame rate (approximately 30fps)
                tokio::time::sleep(Duration::from_millis(33)).await;
            }

            info!("Preview loop ended after {} frames", frame_count);
        });

        // Store the task handle
        *self.preview_task.lock().unwrap() = Some(preview_handle);

        info!("Preview stream started successfully");
        Ok(())
    }

    /// Internal method to stop preview without async
    async fn stop_preview_internal(&self) {
        // Kill the preview process if it exists
        if let Some(mut process) = self.preview_process.lock().unwrap().take() {
            info!("Killing preview process");
            let _ = process.kill();
            let _ = process.wait();
        }

        // Kill any gphoto2 processes
        Self::kill_gphoto_processes();
    }

    /// Stop the camera preview stream
    pub async fn stop_preview(&self) -> Result<(), String> {
        info!("Stopping camera preview...");

        // Set streaming flag to false
        *self.is_streaming.lock().unwrap() = false;

        // Stop the preview process
        self.stop_preview_internal().await;

        // Wait for preview task to complete
        let handle = self.preview_task.lock().unwrap().take();
        if let Some(handle) = handle {
            // Give it a moment to finish gracefully
            tokio::time::timeout(Duration::from_secs(2), handle)
                .await
                .ok();
        }

        info!("Preview stopped");
        Ok(())
    }

    /// Capture a high-resolution photo using gphoto2 CLI
    pub async fn capture_photo(&self, output_path: &str) -> Result<Vec<u8>, String> {
        info!("Capturing photo to: {}", output_path);

        // Stop preview if running
        if *self.is_streaming.lock().unwrap() {
            info!("Stopping preview before capture");
            self.stop_preview().await?;
            // Wait a bit for camera to be ready
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Kill any lingering gphoto2 processes
        Self::kill_gphoto_processes();
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Capture photo using gphoto2
        let output = tokio::process::Command::new("gphoto2")
            .args(&[
                "--capture-image-and-download",
                "--filename",
                output_path,
                "--force-overwrite",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run capture command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to capture photo: {}", stderr));
        }

        info!("Photo captured successfully: {}", output_path);

        // Read the captured file
        let jpeg_data = tokio::fs::read(output_path)
            .await
            .map_err(|e| format!("Failed to read captured photo: {}", e))?;

        Ok(jpeg_data)
    }

    /// Capture with countdown
    pub async fn countdown_capture(
        &self,
        output_path: &str,
        countdown_seconds: u32,
    ) -> Result<Vec<u8>, String> {
        info!("Starting countdown capture: {} seconds", countdown_seconds);

        // Make sure preview is stopped first
        if self.is_streaming() {
            self.stop_preview().await?;
        }

        // Countdown
        for i in (1..=countdown_seconds).rev() {
            info!("Countdown: {}", i);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Capture the photo
        self.capture_photo(output_path).await
    }

    /// Check if camera is connected
    pub async fn check_connection(&self) -> bool {
        // Try to detect camera using gphoto2
        let output = tokio::process::Command::new("gphoto2")
            .arg("--auto-detect")
            .output()
            .await;

        match output {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.contains("usb:")
            }
            Err(_) => false,
        }
    }

    /// Check if preview is streaming
    pub fn is_streaming(&self) -> bool {
        *self.is_streaming.lock().unwrap()
    }

    /// Cleanup camera resources
    pub async fn cleanup(&self) -> Result<(), String> {
        info!("Cleaning up camera resources...");

        // Stop preview if running
        if self.is_streaming() {
            self.stop_preview().await?;
        }

        // Kill any remaining gphoto2 processes
        Self::kill_gphoto_processes();

        info!("Camera cleanup completed");
        Ok(())
    }

    /// Get a single frame from the current preview buffer
    pub fn get_preview_frame(
        &self,
        last_frame_buffer: Arc<Mutex<Option<Vec<u8>>>>,
    ) -> Option<Vec<u8>> {
        if !self.is_streaming() {
            return None;
        }
        last_frame_buffer.lock().unwrap().clone()
    }
}

// Non-Linux stub implementation
#[cfg(not(target_os = "linux"))]
pub struct GPhotoCamera {
    config: CameraConfig,
}

#[cfg(not(target_os = "linux"))]
impl GPhotoCamera {
    pub fn new(config: CameraConfig) -> Result<Self, String> {
        Ok(GPhotoCamera { config })
    }

    pub async fn initialize(&self) -> Result<(), String> {
        Err("GPhoto2 camera functionality not supported on this platform".to_string())
    }

    pub async fn start_preview_stream(
        &self,
        _frame_sink: tokio::sync::mpsc::Sender<Vec<u8>>,
        _last_frame_buffer: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    ) -> Result<(), String> {
        Err("Camera functionality not supported on this platform".to_string())
    }

    pub async fn stop_preview(&self) -> Result<(), String> {
        Err("Camera functionality not supported on this platform".to_string())
    }

    pub async fn capture_photo(&self, _output_path: &str) -> Result<Vec<u8>, String> {
        Err("Camera functionality not supported on this platform".to_string())
    }

    pub async fn countdown_capture(
        &self,
        _output_path: &str,
        _countdown_seconds: u32,
    ) -> Result<Vec<u8>, String> {
        Err("Camera functionality not supported on this platform".to_string())
    }

    pub async fn check_connection(&self) -> bool {
        false
    }

    pub fn is_streaming(&self) -> bool {
        false
    }

    pub async fn cleanup(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn get_preview_frame(
        &self,
        _last_frame_buffer: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    ) -> Option<Vec<u8>> {
        None
    }
}
