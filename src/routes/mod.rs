// Route modules organization

#[cfg(target_os = "linux")]
pub mod base_routes;
#[cfg(target_os = "linux")]
pub mod camera_routes;
#[cfg(target_os = "linux")]
pub mod printer_routes;
#[cfg(target_os = "linux")]
pub mod selection_routes;
#[cfg(target_os = "linux")]
pub mod session_routes;

// Re-export all routes for convenience
#[cfg(target_os = "linux")]
pub use base_routes::*;
#[cfg(target_os = "linux")]
pub use camera_routes::*;
#[cfg(target_os = "linux")]
pub use printer_routes::*;
#[cfg(target_os = "linux")]
pub use selection_routes::*;
#[cfg(target_os = "linux")]
pub use session_routes::*;
