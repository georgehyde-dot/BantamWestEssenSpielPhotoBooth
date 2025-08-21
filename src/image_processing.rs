// Image processing module for photo booth
// Handles operations like removing autofocus boxes from captured images

#[cfg(target_os = "linux")]
use image::{DynamicImage, GenericImage, GenericImageView, Pixel, Rgb, RgbImage};

#[cfg(target_os = "linux")]
pub struct ImageProcessor;

#[cfg(target_os = "linux")]
impl ImageProcessor {
    /// Remove autofocus boxes from an image
    /// For testing: replaces detected white pixels with bright red
    pub fn remove_autofocus_boxes(image: &DynamicImage) -> DynamicImage {
        let mut img = image.to_rgb8();
        let (width, height) = img.dimensions();

        // First pass: Mark all white/bright pixels as red for debugging
        let white_pixels = Self::mark_white_pixels_red(&mut img);

        println!("Image dimensions: {}x{}", width, height);
        println!("Found {} white pixels", white_pixels);

        // Second pass: Try to detect rectangular patterns
        // This will help us understand the structure of the autofocus boxes
        Self::detect_and_mark_rectangles(&mut img);

        DynamicImage::ImageRgb8(img)
    }

    /// Mark all white/bright pixels as bright red for debugging
    fn mark_white_pixels_red(img: &mut RgbImage) -> usize {
        let (width, height) = img.dimensions();
        let mut white_pixel_count = 0;

        // Multiple thresholds to test
        const THRESHOLDS: [u8; 3] = [250, 240, 230];

        for threshold in THRESHOLDS.iter() {
            println!("Testing with threshold: {}", threshold);
            let mut count_at_threshold = 0;

            for y in 0..height {
                for x in 0..width {
                    let pixel = img.get_pixel(x, y);

                    // Check if all channels are above threshold (white/bright gray)
                    if pixel[0] >= *threshold && pixel[1] >= *threshold && pixel[2] >= *threshold {
                        // Mark as bright red for debugging
                        img.put_pixel(x, y, Rgb([255, 0, 0]));
                        white_pixel_count += 1;
                        count_at_threshold += 1;
                    }
                }
            }

            if count_at_threshold > 0 {
                println!(
                    "  Found {} pixels at threshold {}",
                    count_at_threshold, threshold
                );
            }
        }

        white_pixel_count
    }

