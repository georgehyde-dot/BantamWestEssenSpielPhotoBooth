// Linux (Raspberry Pi) implementation: V4L2 (v4l crate) MJPEG preview and capture over HTTP.
// Non-Linux: build a stub that informs the user this is Linux-only.

#[cfg(target_os = "linux")]
use actix_files as fs;
#[cfg(target_os = "linux")]
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
#[cfg(target_os = "linux")]
use async_stream;
#[cfg(target_os = "linux")]
use chrono;
#[cfg(target_os = "linux")]
use serde_json;
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;
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
use v4l::prelude::*;
#[cfg(target_os = "linux")]
use v4l::Format;
#[cfg(target_os = "linux")]
use v4l::FourCC;
// bring Capture trait into scope so Device.format(), Device.set_format() are available
#[cfg(target_os = "linux")]
use v4l::io::userptr;
#[cfg(target_os = "linux")]
use v4l::video::Capture;

#[cfg(target_os = "linux")]
use bytes::Bytes; // for streaming
#[cfg(target_os = "linux")]
use image::{ImageBuffer, Rgb};
#[cfg(target_os = "linux")]
use std::io::Cursor;

// Printer functionality imports
#[cfg(target_os = "linux")]
use async_trait::async_trait;
#[cfg(all(target_os = "linux", feature = "printer-cups"))]
use printers::{
    common::base::job::PrinterJobOptions, common::base::printer::Printer as PrintersCratePrinter,
    get_printers,
};
#[cfg(target_os = "linux")]
use serde::Serialize;
#[cfg(target_os = "linux")]
use std::error::Error;
#[cfg(target_os = "linux")]
use std::fmt;

