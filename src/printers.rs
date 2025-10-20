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
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize)]
pub enum PaperSize {
    Letter,
    A4,
    Photo4x6,
    Photo5x7,
    Custom(String),
}

#[derive(Debug, Clone, Serialize)]
pub enum PrintQuality {
    Draft,
    Normal,
    High,
    Photo,
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

// Configuration for different printer models
#[derive(Debug, Clone)]
pub struct PrinterConfig {
    pub primary_name: String,
    pub fallback_names: Vec<String>,
    pub driver_ppd: String,
    pub default_paper_size: String,
    pub default_resolution: String,
    pub custom_options: Vec<(String, String)>,
}

impl PrinterConfig {
    /// Configuration for DNP DS620 printer with gutenprint
    pub fn dnp_ds620() -> Self {
        Self {
            primary_name: "DNP_DS620_Photo".to_string(),
            fallback_names: vec![
                "DS620".to_string(),
                "DNP-DS620".to_string(),
                "DNP_DS620".to_string(),
            ],
            driver_ppd: "gutenprint.5.3://dnp-ds620/expert".to_string(),
            default_paper_size: "w288h432".to_string(), // 4x6 inches
            default_resolution: "300x300dpi".to_string(),
            custom_options: vec![
                ("StpiShrinkOutput".to_string(), "Expand".to_string()),
                ("StpLaminate".to_string(), "Glossy".to_string()),
                ("StpImageType".to_string(), "Photo".to_string()),
            ],
        }
    }

    /// Configuration for Epson XP-8700 with TurboPrint (legacy support)
    pub fn epson_xp8700_turboprint() -> Self {
        Self {
            primary_name: "XP8700series-TurboPrint".to_string(),
            fallback_names: vec![
                "EPSON_XP_8700_Series_USB".to_string(),
                "XP-8700".to_string(),
                "EPSON_XP-8700_Series".to_string(),
            ],
            driver_ppd: "".to_string(), // TurboPrint manages its own PPD
            default_paper_size: "Borderless4x6in".to_string(),
            default_resolution: "360x360dpi".to_string(),
            custom_options: vec![(
                "MediaType".to_string(),
                "ZedonetPhotoGlossy200g_6".to_string(),
            )],
        }
    }
}

// Generic CUPS printer implementation
#[cfg(feature = "printer-cups")]
pub struct CupsPrinter {
    printer_name: String,
    cups_printer: Option<PrintersCratePrinter>,
    config: PrinterConfig,
}

#[cfg(feature = "printer-cups")]
impl CupsPrinter {
    pub async fn new(config: PrinterConfig) -> Result<Self, PrinterError> {
        info!(
            "Initializing CUPS printer with configuration for: {}",
            config.primary_name
        );

        // Get all available printers
        let printers = get_printers();

        info!("Found {} printer(s) in CUPS", printers.len());
        debug!("Available printers:");
        for printer in &printers {
            debug!(
                "  - Name: '{}', System Name: '{}', URI: '{}'",
                printer.name, printer.system_name, printer.uri
            );
        }

        // Try to find the printer with various strategies
        let cups_printer = Self::find_printer(&printers, &config);

        match cups_printer {
            Some(printer) => {
                info!(
                    "Successfully connected to printer: '{}' (System: '{}')",
                    printer.name, printer.system_name
                );
                Ok(CupsPrinter {
                    printer_name: printer.name.clone(),
                    cups_printer: Some(printer),
                    config,
                })
            }
            None => {
                warn!("Printer '{}' not found in CUPS", config.primary_name);
                warn!("Searched for:");
                warn!("  Primary: {}", config.primary_name);
                for name in &config.fallback_names {
                    warn!("  Fallback: {}", name);
                }
                warn!("Available printers were:");
                for printer in &printers {
                    warn!("  - '{}'", printer.name);
                }
                Err(PrinterError::NotFound(format!(
                    "Printer '{}' not found in CUPS",
                    config.primary_name
                )))
            }
        }
    }

