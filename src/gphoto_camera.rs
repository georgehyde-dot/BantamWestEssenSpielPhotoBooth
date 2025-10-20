// GPhoto2-based camera implementation for Canon EOS Rebel T7
// Uses gphoto2 CLI for preview streaming and capture operations

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// Use the camera config from the config module
use crate::config::CameraConfig;

pub struct GPhotoCamera {
    config: CameraConfig,
    preview_process: Arc<Mutex<Option<Child>>>,
    is_streaming: Arc<Mutex<bool>>,
}

impl GPhotoCamera {
    /// Create a new GPhotoCamera instance
    pub fn new(config: CameraConfig) -> Result<Self, String> {
        Ok(GPhotoCamera {
            config,
            preview_process: Arc::new(Mutex::new(None)),
            is_streaming: Arc::new(Mutex::new(false)),
        })
    }

    /// Kill any existing gphoto2 and related processes
    fn kill_gphoto_processes() {
        debug!("Killing gphoto2 processes with SIGTERM...");
        // Kill gphoto2 processes
        let _ = Command::new("pkill").args(&["-f", "gphoto2"]).output();
        // Kill any ffmpeg processes that might be connected to v4l2 devices
        let _ = Command::new("pkill").args(&["-f", "ffmpeg.*v4l2"]).output();
        // Give processes time to die
        debug!("Waiting 200ms for graceful termination...");
        std::thread::sleep(Duration::from_millis(200));
        // Force kill if still running
        debug!("Force killing any remaining processes with SIGKILL...");
        let _ = Command::new("pkill")
            .args(&["-9", "-f", "gphoto2"])
            .output();
        let _ = Command::new("pkill")
            .args(&["-9", "-f", "ffmpeg.*v4l2"])
            .output();

        // Verify processes are dead
        let check = Command::new("pgrep").args(&["-f", "gphoto2"]).output();
        if let Ok(output) = check {
            if !output.stdout.is_empty() {
                warn!("Some gphoto2 processes still running after kill attempt!");
            } else {
                debug!("All gphoto2 processes successfully terminated");
            }
        }
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
    pub async fn start_preview_stream(&self) -> Result<(), String> {
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
        let v4l2_device = self.config.v4l2_loopback_device.clone(); // e.g., "/dev/video0"

        info!("Starting gphoto2 preview stream to {}", v4l2_device);

        // Use bash to run the piped command
        // Set process group to ensure all children are killed together
        let mut cmd = Command::new("bash");
        cmd.args(&[
            "-c",
            &format!(
                "gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -threads 0 -f v4l2 {}",
                v4l2_device
            )
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

        // Create a new process group so we can kill all children
        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }

        let preview_cmd = cmd
            .spawn()
            .map_err(|e| format!("Failed to start preview command: {}", e))?;

        // Store the process handle
        *self.preview_process.lock().unwrap() = Some(preview_cmd);

        // Set streaming flag
        *self.is_streaming.lock().unwrap() = true;

        // Give the stream a moment to stabilize
        tokio::time::sleep(Duration::from_secs(1)).await;

        info!("Preview stream started successfully");
        Ok(())
    }

    /// Internal method to stop preview without async
    async fn stop_preview_internal(&self) {
        let stop_start = std::time::Instant::now();

        // Kill the preview process if it exists
        if let Some(mut process) = self.preview_process.lock().unwrap().take() {
            let pid = process.id();
            info!("Killing preview process PID {} and its children", pid);

            // Try to get the process ID
            // Kill the entire process group (negative PID kills the group)
            debug!("Sending SIGTERM to process group -{}", pid);
            unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
            }
            // Give it a moment to terminate gracefully
            debug!("Waiting 100ms for graceful termination...");
            std::thread::sleep(Duration::from_millis(100));
            // Force kill if still running
            debug!("Sending SIGKILL to process group -{}", pid);
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }

            // Also try the standard kill
            let _ = process.kill();
            let wait_result = process.wait();
            debug!("Process wait result: {:?}", wait_result);

            info!("Preview process killed in {:?}", stop_start.elapsed());
        } else {
            debug!("No preview process to kill");
        }

        // Kill any remaining gphoto2/ffmpeg processes
        debug!("Cleaning up any remaining processes...");
        Self::kill_gphoto_processes();
        info!("Preview stop completed in {:?}", stop_start.elapsed());
    }

    /// Stop the camera preview stream
    pub async fn stop_preview(&self) -> Result<(), String> {
        info!("Stopping camera preview...");

        // Set streaming flag to false
        *self.is_streaming.lock().unwrap() = false;

        // Stop the preview process
        self.stop_preview_internal().await;

        info!("Preview stopped");
        Ok(())
    }