// Printer functionality
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub enum PaperSize {
    Letter,
    A4,
    Photo4x6,
    Photo5x7,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub enum PrintQuality {
    Draft,
    Normal,
    High,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct PrintJob {
    pub file_path: String,
    pub copies: u32,
    pub paper_size: PaperSize,
    pub quality: PrintQuality,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Serialize)]
pub struct PrinterStatus {
    pub is_online: bool,
    pub paper_level: Option<u32>,
    pub toner_level: Option<u32>,
    pub error_message: Option<String>,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub enum PrinterError {
    NotFound(String),
    NotReady(String),
    PrintFailed(String),
    IoError(String),
}

#[cfg(target_os = "linux")]
impl fmt::Display for PrinterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrinterError::NotFound(msg) => write!(f, "Printer not found: {}", msg),
            PrinterError::NotReady(msg) => write!(f, "Printer not ready: {}", msg),
            PrinterError::PrintFailed(msg) => write!(f, "Print failed: {}", msg),
            PrinterError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

#[cfg(target_os = "linux")]
impl Error for PrinterError {}

#[cfg(target_os = "linux")]
#[async_trait]
pub trait Printer {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError>;
    async fn is_ready(&self) -> bool;
    async fn get_status(&self) -> Result<PrinterStatus, PrinterError>;
    fn type_name(&self) -> &'static str;
}

#[cfg(all(target_os = "linux", feature = "printer-cups"))]
pub struct EpsonPrinter {
    printer_name: String,
    cups_printer: Option<PrintersCratePrinter>,
}

#[cfg(all(target_os = "linux", feature = "printer-cups"))]
impl EpsonPrinter {
    pub async fn new(printer_name: &str) -> Result<Self, PrinterError> {
        eprintln!("Initializing Epson XP-8700 printer: {}", printer_name);

        // Get all available printers
        let printers = get_printers();
        eprintln!("Found {} printers", printers.len());

        // Log all available printers for debugging
        for printer in &printers {
            eprintln!("Available printer: {}", printer.name);
        }

        // Find the XP-8700 printer (try exact match first, then partial match)
        let cups_printer = printers
            .iter()
            .find(|p| p.name.to_lowercase() == printer_name.to_lowercase())
            .or_else(|| {
                printers
                    .iter()
                    .find(|p| p.name.to_lowercase().contains("xp-8700"))
            })
            .cloned();

        match cups_printer {
            Some(printer) => {
                eprintln!("Found XP-8700 printer: {}", printer.name);
                Ok(EpsonPrinter {
                    printer_name: printer.name.clone(),
                    cups_printer: Some(printer),
                })
            }
            None => {
                eprintln!(
                    "XP-8700 printer '{}' not found. Available printers: {:?}",
                    printer_name,
                    printers.iter().map(|p| &p.name).collect::<Vec<_>>()
                );
                Err(PrinterError::NotFound(format!(
                    "XP-8700 printer '{}' not found in CUPS",
                    printer_name
                )))
            }
        }
    }
}

#[cfg(all(target_os = "linux", feature = "printer-cups"))]
#[async_trait]
impl Printer for EpsonPrinter {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError> {
        eprintln!(
            "Printing photo: {} with {} copies",
            job.file_path, job.copies
        );

        let printer = self
            .cups_printer
            .as_ref()
            .ok_or_else(|| PrinterError::NotReady("Printer not initialized".to_string()))?;

        // Check if file exists and get file info
        let file_path = std::path::Path::new(&job.file_path);
        if !file_path.exists() {
            return Err(PrinterError::IoError(format!(
                "File not found: {}",
                job.file_path
            )));
        }

        // Log file details for debugging
        match file_path.metadata() {
            Ok(metadata) => {
                eprintln!("File size: {} bytes", metadata.len());
                eprintln!("File modified: {:?}", metadata.modified());
                eprintln!("File permissions: {:o}", metadata.permissions().mode());

                // Check if file is readable
                match std::fs::File::open(&job.file_path) {
                    Ok(_) => eprintln!("✓ File is readable by current user"),
                    Err(e) => {
                        eprintln!("✗ File read test failed: {}", e);
                        return Err(PrinterError::IoError(format!(
                            "Cannot read file {}: {}",
                            job.file_path, e
                        )));
                    }
                }
            }
            Err(e) => {
                eprintln!("Could not read file metadata: {}", e);
                return Err(PrinterError::IoError(format!(
                    "Cannot access file metadata for {}: {}",
                    job.file_path, e
                )));
            }
        }

        // Check MIME type detection
        if let Ok(output) = std::process::Command::new("file")
            .arg("-b")
            .arg("--mime-type")
            .arg(&job.file_path)
            .output()
        {
            let mime_type = String::from_utf8_lossy(&output.stdout).trim().to_string();
            eprintln!("Detected MIME type: {}", mime_type);

            if !mime_type.starts_with("image/") {
                eprintln!(
                    "⚠ Warning: File may not be a valid image (MIME: {})",
                    mime_type
                );
            }
        } else {
            eprintln!("⚠ Warning: Could not detect file MIME type");
        }

        // Validate image file structure to prevent corrupted file printing
        eprintln!("Validating image file structure...");
        if let Ok(file_bytes) = std::fs::read(&job.file_path) {
            // Try to decode the image to validate it's a proper image file
            match image::load_from_memory(&file_bytes) {
                Ok(_) => {
                    eprintln!("✓ Image file structure appears valid");
                }
                Err(e) => {
                    return Err(PrinterError::IoError(format!(
                        "Image file validation failed: {}",
                        e
                    )));
                }
            }
        } else {
            return Err(PrinterError::IoError(format!(
                "Cannot read file for validation: {}",
                job.file_path
            )));
        }

        // Use original file directly - ensure proper permissions for CUPS access
        eprintln!("Using original file directly for CUPS printing");

        // Set proper permissions on original file for CUPS access
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(mut perms) =
                std::fs::metadata(&job.file_path).and_then(|m| Ok(m.permissions()))
            {
                perms.set_mode(0o644); // rw-r--r--
                if let Err(e) = std::fs::set_permissions(&job.file_path, perms) {
                    eprintln!("Warning: Could not set file permissions: {}", e);
                }
            }
        }

        let print_file_path = job.file_path.clone();

        // Use minimal printer options - let CUPS handle defaults
        let mut raw_properties = Vec::new();

        // Only set essential options for 4x6 photo printing
        eprintln!("Using minimal print settings to avoid filter conflicts");

        // Set paper size for 4x6 photo (use borderless format for EPSON_XP_8700_Series_USB)
        let paper_size_str = match job.paper_size {
            PaperSize::Photo4x6 => "4x6.Borderless",
            PaperSize::Photo5x7 => "5x7.Borderless",
            _ => "Letter", // fallback
        };
        raw_properties.push(("PageSize", paper_size_str.to_string()));

        // Set photo tray and media type for photo sizes (matching working printer defaults)
        if matches!(job.paper_size, PaperSize::Photo4x6 | PaperSize::Photo5x7) {
            raw_properties.push(("InputSlot", "Photo".to_string()));
            raw_properties.push(("MediaType", "PhotographicSemiGloss".to_string()));
        }

        // Set copies
        raw_properties.push(("copies", job.copies.to_string()));

        let job_name = format!("PhotoBooth-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));

        // Add job title to the raw properties for better printer recognition
        raw_properties.push(("job-name", job_name.clone()));

        // Convert to the format expected by the printers crate
        let raw_props: Vec<(&str, &str)> = raw_properties
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let options = PrinterJobOptions {
            name: Some(&job_name),
            raw_properties: &raw_props,
        };

        eprintln!(
            "Sending print job to printer '{}' with options: {:?}",
            self.printer_name, raw_props
        );
        eprintln!("Print job name: {}", job_name);
        eprintln!(
            "Original file path: {}",
            std::path::Path::new(&job.file_path)
                .canonicalize()
                .unwrap_or_else(|_| std::path::PathBuf::from(&job.file_path))
                .display()
        );
        eprintln!("Print file path: {}", print_file_path);

        // Final file check before printing
        eprintln!("Final pre-print checks for file: {}", print_file_path);
        if let Ok(metadata) = std::fs::metadata(&print_file_path) {
            eprintln!("Print file size: {} bytes", metadata.len());
            eprintln!(
                "Print file permissions: {:o}",
                metadata.permissions().mode()
            );
        }

        // Test file access as a different way to catch permission issues
        match std::fs::File::open(&print_file_path) {
            Ok(_) => eprintln!("✓ Print file is accessible"),
            Err(e) => {
                eprintln!("✗ Cannot access print file: {}", e);
                // Clean up temp file before returning error
                if print_file_path != job.file_path {
                    let _ = std::fs::remove_file(&print_file_path);
                }
                return Err(PrinterError::IoError(format!(
                    "Print file not accessible: {}",
                    e
                )));
            }
        }

        match printer.print_file(&print_file_path, options) {
            Ok(job_id) => {
                eprintln!("Print job submitted successfully with ID: {}", job_id);
                eprintln!("Using original file directly with CUPS");

                // Check job status immediately after submission
                if let Ok(output) = std::process::Command::new("lpstat")
                    .arg("-o")
                    .arg(&job_id.to_string())
                    .output()
                {
                    let status = String::from_utf8_lossy(&output.stdout);
                    eprintln!("Initial job status: {}", status.trim());
                }

                Ok(job_id.to_string())
            }
            Err(e) => {
                eprintln!("Print job failed with error: {}", e);
                eprintln!("Error type: {:?}", e);

                // Check CUPS error log for more details
                if let Ok(output) = std::process::Command::new("tail")
                    .arg("-5")
                    .arg("/var/log/cups/error_log")
                    .output()
                {
                    let error_log = String::from_utf8_lossy(&output.stdout);
                    if !error_log.trim().is_empty() {
                        eprintln!("Recent CUPS errors:");
                        eprintln!("{}", error_log);
                    }
                }

                Err(PrinterError::PrintFailed(format!(
                    "CUPS print error: {}",
                    e
                )))
            }
        }
    }

    async fn is_ready(&self) -> bool {
        // For now, assume printer is ready if we have a CUPS printer object
        self.cups_printer.is_some()
    }

    async fn get_status(&self) -> Result<PrinterStatus, PrinterError> {
        // Basic status - the printers crate doesn't provide detailed status info
        Ok(PrinterStatus {
            is_online: self.cups_printer.is_some(),
            paper_level: None, // Not available through printers crate
            toner_level: None, // Not available through printers crate
            error_message: None,
        })
    }

    fn type_name(&self) -> &'static str {
        "Epson XP-8700 CUPS Printer"
    }
}

#[cfg(target_os = "linux")]
pub struct MockPrinter;

#[cfg(target_os = "linux")]
#[async_trait]
impl Printer for MockPrinter {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError> {
        eprintln!(
            "Mock printer: Would print {} with {} copies",
            job.file_path, job.copies
        );

        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Generate a mock job ID
        let job_id = format!("mock-job-{}", chrono::Utc::now().timestamp());
        eprintln!("Mock print job created: {}", job_id);

        Ok(job_id)
    }

    async fn is_ready(&self) -> bool {
        true
    }

    async fn get_status(&self) -> Result<PrinterStatus, PrinterError> {
        Ok(PrinterStatus {
            is_online: true,
            paper_level: Some(85),
            toner_level: Some(60),
            error_message: None,
        })
    }

    fn type_name(&self) -> &'static str {
        "Mock Printer"
    }
}

