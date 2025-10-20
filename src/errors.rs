use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Initialization error: {0}")]
    Initialization(String),

    #[error("Camera error: {0}")]
    Camera(#[from] CameraError),

    #[error("Printer error: {0}")]
    Printer(#[from] PrinterError),

    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Web error: {0}")]
    Web(String),
}

#[derive(Debug, Error)]
pub enum CameraError {
    #[error("Camera device not found: {device}")]
    DeviceNotFound { device: String },

    #[error("Failed to open camera device: {0}")]
    OpenFailed(String),

    #[error("Failed to set camera format: {0}")]
    FormatError(String),

    #[error("Failed to start camera stream: {0}")]
    StreamStartError(String),

    #[error("Failed to capture frame: {0}")]
    CaptureError(String),

    #[error("No frame available")]
    NoFrameAvailable,

    #[error("Camera I/O error: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum PrinterError {
    #[error("Printer not found: {name}")]
    NotFound { name: String },

    #[error("Printer not ready: {reason}")]
    NotReady { reason: String },

    #[error("Print job failed: {0}")]
    PrintFailed(String),

    #[error("Invalid print job configuration: {0}")]
    InvalidConfig(String),

    #[error("Printer I/O error: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Failed to load image: {0}")]
    ImageLoadError(String),

    #[error("Failed to save image: {0}")]
    ImageSaveError(String),

    #[error("Template composition error: {0}")]
    CompositionError(String),

    #[error("Background image not found: {path}")]
    BackgroundNotFound { path: String },

    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),

    #[error("Font loading error: {0}")]
    FontError(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid port number")]
    InvalidPort,

    #[error("Invalid video width")]
    InvalidVideoWidth,

    #[error("Invalid video height")]
    InvalidVideoHeight,

    #[error("Invalid video dimensions")]
    InvalidVideoDimensions,

    #[error("Unsupported video format: {format}")]
    UnsupportedVideoFormat { format: String },

    #[error("Invalid storage path: {path}")]
    InvalidStoragePath { path: String },

    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Failed to create directory: {path}")]
    CreateDirectoryFailed { path: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("Storage I/O error: {0}")]
    IoError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Database not found: {path}")]
    NotFound { path: String },

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::Initialization(_) => 500, // Internal Server Error
            AppError::Camera(_) => 503,         // Service Unavailable
            AppError::Printer(_) => 503,        // Service Unavailable
            AppError::Template(_) => 500,       // Internal Server Error
            AppError::Config(_) => 500,         // Internal Server Error
            AppError::Storage(StorageError::FileNotFound { .. }) => 404, // Not Found
            AppError::Storage(StorageError::PermissionDenied { .. }) => 403, // Forbidden
            AppError::Storage(_) => 500,        // Internal Server Error
            AppError::Database(_) => 503,       // Service Unavailable
            AppError::Web(_) => 500,            // Internal Server Error
        }
    }

    pub fn error_response(&self) -> serde_json::Value {
        serde_json::json!({
            "ok": false,
            "error": self.to_string(),
            "error_type": self.error_type()
        })
    }

    fn error_type(&self) -> &'static str {
        match self {
            AppError::Initialization(_) => "initialization_error",
            AppError::Camera(_) => "camera_error",
            AppError::Printer(_) => "printer_error",
            AppError::Template(_) => "template_error",
            AppError::Config(_) => "config_error",
            AppError::Storage(_) => "storage_error",
            AppError::Database(_) => "database_error",
            AppError::Web(_) => "web_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_error_display() {
        let err = CameraError::DeviceNotFound {
            device: "/dev/video0".to_string(),
        };
        assert_eq!(err.to_string(), "Camera device not found: /dev/video0");
    }

    #[test]
    fn test_app_error_status_codes() {
        let err = AppError::Storage(StorageError::FileNotFound {
            path: "test.png".to_string(),
        });
        assert_eq!(err.status_code(), 404);

        let err = AppError::Camera(CameraError::NoFrameAvailable);
        assert_eq!(err.status_code(), 503);
    }

    #[test]
    fn test_error_response_json() {
        let err = AppError::Printer(PrinterError::NotFound {
            name: "TestPrinter".to_string(),
        });
        let json = err.error_response();

        assert_eq!(json["ok"], false);
        assert_eq!(
            json["error"],
            "Printer error: Printer not found: TestPrinter"
        );
        assert_eq!(json["error_type"], "printer_error");
    }
}
