use async_trait::async_trait;
use chrono;
use image;
#[cfg(feature = "printer-cups")]
use printers::{
    common::base::job::PrinterJobOptions, common::base::printer::Printer as PrintersCratePrinter,
    get_printers,
};
use serde::Serialize;
use std::error::Error;
use std::fmt;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
pub enum PaperSize {
    Letter,
    A4,
    Photo4x6,
    Photo5x7,
}

#[derive(Debug, Clone, Serialize)]
pub enum PrintQuality {
    Draft,
    Normal,
    High,
}

#[derive(Debug)]
pub struct PrintJob {
    pub file_path: String,
    pub copies: u32,
    pub paper_size: PaperSize,
    pub quality: PrintQuality,
}

#[derive(Debug, Serialize)]
pub struct PrinterStatus {
    pub is_online: bool,
    pub paper_level: Option<u8>,
    pub toner_level: Option<u8>,
    pub error_message: Option<String>,
}

#[derive(Debug)]
pub enum PrinterError {
    NotFound(String),
    NotReady(String),
    PrintFailed(String),
    IoError(String),
}

impl fmt::Display for PrinterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrinterError::NotFound(msg) => write!(f, "Printer not found: {}", msg),
            PrinterError::NotReady(msg) => write!(f, "Printer not ready: {}", msg),
            PrinterError::PrintFailed(msg) => write!(f, "Print failed: {}", msg),
            PrinterError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl Error for PrinterError {}

// Printer trait
#[async_trait]
pub trait Printer: Send + Sync {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError>;
    async fn is_ready(&self) -> bool;
    async fn get_status(&self) -> Result<PrinterStatus, PrinterError>;
    fn type_name(&self) -> &'static str;
}

// Epson printer implementation
#[cfg(feature = "printer-cups")]
pub struct EpsonPrinter {
    printer_name: String,
    cups_printer: Option<PrintersCratePrinter>,
}

#[cfg(feature = "printer-cups")]
impl EpsonPrinter {
    pub async fn new(printer_name: &str) -> Result<Self, PrinterError> {
        // Get all available printers
        let printers = get_printers();

        info!("Looking for printer: {}", printer_name);
        info!("Available printers:");
        for printer in &printers {
            info!(
                "  - Name: '{}', System Name: '{}', URI: '{}'",
                printer.name, printer.system_name, printer.uri
            );
        }

        // Look for exact match first, TODO clean this up
        let cups_printer = printers
            .iter()
            .find(|p| {
                p.name == "XP8700series-TurboPrint" || p.system_name == "XP8700series-TurboPrint"
            })
            .or_else(|| printers.iter().find(|p| p.name == printer_name))
            .or_else(|| {
                printers
                    .iter()
                    .find(|p| p.name.contains("XP8700") && p.name.contains("TurboPrint"))
            })
            .or_else(|| printers.iter().find(|p| p.name.contains("TurboPrint")))
            .cloned();

        match cups_printer {
            Some(printer) => {
                info!(
                    "Selected printer: '{}' (System: '{}')",
                    printer.name, printer.system_name
                );
                Ok(EpsonPrinter {
                    printer_name: printer.name.clone(),
                    cups_printer: Some(printer),
                })
            }
            None => {
                warn!("Printer '{}' not found in CUPS", printer_name);
                warn!("Available printers were:");
                for printer in &printers {
                    warn!("  - '{}'", printer.name);
                }
                Err(PrinterError::NotFound(format!(
                    "TurboPrint printer '{}' not found in CUPS",
                    printer_name
                )))
            }
        }
    }
}

#[cfg(feature = "printer-cups")]
#[async_trait]
impl Printer for EpsonPrinter {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError> {
        info!(
            "Starting print job: {} copies of {}",
            job.copies, job.file_path
        );

        let printer = self
            .cups_printer
            .as_ref()
            .ok_or_else(|| PrinterError::NotReady("Printer not initialized".to_string()))?;

        info!("Using printer: {}", printer.name);

        // Check if file exists
        let file_path = std::path::Path::new(&job.file_path);
        if !file_path.exists() {
            return Err(PrinterError::IoError(format!(
                "File not found: {}",
                job.file_path
            )));
        }

        // Validate image file
        if let Ok(file_bytes) = std::fs::read(&job.file_path) {
            if let Err(e) = image::load_from_memory(&file_bytes) {
                return Err(PrinterError::IoError(format!(
                    "Image file validation failed: {}",
                    e
                )));
            }
        } else {
            return Err(PrinterError::IoError(format!(
                "Cannot read file: {}",
                job.file_path
            )));
        }