#[cfg(target_os = "linux")]
pub async fn new_printer() -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    #[cfg(feature = "printer-cups")]
    {
        // Try to find working XP-8700 printer (prefer working EPSON_XP_8700_Series_USB)
        let possible_names = [
            "EPSON_XP_8700_Series_USB", // Working printer with correct settings
            "XP-8700",
            "xp-8700",
            "Epson XP-8700",
            "EPSON XP-8700",
        ];

        for name in &possible_names {
            match EpsonPrinter::new(name).await {
                Ok(printer) => {
                    eprintln!(
                        "Successfully initialized XP-8700 printer with name: {}",
                        name
                    );
                    return Ok(std::sync::Arc::new(printer));
                }
                Err(e) => {
                    eprintln!("Failed to initialize XP-8700 with name '{}': {}", name, e);
                }
            }
        }

        eprintln!("XP-8700 printer not found with any expected name, using mock printer");
    }

    // Fall back to mock printer
    Ok(std::sync::Arc::new(MockPrinter))
}

// Non-Linux stub types
#[cfg(not(target_os = "linux"))]
pub struct MockPrinter;

#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
pub enum PrinterError {
    NotSupported,
}

#[cfg(not(target_os = "linux"))]
impl std::fmt::Display for PrinterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Printer functionality not supported on this platform")
    }
}

