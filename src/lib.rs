// Library module organization

// Printer functionality
pub mod printers;

// Template functionality
pub mod templates;

// Re-export commonly used types for convenience
#[cfg(target_os = "linux")]
pub use printers::{
    new_printer, MockPrinter, PaperSize, PrintJob, PrintQuality, Printer, PrinterError,
    PrinterStatus,
};

#[cfg(all(target_os = "linux", feature = "printer-cups"))]
pub use printers::EpsonPrinter;

// Template exports
#[cfg(target_os = "linux")]
pub use templates::{create_templated_print_with_text, PrintTemplate, TemplateError};