    /// Capture a high-resolution photo using gphoto2 CLI
    pub async fn capture_photo(&self, output_path: &str) -> Result<Vec<u8>, String> {
        let capture_start = std::time::Instant::now();
        info!("=== CAPTURE PHOTO START ===");
        info!("Output path: {}", output_path);
        info!("Capture started at: {:?}", capture_start);

        // Stop preview if running
        let is_streaming = *self.is_streaming.lock().unwrap();
        info!("Preview streaming status: {}", is_streaming);

        if is_streaming {
            info!("Stopping preview before capture...");
            let stop_start = std::time::Instant::now();
            self.stop_preview().await?;
            let stop_duration = stop_start.elapsed();
            info!("Preview stopped in: {:?}", stop_duration);

            // Wait a bit for camera to be ready
            info!("Waiting 500ms for camera state transition...");
            tokio::time::sleep(Duration::from_millis(500)).await;
            info!(
                "Wait complete, elapsed since capture start: {:?}",
                capture_start.elapsed()
            );
        } else {
            info!("Preview already stopped, proceeding directly to capture");
        }

        // Kill any lingering gphoto2 processes
        info!("Killing any lingering gphoto2 processes...");
        let kill_start = std::time::Instant::now();
        Self::kill_gphoto_processes();
        info!("Process kill complete in: {:?}", kill_start.elapsed());

        info!("Waiting 200ms for process cleanup...");
        tokio::time::sleep(Duration::from_millis(200)).await;
        info!(
            "Total elapsed since capture start: {:?}",
            capture_start.elapsed()
        );

        // Capture photo using gphoto2
        info!("Executing gphoto2 capture command...");
        info!(
            "Command: gphoto2 --capture-image-and-download --filename {} --force-overwrite",
            output_path
        );
        let capture_cmd_start = std::time::Instant::now();

        let output = tokio::process::Command::new("gphoto2")
            .args(&[
                "--capture-image-and-download",
                "--filename",
                output_path,
                "--force-overwrite",
            ])
            .output()
            .await
            .map_err(|e| {
                let elapsed = capture_cmd_start.elapsed();
                error!("Failed to run capture command after {:?}: {}", elapsed, e);
                format!("Failed to run capture command: {}", e)
            })?;

        let capture_cmd_duration = capture_cmd_start.elapsed();
        info!("Capture command completed in: {:?}", capture_cmd_duration);
        info!(
            "Total elapsed since capture start: {:?}",
            capture_start.elapsed()
        );

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!("Capture failed with exit code: {:?}", output.status.code());
            error!("STDERR: {}", stderr);
            error!("STDOUT: {}", stdout);
            error!("Total time before failure: {:?}", capture_start.elapsed());

            // Check for specific error patterns
            if stderr.contains("Device Busy") || stderr.contains("PTP Device Busy") {
                error!("Camera is busy - may need longer delay after stopping preview");
            }
            if stderr.contains("I/O in progress") {
                error!("I/O operation in progress - camera still processing previous command");
            }

            return Err(format!("Failed to capture photo: {}", stderr));
        }

        info!("=== CAPTURE PHOTO SUCCESS ===");
        info!("Photo captured successfully: {}", output_path);
        info!("Total capture time: {:?}", capture_start.elapsed());

        // Read the captured file
        let jpeg_data = tokio::fs::read(output_path)
            .await
            .map_err(|e| format!("Failed to read captured photo: {}", e))?;

        Ok(jpeg_data)
    }
}

impl Drop for GPhotoCamera {
    fn drop(&mut self) {
        info!("GPhotoCamera dropping, cleaning up processes...");

        // Set streaming flag to false
        *self.is_streaming.lock().unwrap() = false;

        // Kill the preview process if it exists
        if let Some(mut process) = self.preview_process.lock().unwrap().take() {
            info!("Cleaning up preview process on drop");

            // Try to get the process ID
            let pid = process.id();
            // Kill the entire process group
            unsafe {
                libc::kill(-(pid as i32), libc::SIGTERM);
                std::thread::sleep(Duration::from_millis(100));
                libc::kill(-(pid as i32), libc::SIGKILL);
            }

            let _ = process.kill();
            let _ = process.wait();
        }

        // Kill any remaining gphoto2/ffmpeg processes
        Self::kill_gphoto_processes();

        info!("GPhotoCamera cleanup complete");
    }
}