    /// Detect rectangular patterns in the image
    /// This helps identify autofocus box structures
    fn detect_and_mark_rectangles(img: &mut RgbImage) {
        let (width, height) = img.dimensions();

        // Look for connected components of white pixels
        // Start with a simple edge detection approach
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let center = img.get_pixel(x, y);

                // If this pixel is already red (was white), check its pattern
                if center[0] == 255 && center[1] == 0 && center[2] == 0 {
                    // Check if this is part of a line structure
                    let is_horizontal_line = Self::is_part_of_horizontal_line(img, x, y);
                    let is_vertical_line = Self::is_part_of_vertical_line(img, x, y);

                    // Mark different types of structures with different colors for debugging
                    if is_horizontal_line && is_vertical_line {
                        // Corner or intersection - mark as yellow
                        img.put_pixel(x, y, Rgb([255, 255, 0]));
                    } else if is_horizontal_line {
                        // Horizontal line - mark as green
                        img.put_pixel(x, y, Rgb([0, 255, 0]));
                    } else if is_vertical_line {
                        // Vertical line - mark as blue
                        img.put_pixel(x, y, Rgb([0, 0, 255]));
                    }
                    // Otherwise leave as red
                }
            }
        }
    }

    /// Check if a pixel is part of a horizontal line of white/red pixels
    fn is_part_of_horizontal_line(img: &RgbImage, x: u32, y: u32) -> bool {
        let (width, _) = img.dimensions();
        let mut left_count = 0;
        let mut right_count = 0;

        // Check left
        for dx in 1..=5 {
            if x >= dx {
                let p = img.get_pixel(x - dx, y);
                if (p[0] == 255 && p[1] == 0 && p[2] == 0) || // Red (was white)
                   (p[0] > 240 && p[1] > 240 && p[2] > 240)
                {
                    // Still white
                    left_count += 1;
                } else {
                    break;
                }
            }
        }

        // Check right
        for dx in 1..=5 {
            if x + dx < width {
                let p = img.get_pixel(x + dx, y);
                if (p[0] == 255 && p[1] == 0 && p[2] == 0) || // Red (was white)
                   (p[0] > 240 && p[1] > 240 && p[2] > 240)
                {
                    // Still white
                    right_count += 1;
                } else {
                    break;
                }
            }
        }

        left_count + right_count >= 3
    }

    /// Check if a pixel is part of a vertical line of white/red pixels
    fn is_part_of_vertical_line(img: &RgbImage, x: u32, y: u32) -> bool {
        let (_, height) = img.dimensions();
        let mut up_count = 0;
        let mut down_count = 0;

        // Check up
        for dy in 1..=5 {
            if y >= dy {
                let p = img.get_pixel(x, y - dy);
                if (p[0] == 255 && p[1] == 0 && p[2] == 0) || // Red (was white)
                   (p[0] > 240 && p[1] > 240 && p[2] > 240)
                {
                    // Still white
                    up_count += 1;
                } else {
                    break;
                }
            }
        }

        // Check down
        for dy in 1..=5 {
            if y + dy < height {
                let p = img.get_pixel(x, y + dy);
                if (p[0] == 255 && p[1] == 0 && p[2] == 0) || // Red (was white)
                   (p[0] > 240 && p[1] > 240 && p[2] > 240)
                {
                    // Still white
                    down_count += 1;
                } else {
                    break;
                }
            }
        }

        up_count + down_count >= 3
    }

    /// Alternative approach: Simple threshold replacement
    /// This is a more aggressive approach that replaces any pixel above a certain brightness
    pub fn simple_white_removal(image: &DynamicImage, threshold: u8) -> DynamicImage {
        let mut img = image.to_rgb8();
        let (width, height) = img.dimensions();

        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y);

                // If any channel is above threshold, mark as red
                if pixel[0] > threshold || pixel[1] > threshold || pixel[2] > threshold {
                    img.put_pixel(x, y, Rgb([255, 0, 0]));
                }
            }
        }

        DynamicImage::ImageRgb8(img)
    }

    /// Another approach: Look for thin rectangular shapes specifically
    pub fn detect_thin_rectangles(image: &DynamicImage) -> DynamicImage {
        let mut img = image.to_rgb8();
        let (width, height) = img.dimensions();

        // Scan for potential box edges
        for y in 10..height - 10 {
            for x in 10..width - 10 {
                if Self::is_box_corner(&img, x, y) {
                    // Mark detected corners
                    for dy in 0..3 {
                        for dx in 0..3 {
                            img.put_pixel(x + dx, y + dy, Rgb([255, 0, 255])); // Magenta for corners
                        }
                    }
                }
            }
        }

        DynamicImage::ImageRgb8(img)
    }

    /// Check if a position might be a corner of an autofocus box
    fn is_box_corner(img: &RgbImage, x: u32, y: u32) -> bool {
        let pixel = img.get_pixel(x, y);

        // Must be white-ish
        if pixel[0] < 230 || pixel[1] < 230 || pixel[2] < 230 {
            return false;
        }

        // Check for L-shape pattern (corner)
        let has_horizontal = Self::has_white_line_horizontal(img, x, y, 10);
        let has_vertical = Self::has_white_line_vertical(img, x, y, 10);

        has_horizontal && has_vertical
    }

    /// Check for a horizontal white line from a point
    fn has_white_line_horizontal(img: &RgbImage, x: u32, y: u32, min_length: u32) -> bool {
        let (width, _) = img.dimensions();
        let mut white_count = 0;

        // Check to the right
        for dx in 0..min_length {
            if x + dx >= width {
                break;
            }
            let p = img.get_pixel(x + dx, y);
            if p[0] > 230 && p[1] > 230 && p[2] > 230 {
                white_count += 1;
            }
        }

        white_count >= min_length - 2
    }

    /// Check for a vertical white line from a point
    fn has_white_line_vertical(img: &RgbImage, x: u32, y: u32, min_length: u32) -> bool {
        let (_, height) = img.dimensions();
        let mut white_count = 0;

        // Check downward
        for dy in 0..min_length {
            if y + dy >= height {
                break;
            }
            let p = img.get_pixel(x, y + dy);
            if p[0] > 230 && p[1] > 230 && p[2] > 230 {
                white_count += 1;
            }
        }

        white_count >= min_length - 2
    }

    /// Brightness-difference based approach
    /// Detects pixels that are significantly brighter than their neighbors
    pub fn brightness_difference_removal(image: &DynamicImage) -> DynamicImage {
        let mut img = image.to_rgb8();
        let (width, height) = img.dimensions();

        // Clone for analysis to avoid modifying while reading
        let original = img.clone();

        println!("Using brightness difference detection...");
        let mut detected_count = 0;

        // Scan the image looking for pixels significantly brighter than surroundings
        for y in 2..height - 2 {
            for x in 2..width - 2 {
                let center_pixel = original.get_pixel(x, y);
                let center_brightness =
                    (center_pixel[0] as u32 + center_pixel[1] as u32 + center_pixel[2] as u32) / 3;

                // Skip if not bright enough to be a white line
                if center_brightness < 200 {
                    continue;
                }

                // Calculate average brightness of surrounding pixels
                let mut surrounding_sum = 0u32;
                let mut surrounding_count = 0u32;

                // Sample surrounding pixels (excluding immediate neighbors to avoid edge effects)
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        // Skip center and immediate neighbors
                        if dy.abs() <= 1 && dx.abs() <= 1 {
                            continue;
                        }

                        let px = (x as i32 + dx) as u32;
                        let py = (y as i32 + dy) as u32;

                        let p = original.get_pixel(px, py);
                        surrounding_sum += (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3;
                        surrounding_count += 1;
                    }
                }

                let avg_surrounding = surrounding_sum / surrounding_count;

                // If this pixel is significantly brighter than surroundings, it's likely part of an autofocus box
                if center_brightness > avg_surrounding + 50 {
                    // Mark as bright red for testing
                    img.put_pixel(x, y, Rgb([255, 0, 0]));
                    detected_count += 1;
                }
            }
        }

        println!(
            "Detected {} bright pixels using difference method",
            detected_count
        );
        DynamicImage::ImageRgb8(img)
    }
}

// Non-Linux stubs
#[cfg(not(target_os = "linux"))]
pub struct ImageProcessor;

#[cfg(not(target_os = "linux"))]
impl ImageProcessor {
    pub fn remove_autofocus_boxes(image: &image::DynamicImage) -> image::DynamicImage {
        image.clone()
    }

    pub fn simple_white_removal(
        image: &image::DynamicImage,
        _threshold: u8,
    ) -> image::DynamicImage {
        image.clone()
    }

    pub fn detect_thin_rectangles(image: &image::DynamicImage) -> image::DynamicImage {
        image.clone()
    }

    pub fn brightness_difference_removal(image: &image::DynamicImage) -> image::DynamicImage {
        image.clone()
    }
}