#[cfg(not(target_os = "linux"))]
impl std::error::Error for PrinterError {}

#[cfg(not(target_os = "linux"))]
pub async fn new_printer() -> Result<std::sync::Arc<MockPrinter>, PrinterError> {
    Err(PrinterError::NotSupported)
}

fn video_settings() -> (String, u32, u32) {
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
        println!("Using MJPEG format: {}x{}", fmt.width, fmt.height);
        return Ok(fmt);
    }

    // If MJPEG failed, try YUYV
    let mut fmt = dev.format().map_err(|e| format!("format(): {e}"))?;
    fmt.width = width;
    fmt.height = height;
    fmt.fourcc = FourCC::new(b"YUYV");
    let fmt = dev
        .set_format(&fmt)
        .map_err(|e| format!("set_format(): {e}"))?;

    if fmt.fourcc == FourCC::new(b"YUYV") {
        println!("Using YUYV format: {}x{}", fmt.width, fmt.height);
        return Ok(fmt);
    }

    Err(format!(
        "Device does not support MJPEG or YUYV, got {}. Only MJPEG and YUYV are supported.",
        fmt.fourcc
    ))
}

#[cfg(target_os = "linux")]
fn yuyv_to_jpeg(yuyv_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    // YUYV format: 2 bytes per pixel (4 bytes for 2 pixels: Y1 U Y2 V)
    let expected_size = (width * height * 2) as usize;

    if yuyv_data.len() < expected_size {
        return Err(format!(
            "YUYV data too small: got {}, expected {} for {}x{}",
            yuyv_data.len(),
            expected_size,
            width,
            height
        ));
    }

    // Create RGB data buffer
    let mut rgb_data = vec![0u8; (width * height * 3) as usize];

    // Convert YUYV to RGB
    for y in 0..height {
        for x in 0..(width / 2) {
            let yuyv_base = ((y * width / 2 + x) * 4) as usize;
            let rgb_base1 = ((y * width + x * 2) * 3) as usize;
            let rgb_base2 = ((y * width + x * 2 + 1) * 3) as usize;

            if yuyv_base + 3 >= yuyv_data.len() {
                continue;
            }

            let y1 = yuyv_data[yuyv_base] as f32;
            let u = yuyv_data[yuyv_base + 1] as f32 - 128.0;
            let y2 = yuyv_data[yuyv_base + 2] as f32;
            let v = yuyv_data[yuyv_base + 3] as f32 - 128.0;

            // YUV to RGB conversion (ITU-R BT.601)
            let convert_yuv_to_rgb = |y: f32| -> (u8, u8, u8) {
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
                (r, g, b)
            };

            let (r1, g1, b1) = convert_yuv_to_rgb(y1);
            let (r2, g2, b2) = convert_yuv_to_rgb(y2);

            if rgb_base2 + 2 < rgb_data.len() {
                rgb_data[rgb_base1] = r1;
                rgb_data[rgb_base1 + 1] = g1;
                rgb_data[rgb_base1 + 2] = b1;

                rgb_data[rgb_base2] = r2;
                rgb_data[rgb_base2 + 1] = g2;
                rgb_data[rgb_base2 + 2] = b2;
            }
        }
    }

    // Create image and encode as JPEG
    let img = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width, height, rgb_data)
        .ok_or("Failed to create image buffer")?;

    let mut jpeg_data = Vec::new();
    let mut cursor = Cursor::new(&mut jpeg_data);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| format!("JPEG encoding failed: {}", e))?;

    Ok(jpeg_data)
}

