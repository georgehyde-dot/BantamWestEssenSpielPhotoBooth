// Template functionality for photo booth prints

use image::{DynamicImage, ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::error::Error;
use std::fmt;

// Constants for a 4x6" print at 300 DPI
const PRINT_WIDTH: u32 = 1200; // 4 inches * 300 DPI
const PRINT_HEIGHT: u32 = 1800; // 6 inches * 300 DPI

// Define the area for the photo within the template
const PHOTO_WIDTH: u32 = 1000; // Leave room for borders
const PHOTO_HEIGHT: u32 = 667; // Maintain 3:2 aspect ratio
const PHOTO_Y_POSITION: u32 = 400; // Position for photo

#[derive(Debug)]
pub enum TemplateError {
    ImageLoadError(String),
    ImageSaveError(String),
    CompositionError(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::ImageLoadError(msg) => write!(f, "Failed to load image: {}", msg),
            TemplateError::ImageSaveError(msg) => write!(f, "Failed to save image: {}", msg),
            TemplateError::CompositionError(msg) => write!(f, "Composition error: {}", msg),
        }
    }
}

impl Error for TemplateError {}

pub struct PrintTemplate {
    story_text: String,
    background_color: Rgb<u8>,
    text_color: Rgb<u8>,
    background_path: Option<String>,
}

impl Default for PrintTemplate {
    fn default() -> Self {
        PrintTemplate {
            story_text: "STORY HERE".to_string(),
            background_color: Rgb([255, 255, 255]), // White background
            text_color: Rgb([50, 50, 50]),          // Dark gray text
            background_path: None,
        }
    }
}

impl PrintTemplate {
    pub fn new(story: &str) -> Self {
        PrintTemplate {
            story_text: story.to_string(),
            ..Default::default()
        }
    }

    pub fn with_background(mut self, path: &str) -> Self {
        self.background_path = Some(path.to_string());
        self
    }

    pub fn apply_to_photo(&self, photo_path: &str, output_path: &str) -> Result<(), TemplateError> {
        if !std::path::Path::new(photo_path).exists() {
            return Err(TemplateError::ImageLoadError(format!(
                "Photo file does not exist: {}",
                photo_path
            )));
        }

        let photo =
            image::open(photo_path).map_err(|e| TemplateError::ImageLoadError(e.to_string()))?;

        let templated = self.compose_template(photo)?;

        templated
            .save(output_path)
            .map_err(|e| TemplateError::ImageSaveError(e.to_string()))?;

        Ok(())
    }

    fn compose_template(&self, photo: DynamicImage) -> Result<RgbImage, TemplateError> {
        // 1. Load the background image
        let mut canvas = if let Some(bg_path) = &self.background_path {
            match image::open(bg_path) {
                Ok(bg) => {
                    // Resize background to match print dimensions
                    image::imageops::resize(
                        &bg.to_rgb8(),
                        PRINT_WIDTH,
                        PRINT_HEIGHT,
                        image::imageops::FilterType::Lanczos3,
                    )
                }
                Err(_) => {
                    // Fallback to solid color if background can't be loaded
                    ImageBuffer::from_pixel(PRINT_WIDTH, PRINT_HEIGHT, self.background_color)
                }
            }
        } else {
            // Fallback to solid color if background doesn't exist
            ImageBuffer::from_pixel(PRINT_WIDTH, PRINT_HEIGHT, self.background_color)
        };

        // 2. Scale the photo to fit its designated area
        let scaled_photo = self.scale_photo_to_fit(photo)?;

        // 3. Place the scaled photo onto the canvas
        self.place_photo(&mut canvas, &scaled_photo);

        // 4. Add story text
        self.add_story_text(&mut canvas)?;

        Ok(canvas)
    }

    fn scale_photo_to_fit(&self, photo: DynamicImage) -> Result<RgbImage, TemplateError> {
        let photo_rgb = photo.to_rgb8();
        if photo_rgb.width() == 0 || photo_rgb.height() == 0 {
            return Err(TemplateError::CompositionError(
                "Invalid photo dimensions".to_string(),
            ));
        }
        let scaled = image::imageops::resize(
            &photo_rgb,
            PHOTO_WIDTH,
            PHOTO_HEIGHT,
            image::imageops::FilterType::Lanczos3,
        );
        Ok(scaled)
    }

    fn place_photo(&self, canvas: &mut RgbImage, photo: &RgbImage) {
        let photo_x = (PRINT_WIDTH - photo.width()) / 2;
        let photo_y = PHOTO_Y_POSITION;
        image::imageops::overlay(canvas, photo, photo_x as i64, photo_y as i64);
    }

    fn add_story_text(&self, canvas: &mut RgbImage) -> Result<(), TemplateError> {
        let font_data = match std::fs::read("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf") {
            Ok(data) => data,
            Err(_) => {
                println!("Warning: Font not found. Skipping text.");
                return Ok(());
            }
        };
        let font = match Font::try_from_bytes(&font_data) {
            Some(f) => f,
            None => {
                println!("Warning: Failed to parse font. Skipping text.");
                return Ok(());
            }
        };

        let story_scale = Scale { x: 40.0, y: 40.0 };

        // Story text positioned below the photo
        let story_lines = self.wrap_text(&font, &self.story_text, story_scale, PRINT_WIDTH - 100);
        let line_height = 35; // Space between lines

        // Position story text below the photo with some padding
        let story_start_y = PHOTO_Y_POSITION + PHOTO_HEIGHT + 150;

        for (i, line) in story_lines.iter().enumerate() {
            let line_width = self.measure_text_width(&font, line, story_scale);
            let line_y = story_start_y + (i as u32 * line_height);

            // Make sure we don't draw below the bottom of the canvas
            if line_y < PRINT_HEIGHT - 200 {
                draw_text_mut(
                    canvas,
                    self.text_color,
                    ((PRINT_WIDTH - line_width) / 2) as i32,
                    line_y as i32,
                    story_scale,
                    &font,
                    line,
                );
            }
        }

        Ok(())
    }

    fn measure_text_width(&self, font: &Font, text: &str, scale: Scale) -> u32 {
        font.layout(text, scale, rusttype::point(0.0, 0.0))
            .last()
            .and_then(|g| g.pixel_bounding_box())
            .map_or(0, |bb| bb.max.x as u32)
    }

    fn wrap_text(&self, font: &Font, text: &str, scale: Scale, max_width: u32) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let width = self.measure_text_width(font, &test_line, scale);

            if width > max_width && !current_line.is_empty() {
                // Current line is full, start a new one
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                // Add word to current line
                current_line = test_line;
            }
        }

        // Don't forget the last line
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}

pub fn create_templated_print_with_background(
    photo_path: &str,
    output_path: &str,
    story: &str,
    background_path: &str,
    _header_path: &str,
    _footer_path: &str,
    _break_path: &str,
) -> Result<(), TemplateError> {
    // Ignoring header, footer, and break paths - just use background
    let template = PrintTemplate::new(story).with_background(background_path);
    template.apply_to_photo(photo_path, output_path)
}
