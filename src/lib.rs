// Library module organization

// Printer functionality
pub mod printers;

// Session functionality
pub mod session;

// Template functionality
pub mod templates;

pub mod errors;

// Configuration module
pub mod config;

// GPhoto2 camera functionality
pub mod gphoto_camera;

// Re-export commonly used types for convenience
pub use printers::{
    new_printer, MockPrinter, PaperSize, PrintJob, PrintQuality, Printer, PrinterError,
    PrinterStatus,
};

#[cfg(feature = "printer-cups")]
pub use printers::EpsonPrinter;

// Session exports
pub use session::Session;

// Template exports
pub use templates::{create_templated_print_with_background, PrintTemplate, TemplateError};