    fn find_printer(
        printers: &[PrintersCratePrinter],
        config: &PrinterConfig,
    ) -> Option<PrintersCratePrinter> {
        // Try exact match with primary name
        info!("Looking for exact match with: {}", config.primary_name);
        if let Some(p) = printers
            .iter()
            .find(|p| p.name == config.primary_name || p.system_name == config.primary_name)
        {
            info!("Found exact match");
            return Some(p.clone());
        }

        // Try fallback names
        for fallback_name in &config.fallback_names {
            info!("Trying fallback name: {}", fallback_name);
            if let Some(p) = printers
                .iter()
                .find(|p| &p.name == fallback_name || &p.system_name == fallback_name)
            {
                info!("Found match with fallback name");
                return Some(p.clone());
            }

            // Try case-insensitive match
            if let Some(p) = printers.iter().find(|p| {
                p.name.to_lowercase() == fallback_name.to_lowercase()
                    || p.system_name.to_lowercase() == fallback_name.to_lowercase()
            }) {
                info!("Found case-insensitive match with fallback name");
                return Some(p.clone());
            }

            // Try partial match
            if let Some(p) = printers.iter().find(|p| {
                p.name.contains(fallback_name.as_str())
                    || p.system_name.contains(fallback_name.as_str())
            }) {
                info!("Found partial match with fallback name");
                return Some(p.clone());
            }
        }

        None
    }

    fn get_paper_size_string(&self, paper_size: &PaperSize) -> String {
        match paper_size {
            PaperSize::Photo4x6 => {
                // For DNP printers, use specific size format
                if self.config.primary_name.contains("DNP") {
                    "w288h432".to_string() // 4x6 inches at 72 DPI
                } else {
                    "Borderless4x6in".to_string()
                }
            }
            PaperSize::Photo5x7 => {
                if self.config.primary_name.contains("DNP") {
                    "w360h504".to_string() // 5x7 inches at 72 DPI
                } else {
                    "Borderless5x7in".to_string()
                }
            }
            PaperSize::Letter => "Letter".to_string(),
            PaperSize::A4 => "A4".to_string(),
            PaperSize::Custom(size) => size.clone(),
        }
    }

    fn get_resolution_string(&self, quality: &PrintQuality) -> String {
        match quality {
            PrintQuality::Draft => "150x150dpi".to_string(),
            PrintQuality::Normal => "300x300dpi".to_string(),
            PrintQuality::High => "600x600dpi".to_string(),
            PrintQuality::Photo => {
                // Use configured default for photo quality
                self.config.default_resolution.clone()
            }
        }
    }
}

#[cfg(feature = "printer-cups")]
#[async_trait]
impl Printer for CupsPrinter {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError> {
        info!(
            "Starting print job for {}: {} copies of {}",
            self.printer_name, job.copies, job.file_path
        );

        let printer = self
            .cups_printer
            .as_ref()
            .ok_or_else(|| PrinterError::NotReady("Printer not initialized".to_string()))?;

        // Check if file exists
        let file_path = std::path::Path::new(&job.file_path);
        if !file_path.exists() {
            return Err(PrinterError::IoError(format!(
                "File not found: {}",
                job.file_path
            )));
        }

        // Validate image file
        match std::fs::read(&job.file_path) {
            Ok(file_bytes) => {
                if let Err(e) = image::load_from_memory(&file_bytes) {
                    return Err(PrinterError::IoError(format!(
                        "Image file validation failed: {}",
                        e
                    )));
                }
            }
            Err(e) => {
                return Err(PrinterError::IoError(format!(
                    "Cannot read file {}: {}",
                    job.file_path, e
                )));
            }
        }