#[cfg(target_os = "linux")]
fn preview_loop(
    path: String,
    width: u32,
    height: u32,
    mut tx: mpsc::Sender<Vec<u8>>,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<(), String> {
    let mut dev = Device::with_path(path).map_err(|e| format!("open device: {e}"))?;
    let fmt = configure_device(&mut dev, width, height)?;

    println!(
        "Configured format: {}x{}, fourcc: {}, stride: {}",
        fmt.width, fmt.height, fmt.fourcc, fmt.stride
    );

    let is_mjpeg = fmt.fourcc == FourCC::new(b"MJPG");
    let mut frame_count = 0;

    println!("Trying userptr streaming for HDMI capture compatibility...");

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
        Err(e) => {
            eprintln!("Userptr streaming failed: {e}");
            println!("Falling back to mmap streaming...");
        }
    }

    // Fallback to mmap streaming
    try_mmap_streaming(
        &mut dev,
        &fmt,
        is_mjpeg,
        &mut tx,
        &mut frame_count,
        &last_frame,
    )
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
    println!("Creating userptr stream...");

    let mut stream = userptr::Stream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create UserptrStream: {e}"))?;

    println!("Userptr stream created successfully, starting capture...");

    loop {
        match stream.next() {
            Ok((buffer, _meta)) => {
                *frame_count += 1;

                if *frame_count <= 3 {
                    println!(
                        "Userptr Frame {}: {} bytes, format: {}",
                        *frame_count,
                        buffer.len(),
                        if is_mjpeg { "MJPEG" } else { "YUYV" }
                    );
                }

                let jpeg_data = if is_mjpeg {
                    buffer.to_vec()
                } else {
                    // Convert YUYV to JPEG
                    match yuyv_to_jpeg(buffer, fmt.width, fmt.height) {
                        Ok(jpeg) => jpeg,
                        Err(e) => {
                            eprintln!("YUYV to JPEG conversion failed: {}", e);
                            continue;
                        }
                    }
                };

                {
                    let mut lf = last_frame.lock().unwrap();
                    *lf = Some(jpeg_data.clone());
                }
                if tx.blocking_send(jpeg_data).is_err() {
                    println!("Client disconnected, stopping preview loop");
                    break;
                }
            }
            Err(e) => {
                return Err(format!("Userptr stream error: {e}"));
            }
        }
    }

    println!("Userptr preview loop exiting...");
    Ok(())
}

