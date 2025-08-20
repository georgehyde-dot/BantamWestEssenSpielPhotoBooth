// Library module organization

// Printer functionality
pub mod printers;

// Session functionality
pub mod session;

// Template functionality
pub mod templates;

pub mod errors;

// Re-export commonly used types for convenience
#[cfg(target_os = "linux")]
pub use printers::{
    new_printer, MockPrinter, PaperSize, PrintJob, PrintQuality, Printer, PrinterError,
    PrinterStatus,
};

#[cfg(all(target_os = "linux", feature = "printer-cups"))]
pub use printers::EpsonPrinter;

// Session exports
#[cfg(target_os = "linux")]
pub use session::Session;

// Template exports
#[cfg(target_os = "linux")]
pub use templates::{
    create_templated_print_with_background, create_templated_print_with_text, PrintTemplate,
    TemplateError,
};