        // Set proper permissions on original file for CUPS access
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&job.file_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o644); // rw-r--r--
                if let Err(e) = std::fs::set_permissions(&job.file_path, perms) {
                    warn!("Failed to set file permissions: {}", e);
                }
            }
        }

        // Set up printer options
        let mut raw_properties = Vec::new();

        // Paper size
        let paper_size_str = self.get_paper_size_string(&job.paper_size);
        raw_properties.push(("PageSize", paper_size_str.clone()));
        debug!("Paper size: {}", paper_size_str);

        // Resolution based on quality
        let resolution = self.get_resolution_string(&job.quality);
        raw_properties.push(("Resolution", resolution.clone()));
        debug!("Resolution: {}", resolution);

        // Number of copies
        raw_properties.push(("copies", job.copies.to_string()));

        // Add custom options from config
        for (key, value) in &self.config.custom_options {
            raw_properties.push((key.as_str(), value.clone()));
            debug!("Custom option: {} = {}", key, value);
        }

        // Job name with timestamp
        let job_name = format!("PhotoBooth-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
        raw_properties.push(("job-name", job_name.clone()));

        info!(
            "Print settings: {} copies, Paper: {:?}, Quality: {:?}, Resolution: {}",
            job.copies, job.paper_size, job.quality, resolution
        );

        // Convert to the format expected by the CUPS API
        let raw_props: Vec<(&str, &str)> = raw_properties
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();

        let options = PrinterJobOptions {
            name: Some(&job_name),
            raw_properties: &raw_props,
        };

        // Submit the print job
        match printer.print_file(&job.file_path, options) {
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
        if self.config.primary_name.contains("DNP") {
            "DNP DS620 Photo Printer"
        } else if self.config.primary_name.contains("XP8700")
            || self.config.primary_name.contains("XP-8700")
        {
            "Epson XP-8700 (TurboPrint)"
        } else {
            "CUPS Printer"
        }
    }
}

// Mock printer implementation for testing or when no real printer is available
pub struct MockPrinter;

#[async_trait]
impl Printer for MockPrinter {
    async fn print_photo(&self, job: PrintJob) -> Result<String, PrinterError> {
        info!(
            "MockPrinter: Simulating print of {} ({} copies)",
            job.file_path, job.copies
        );

        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Generate a mock job ID
        let job_id = format!("mock-job-{}", chrono::Utc::now().timestamp());
        info!("MockPrinter: Generated job ID: {}", job_id);
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
        "Mock Printer (Testing Mode)"
    }
}

// Factory function to create appropriate printer instance
#[cfg(feature = "printer-cups")]
pub async fn new_printer() -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    info!("=== Initializing Photo Booth Printer System ===");

    // Try DNP DS620 first (new default)
    let dnp_config = PrinterConfig::dnp_ds620();
    info!("Attempting to connect to DNP DS620 printer...");

    match CupsPrinter::new(dnp_config).await {
        Ok(printer) => {
            info!("✓ Successfully connected to DNP DS620 printer");
            return Ok(std::sync::Arc::new(printer));
        }
        Err(e) => {
            warn!("DNP DS620 not found: {}", e);
            info!("Falling back to legacy Epson configuration...");
        }
    }

    // Try Epson XP-8700 with TurboPrint (legacy fallback)
    let epson_config = PrinterConfig::epson_xp8700_turboprint();
    info!("Attempting to connect to Epson XP-8700 printer...");

    match CupsPrinter::new(epson_config).await {
        Ok(printer) => {
            info!("✓ Successfully connected to Epson XP-8700 printer");
            return Ok(std::sync::Arc::new(printer));
        }
        Err(e) => {
            warn!("Epson XP-8700 not found: {}", e);
        }
    }

    // Fall back to mock printer
    warn!("═══════════════════════════════════════════════════════");
    warn!("No physical printer found - using Mock Printer");
    warn!("This is suitable for development and testing only");
    warn!("═══════════════════════════════════════════════════════");
    info!("MockPrinter initialized for testing");

    Ok(std::sync::Arc::new(MockPrinter))
}

#[cfg(feature = "printer-cups")]
pub async fn new_printer_with_config(
    config: PrinterConfig,
) -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    info!(
        "Initializing printer with custom configuration: {}",
        config.primary_name
    );

    match CupsPrinter::new(config).await {
        Ok(printer) => {
            info!("Successfully connected to configured printer");
            Ok(std::sync::Arc::new(printer))
        }
        Err(e) => {
            warn!("Failed to connect to configured printer: {}", e);
            warn!("Falling back to mock printer for testing");
            Ok(std::sync::Arc::new(MockPrinter))
        }
    }
}

#[cfg(not(feature = "printer-cups"))]
pub async fn new_printer() -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    info!("CUPS feature not enabled - using Mock Printer");
    Ok(std::sync::Arc::new(MockPrinter))
}

#[cfg(not(feature = "printer-cups"))]
pub async fn new_printer_with_config(
    _config: PrinterConfig,
) -> Result<std::sync::Arc<dyn Printer + Send + Sync>, PrinterError> {
    info!("CUPS feature not enabled - using Mock Printer");
    Ok(std::sync::Arc::new(MockPrinter))
}