#[cfg(target_os = "linux")]
fn try_mmap_streaming(
    dev: &mut Device,
    fmt: &Format,
    is_mjpeg: bool,
    tx: &mut mpsc::Sender<Vec<u8>>,
    frame_count: &mut usize,
    last_frame: &Arc<Mutex<Option<Vec<u8>>>>,
) -> Result<(), String> {
    let mut stream = MmapStream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create MmapStream: {e}"))?;

    println!("Mmap stream created successfully, starting capture...");

    loop {
        match stream.next() {
            Ok((buffer, _meta)) => {
                *frame_count += 1;

                if *frame_count <= 3 {
                    println!(
                        "Mmap Frame {}: {} bytes, format: {}",
                        *frame_count,
                        buffer.len(),
                        if is_mjpeg { "MJPEG" } else { "YUYV" }
                    );
                }

                let jpeg_data = if is_mjpeg {
                    buffer.to_vec()
                } else {
                    // Convert YUYV to JPEG
                    match yuyv_to_jpeg(buffer, fmt.width, fmt.height) {
                        Ok(jpeg) => jpeg,
                        Err(e) => {
                            eprintln!("YUYV to JPEG conversion failed: {}", e);
                            continue;
                        }
                    }
                };

                {
                    let mut lf = last_frame.lock().unwrap();
                    *lf = Some(jpeg_data.clone());
                }
                if tx.blocking_send(jpeg_data).is_err() {
                    println!("Client disconnected, stopping preview loop");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Mmap stream error: {e}");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    println!("Mmap preview loop exiting...");
    Ok(())
}

#[cfg(target_os = "linux")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let last_frame = Arc::new(Mutex::new(None::<Vec<u8>>));

    #[cfg(target_os = "linux")]
    let printer = match new_printer().await {
        Ok(p) => {
            eprintln!("Printer initialized: {}", p.type_name());
            Some(p)
        }
        Err(e) => {
            eprintln!("Failed to initialize printer: {}", e);
            None
        }
    };

    #[cfg(not(target_os = "linux"))]
    let printer = None;

    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(last_frame.clone()))
            .service(index)
            .service(preview_stream)
            .service(capture_image)
            .service(photo_page)
            .service(
                fs::Files::new("/images", "/usr/local/share/photo_booth").show_files_listing(),
            );

        if let Some(p) = printer.clone() {
            app = app.app_data(web::Data::new(p)).service(print_photo);
        }

        app
    })
    // Bind to all interfaces so you can reach it from other devices on the LAN
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[cfg(target_os = "linux")]
#[get("/")]
async fn index() -> impl Responder {
    let html = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8"/>
<title>V4L2 Camera Preview (MJPEG)</title>
<style>
body { background:black; color:white; font-family:sans-serif; text-align:center; margin:0; }
#wrap { padding: 20px; }
#countdown { position:absolute; top:40%; left:0; right:0; font-size:80px; }
button { font-size: 20px; padding: 10px 20px; }
img { background:#222; }
</style>
</head>
<body>
<div id="wrap">
<h1>V4L2 Camera Preview (MJPEG)</h1>
<p>Set VIDEO_DEVICE (default /dev/video1), VIDEO_WIDTH (default 1920), VIDEO_HEIGHT (default 1080).</p>
<div style="position:relative; display:inline-block;">
    <img id="preview" src="/preview" width="800" alt="Preview stream"/>
    <div id="countdown"></div>
</div><br/>
<button id="startBtn" onclick="startCountdown()">Start Countdown</button>
</div>

<script>
let countdownRunning = false;
function startCountdown() {
    if (countdownRunning) return;
    countdownRunning = true;
    const btn = document.getElementById("startBtn");
    if (btn) btn.disabled = true;

    let t = 3;
    const countdown = document.getElementById("countdown");
    countdown.textContent = t;
    const interval = setInterval(() => {
        t--;
        if (t > 0) {
            countdown.textContent = t;
        } else {
            clearInterval(interval);
            countdown.textContent = "";
            fetch('/capture', { method: 'POST' })
                .then(r => r.json())
                .then(d => {
                    if (d && d.ok) {
                        if (d.redirect) {
                            window.location.href = d.redirect;
                        } else if (d.path) {
                            window.location.href = '/photo?path=' + encodeURIComponent(d.path);
                        } else if (d.file) {
                            window.location.href = '/photo?file=' + encodeURIComponent(d.file);
                        } else {
                            if (btn) btn.disabled = false;
                            countdownRunning = false;
                            alert('Capture failed: missing redirect or path');
                        }
                    } else {
                        if (btn) btn.disabled = false;
                        countdownRunning = false;
                        alert('Capture failed');
                    }
                })
                .catch(e => {
                    if (btn) btn.disabled = false;
                    countdownRunning = false;
                    alert('Capture failed: ' + e);
                });
        }
    }, 1000);
}
</script>
</body>
</html>
"#;
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[cfg(target_os = "linux")]
#[get("/photo")]
async fn photo_page(
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let path = query.get("path").cloned().unwrap_or_default();
    let file = query.get("file").cloned().unwrap_or_default();
    let html = format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8"/>
<title>Captured Photo</title>
<style>
body {{ background:black; color:white; font-family:sans-serif; text-align:center; margin:0; }}
#wrap {{ padding: 20px; }}
button {{ font-size: 20px; padding: 10px 20px; }}
img {{ background:#222; max-width: 90vw; height: auto; }}
</style>
</head>
<body>
<div id="wrap">
<h1>Captured Image</h1>
{img_tag}
<br/><br/>
<button onclick="window.location.href='/'">Back to Start</button>
<button onclick="printPhoto()">Print Photo (4x6)</button>
</div>

<script>
function printPhoto() {{
    const urlParams = new URLSearchParams(window.location.search);
    const path = urlParams.get('path') || '';
    const file = urlParams.get('file') || '';

    let filename = '';
    if (path && path.startsWith('/images/')) {{
        filename = path.substring('/images/'.length);
    }} else if (file) {{
        filename = file;
    }} else if (path && !path.includes('/')) {{
        filename = path;
    }}

    if (!filename) {{
        alert('No image file found to print');
        return;
    }}

    fetch('/print', {{
        method: 'POST',
        headers: {{
            'Content-Type': 'application/json',
        }},
        body: JSON.stringify({{ filename: filename }})
    }})
    .then(response => response.json())
    .then(data => {{
        if (data.ok) {{
            alert('Print job submitted successfully! Job ID: ' + data.job_id);
        }} else {{
            alert('Print failed: ' + (data.error || 'Unknown error'));
        }}
    }})
    .catch(error => {{
        alert('Print failed: ' + error);
    }});
}}
</script>
</body>
</html>
"#,
        img_tag = {
            let p = path.as_str();
            let f = file.as_str();

            // Accept either:
            // - path like "/images/<filename>"
            // - file like "<filename>"
            // And require a safe filename: only [A-Za-z0-9._-], no slashes.
            let candidate = if let Some(fname) = p.strip_prefix("/images/") {
                Some(fname)
            } else if !p.is_empty() && !p.contains('/') {
                Some(p)
            } else if !f.is_empty() && !f.contains('/') {
                Some(f)
            } else {
                None
            };

            if let Some(fname) = candidate {
                if fname
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
                {
                    format!(r#"<img src="/images/{}" alt="Captured"/>"#, fname)
                } else {
                    "<p>Invalid image name</p>".to_string()
                }
            } else if p.is_empty() && f.is_empty() {
                "<p>No image specified</p>".to_string()
            } else {
                "<p>Invalid image path</p>".to_string()
            }
        }
    );
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

#[cfg(target_os = "linux")]
#[get("/preview")]
async fn preview_stream(last_frame: web::Data<Arc<Mutex<Option<Vec<u8>>>>>) -> impl Responder {
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(8);
    let (path, width, height) = video_settings();

    // Run V4L2 capture in a blocking thread, forward frames over channel
    let last_frame_arc = last_frame.get_ref().clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = preview_loop(path, width, height, tx, last_frame_arc) {
            eprintln!("Preview loop terminated: {e}");
        }
    });

    let stream = async_stream::stream! {
        // multipart/x-mixed-replace; boundary=frame
        const BOUNDARY: &str = "frame";
        let boundary_prefix = format!("--{}\r\n", BOUNDARY).into_bytes();
        let header = b"Content-Type: image/jpeg\r\n\r\n";
        let tail = b"\r\n";

        while let Some(frame) = rx.recv().await {
            let mut part = Vec::with_capacity(boundary_prefix.len() + header.len() + frame.len() + tail.len());
            part.extend_from_slice(&boundary_prefix);
            part.extend_from_slice(header);
            part.extend_from_slice(&frame);
            part.extend_from_slice(tail);

            // Convert to bytes and yield a Result<Bytes, actix_web::Error>
            yield Ok::<Bytes, actix_web::Error>(Bytes::from(part));
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "multipart/x-mixed-replace; boundary=frame"))
        // NOTE: no .no_chunking() call — streaming will chunk as appropriate
        .streaming(stream)
}

#[cfg(target_os = "linux")]
#[post("/capture")]
async fn capture_image(last_frame: web::Data<Arc<Mutex<Option<Vec<u8>>>>>) -> impl Responder {
    std::fs::create_dir_all("/usr/local/share/photo_booth").ok();

    // Set proper permissions on captures directory for CUPS access
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) =
            std::fs::metadata("/usr/local/share/photo_booth").and_then(|m| Ok(m.permissions()))
        {
            perms.set_mode(0o755); // rwxr-xr-x (readable by lp user)
            let _ = std::fs::set_permissions("/usr/local/share/photo_booth", perms);
        }
    }
    let filename = format!(
        "/usr/local/share/photo_booth/cap_{}.png",
        chrono::Utc::now().timestamp()
    );

    let img_opt = { last_frame.lock().unwrap().clone() };
    match img_opt {
        Some(bytes) => {
            let save_path = filename.clone();
            let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
                // Convert JPEG bytes to PNG format using image crate
                let img =
                    image::load_from_memory(&bytes).map_err(|e| format!("decode image: {e}"))?;
                img.save(&save_path).map_err(|e| format!("save PNG: {e}"))?;

                // Set proper permissions on the captured file for CUPS access
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mut perms) =
                        std::fs::metadata(&save_path).and_then(|m| Ok(m.permissions()))
                    {
                        perms.set_mode(0o644); // rw-r--r--
                        let _ = std::fs::set_permissions(&save_path, perms);
                    }
                }

                Ok(())
            })
            .await;

            match res {
                Ok(Ok(())) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "ok": true,
                        "path": format!("/images/{}", std::path::Path::new(&filename).file_name().unwrap().to_string_lossy()),
                        "file": std::path::Path::new(&filename).file_name().unwrap().to_string_lossy(),
                        "redirect": format!("/photo?file={}", std::path::Path::new(&filename).file_name().unwrap().to_string_lossy()),
                    }))
                }
                Ok(Err(e)) => {
                    eprintln!("capture error: {e}");
                    HttpResponse::InternalServerError().json(serde_json::json!({ "ok": false, "error": e }))
                }
                Err(e) => {
                    eprintln!("capture task join error: {e}");
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "ok": false, "error": "join error" }))
                }
            }
        }
        None => HttpResponse::InternalServerError().json(serde_json::json!({
            "ok": false,
            "error": "no frame available yet"
        })),
    }
}