        // Set proper permissions on original file for CUPS access
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(mut perms) =
                std::fs::metadata(&job.file_path).and_then(|m| Ok(m.permissions()))
            {
                perms.set_mode(0o644); // rw-r--r--
                let _ = std::fs::set_permissions(&job.file_path, perms);
            }
        }

        let print_file_path = job.file_path.clone();

        // Set up printer options
        let mut raw_properties = Vec::new();

        let paper_size_str = match job.paper_size {
            PaperSize::Photo4x6 => "Borderless4x6in",
            PaperSize::Photo5x7 => "Borderless5x7in",
            _ => "Letter",
        };
        raw_properties.push(("PageSize", paper_size_str.to_string()));

        if matches!(job.paper_size, PaperSize::Photo4x6 | PaperSize::Photo5x7) {
            raw_properties.push(("MediaType", "EpsonPremiumGlossy_6".to_string()));
            raw_properties.push(("zedoBorderlessExpand", "4".to_string()));
        }

        raw_properties.push(("copies", job.copies.to_string()));
        info!(
            "Print settings: {} copies, Paper: {:?}, Quality: {:?}",
            job.copies, job.paper_size, job.quality
        );

        let job_name = format!("PhotoBooth-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
        raw_properties.push(("job-name", job_name.clone()));

        let raw_props: Vec<(&str, &str)> = raw_properties
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let options = PrinterJobOptions {
            name: Some(&job_name),
            raw_properties: &raw_props,
        };

        // This success messaged doesn't actually mean the print itself was successful,
        // it only shows that the job was loaded into CUPS successfully, and
        // CUPS itself may fail to print the photo for a variety of reasons
        match printer.print_file(&print_file_path, options) {
            Ok(job_id) => {
                info!("Print job submitted successfully with ID: {}", job_id);
                Ok(job_id.to_string())
            }
            Err(e) => {
                warn!("Print job failed: {}", e);
                Err(PrinterError::PrintFailed(format!(
                    "CUPS print error: {}",
                    e
                )))
            }
        }
    }

    async fn is_ready(&self) -> bool {
        self.cups_printer.is_some()
    }

    async fn get_status(&self) -> Result<PrinterStatus, PrinterError> {
        if self.cups_printer.is_some() {
            Ok(PrinterStatus {
                is_online: true,
                paper_level: None,
                toner_level: None,
                error_message: None,
            })
        } else {
            Err(PrinterError::NotReady(
                "Printer not initialized".to_string(),
            ))
        }
    }

    fn type_name(&self) -> &'static str {
        "Epson XP-8700 (TurboPrint)"
    }
}

// Mock printer implementation for testing or when no real printer is available
pub struct MockPrinter;

#[async_trait]
impl Printer for MockPrinter {
    async fn print_photo(&self, _job: PrintJob) -> Result<String, PrinterError> {
        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Generate a mock job ID
        let job_id = format!("mock-job-{}", chrono::Utc::now().timestamp());
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

// Factory function to create appropriate printer instance
#[cfg(feature = "printer-cups")]
pub async fn new_printer() -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    info!("Initializing printer system...");
    new_printer_with_config(
        "XP8700series-TurboPrint",
        &[
            "EPSON_XP_8700_Series_USB",
            "XP-8700",
            "EPSON_XP-8700_Series",
        ],
    )
    .await
}

#[cfg(feature = "printer-cups")]
pub async fn new_printer_with_config(
    primary_name: &str,
    fallback_names: &[&str],
) -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    info!("Attempting to connect to printer: {}", primary_name);

    // Try primary printer first
    match EpsonPrinter::new(primary_name).await {
        Ok(printer) => {
            info!("Successfully connected to primary printer");
            return Ok(std::sync::Arc::new(printer));
        }
        Err(e) => {
            warn!("Failed to connect to primary printer: {}", e);

            // Try fallback printers
            for name in fallback_names {
                info!("Trying fallback printer: {}", name);
                match EpsonPrinter::new(name).await {
                    Ok(printer) => {
                        info!("Successfully connected to fallback printer: {}", name);
                        return Ok(std::sync::Arc::new(printer));
                    }
                    Err(e) => {
                        warn!("Failed to connect to {}: {}", name, e);
                        continue;
                    }
                }
            }
        }
    }

    // Fall back to mock printer
    warn!("No physical printer found, using mock printer for testing");
    Ok(std::sync::Arc::new(MockPrinter))
}

#[cfg(not(feature = "printer-cups"))]
pub async fn new_printer() -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    // When CUPS feature is not enabled, always use mock printer
    Ok(std::sync::Arc::new(MockPrinter))
}
