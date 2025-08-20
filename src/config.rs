use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub camera: CameraConfig,
    pub storage: StorageConfig,
    pub printer: PrinterConfig,
    pub template: TemplateConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CameraConfig {
    pub device: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub base_path: PathBuf,
    pub static_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrinterConfig {
    pub name: String,
    pub fallback_names: Vec<String>,
    pub use_mock: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateConfig {
    pub header_text: String,
    pub name_placeholder: String,
    pub headline_placeholder: String,
    pub story_placeholder: String,
    pub background_filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!("sqlite://{}", self.path.display())
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let server = ServerConfig {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidPort)?,
        };

        let camera = CameraConfig {
            device: std::env::var("VIDEO_DEVICE").unwrap_or_else(|_| "/dev/video0".to_string()),
            width: std::env::var("VIDEO_WIDTH")
                .unwrap_or_else(|_| "1920".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidVideoWidth)?,
            height: std::env::var("VIDEO_HEIGHT")
                .unwrap_or_else(|_| "1080".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidVideoHeight)?,
            format: std::env::var("VIDEO_FORMAT").unwrap_or_else(|_| "MJPG".to_string()),
        };

        let base_path = std::env::var("STORAGE_PATH")
            .unwrap_or_else(|_| "/usr/local/share/photo_booth".to_string());
        let storage = StorageConfig {
            base_path: PathBuf::from(&base_path),
            static_path: PathBuf::from(&base_path).join("static"),
        };

        let printer = PrinterConfig {
            name: std::env::var("PRINTER_NAME")
                .unwrap_or_else(|_| "XP8700series-TurboPrint".to_string()),
            fallback_names: std::env::var("PRINTER_FALLBACK")
                .unwrap_or_else(|_| "EPSON_XP_8700_Series_USB,XP-8700".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            use_mock: std::env::var("USE_MOCK_PRINTER")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        };

        let template = TemplateConfig {
            header_text: std::env::var("TEMPLATE_HEADER")
                .unwrap_or_else(|_| "Photo Booth".to_string()),
            name_placeholder: std::env::var("TEMPLATE_NAME")
                .unwrap_or_else(|_| "NAME HERE".to_string()),
            headline_placeholder: std::env::var("TEMPLATE_HEADLINE")
                .unwrap_or_else(|_| "HEADLINE".to_string()),
            story_placeholder: std::env::var("TEMPLATE_STORY")
                .unwrap_or_else(|_| "STORY HERE".to_string()),
            background_filename: std::env::var("TEMPLATE_BACKGROUND")
                .unwrap_or_else(|_| "background.png".to_string()),
        };

        let database = DatabaseConfig {
            path: std::env::var("DATABASE_PATH")
                .unwrap_or_else(|_| format!("{}/photo_booth.db", base_path))
                .into(),
        };

        let config = Config {
            server,
            camera,
            storage,
            printer,
            template,
            database,
        };

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Validate port range
        if self.server.port == 0 {
            return Err(ConfigError::InvalidPort);
        }

        // Validate video dimensions
        if self.camera.width == 0 || self.camera.height == 0 {
            return Err(ConfigError::InvalidVideoDimensions);
        }

        // Validate video format
        if !["MJPG", "YUYV"].contains(&self.camera.format.as_str()) {
            return Err(ConfigError::UnsupportedVideoFormat);
        }

        Ok(())
    }

    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.server.host, self.server.port)
            .parse()
            .expect("Invalid socket address")
    }

    pub fn images_path(&self) -> PathBuf {
        self.storage.base_path.clone()
    }

    pub fn background_path(&self) -> PathBuf {
        self.storage
            .static_path
            .join(&self.template.background_filename)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid port number")]
    InvalidPort,
    #[error("Invalid video width")]
    InvalidVideoWidth,
    #[error("Invalid video height")]
    InvalidVideoHeight,
    #[error("Invalid video dimensions")]
    InvalidVideoDimensions,
    #[error("Unsupported video format")]
    UnsupportedVideoFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // Clear any existing env vars
        std::env::remove_var("PORT");
        std::env::remove_var("VIDEO_WIDTH");

        let config = Config::from_env().expect("Failed to create config");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.camera.width, 1920);
        assert_eq!(config.camera.device, "/dev/video0");
    }

    #[test]
    fn test_invalid_port() {
        std::env::set_var("PORT", "invalid");
        let result = Config::from_env();
        assert!(matches!(result, Err(ConfigError::InvalidPort)));
        std::env::remove_var("PORT");
    }
}