#[cfg(target_os = "linux")]
#[post("/print")]
async fn print_photo(
    printer: web::Data<Arc<dyn Printer + Send + Sync>>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let filename = match body.get("filename").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "ok": false,
                "error": "filename is required"
            }));
        }
    };

    // Validate filename for security
    if filename.contains('/') || filename.contains("..") {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "ok": false,
            "error": "Invalid filename"
        }));
    }

    let file_path = format!("/usr/local/share/photo_booth/{}", filename);

    // Check if file exists
    if !std::path::Path::new(&file_path).exists() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "ok": false,
            "error": "Image file not found"
        }));
    }

    let print_job = PrintJob {
        file_path,
        copies: 1,
        paper_size: PaperSize::Photo4x6,
        quality: PrintQuality::High,
    };

    match printer.print_photo(print_job).await {
        Ok(job_id) => {
            eprintln!("Print endpoint: Job {} submitted successfully", job_id);
            HttpResponse::Ok().json(serde_json::json!({
                "ok": true,
                "job_id": job_id,
                "message": format!("Print job submitted successfully. Job ID: {}", job_id)
            }))
        }
        Err(e) => {
            eprintln!("Print endpoint error: {}", e);
            eprintln!("Print endpoint error type: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "ok": false,
                "error": format!("Print failed: {}", e)
            }))
        }
    }
}

