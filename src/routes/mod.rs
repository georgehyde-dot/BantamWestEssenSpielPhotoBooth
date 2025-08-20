// Route modules organization

#[cfg(target_os = "linux")]
pub mod session_routes;

// Re-export all routes for convenience
#[cfg(target_os = "linux")]
pub use session_routes::*;
