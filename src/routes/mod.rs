// Route modules organization

pub mod base_routes;
pub mod camera_routes;
pub mod printer_routes;
pub mod selection_routes;
pub mod session_routes;

// Re-export all routes for convenience
pub use base_routes::*;
pub use camera_routes::*;
pub use printer_routes::*;
pub use selection_routes::*;
pub use session_routes::*;