#[cfg(target_os = "linux")]
fn try_userptr_capture(dev: &mut Device, fmt: &Format, is_mjpeg: bool) -> Result<Vec<u8>, String> {
    let mut stream = userptr::Stream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create userptr capture stream: {e}"))?;

    // Grab a few frames and keep the last
    let mut image: Option<Vec<u8>> = None;
    for attempt in 0..5 {
        match stream.next() {
            Ok((buffer, _meta)) => {
                println!(
                    "Userptr captured frame {} with {} bytes",
                    attempt + 1,
                    buffer.len()
                );

                let jpeg_data = if is_mjpeg {
                    buffer.to_vec()
                } else {
                    match yuyv_to_jpeg(buffer, fmt.width, fmt.height) {
                        Ok(jpeg) => jpeg,
                        Err(e) => {
                            eprintln!("YUYV to JPEG conversion failed: {}", e);
                            continue;
                        }
                    }
                };
                image = Some(jpeg_data);

                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            Err(e) => {
                return Err(format!(
                    "userptr capture error on attempt {}: {}",
                    attempt + 1,
                    e
                ));
            }
        }
    }
    image.ok_or_else(|| "No frame captured after 5 attempts".to_string())
}

#[cfg(target_os = "linux")]
fn try_mmap_capture(dev: &mut Device, fmt: &Format, is_mjpeg: bool) -> Result<Vec<u8>, String> {
    let mut stream = MmapStream::with_buffers(dev, Type::VideoCapture, 4)
        .map_err(|e| format!("Failed to create mmap capture stream: {e}"))?;

    // Grab a few frames and keep the last
    let mut image: Option<Vec<u8>> = None;
    for attempt in 0..5 {
        match stream.next() {
            Ok((buffer, _meta)) => {
                println!(
                    "Mmap captured frame {} with {} bytes",
                    attempt + 1,
                    buffer.len()
                );

                let jpeg_data = if is_mjpeg {
                    buffer.to_vec()
                } else {
                    match yuyv_to_jpeg(buffer, fmt.width, fmt.height) {
                        Ok(jpeg) => jpeg,
                        Err(e) => {
                            eprintln!("YUYV to JPEG conversion failed: {}", e);
                            continue;
                        }
                    }
                };
                image = Some(jpeg_data);

                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            Err(e) => {
                eprintln!("mmap capture error on attempt {}: {}", attempt + 1, e);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    image.ok_or_else(|| "No frame captured after 5 attempts".to_string())
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::io::Result<()> {
    eprintln!(
        "This binary is intended to run on Linux (Raspberry Pi). The V4L2-based preview and capture are Linux-only."
    );
    eprintln!("Build for the target device (e.g., aarch64-unknown-linux-gnu) and run it there.");
    Ok(())
}
