// Library module organization

// Printer functionality
pub mod printers;

// Re-export commonly used types for convenience
#[cfg(target_os = "linux")]
pub use printers::{
    new_printer, MockPrinter, PaperSize, PrintJob, PrintQuality, Printer, PrinterError,
    PrinterStatus,
};

#[cfg(all(target_os = "linux", feature = "printer-cups"))]
pub use printers::EpsonPrinter;
