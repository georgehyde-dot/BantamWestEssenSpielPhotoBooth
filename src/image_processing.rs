// Image processing module for photo booth
// Handles removal of autofocus boxes from captured images

#[cfg(target_os = "linux")]
use image::{DynamicImage, Rgb, RgbImage};
#[cfg(target_os = "linux")]
use std::collections::HashSet;

#[cfg(target_os = "linux")]
pub struct ImageProcessor;

#[cfg(target_os = "linux")]
impl ImageProcessor {
    /// Remove autofocus boxes from an image using advanced multi-pass inpainting
    pub fn remove_autofocus_boxes(image: &DynamicImage) -> DynamicImage {
        let mut img = image.to_rgb8();

        // Phase 1: Aggressive detection of bright pixels
        let box_pixels = Self::detect_autofocus_box(&img);

        if box_pixels.is_empty() {
            return DynamicImage::ImageRgb8(img);
        }

        println!("Detected {} autofocus box pixels", box_pixels.len());

        // Phase 2: Multi-pass inpainting with different strategies
        Self::multi_pass_inpaint(&mut img, &box_pixels);

        DynamicImage::ImageRgb8(img)
    }

    /// Detect autofocus box pixels with aggressive expansion
    fn detect_autofocus_box(img: &RgbImage) -> Vec<(u32, u32)> {
        let (width, height) = img.dimensions();
        let mut detected_pixels = HashSet::new();

        // Limit search area to bottom left corner where autofocus box appears
        // Check bottom 40% and left 30% of the image (expanded to catch top line)
        let max_x = (width as f32 * 0.3) as u32;
        let min_y = (height as f32 * 0.6) as u32;

        let search_area = (max_x * (height - min_y)) as f32;
        let total_area = (width * height) as f32;
        let area_reduction = ((1.0 - search_area / total_area) * 100.0) as u32;

        println!(
            "Searching bottom-left corner: {}x{} pixels ({}% area reduction)",
            max_x,
            height - min_y,
            area_reduction
        );

        // Pass 1: Find bright pixels (main box lines)
        for y in min_y..height {
            for x in 0..max_x {
                let pixel = img.get_pixel(x, y);
                // Lower threshold to catch more of the box
                if pixel[0] > 235 && pixel[1] > 235 && pixel[2] > 235 {
                    detected_pixels.insert((x, y));
                }
            }
        }

        // Pass 2: Find high-contrast edges that might be part of the box
        let mut edge_pixels = HashSet::new();
        for y in min_y.max(1)..height - 1 {
            for x in 1..max_x.min(width - 1) {
                if Self::is_high_contrast_edge(img, x, y) {
                    edge_pixels.insert((x, y));
                }
            }
        }

        // Combine edge pixels that are near detected bright pixels
        for &(x, y) in &edge_pixels {
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let nx = (x as i32 + dx) as u32;
                    let ny = (y as i32 + dy) as u32;
                    if detected_pixels.contains(&(nx, ny)) {
                        detected_pixels.insert((x, y));
                        break;
                    }
                }
            }
        }

        // Pass 3: Aggressive expansion to catch anti-aliasing and glow
        let original_pixels: Vec<(u32, u32)> = detected_pixels.iter().cloned().collect();
        for &(x, y) in &original_pixels {
            // Expand in a 5x5 area around each detected pixel
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    // Keep expansion within the search area bounds
                    if nx >= 0
                        && ny >= 0
                        && nx < max_x as i32
                        && ny >= min_y as i32
                        && nx < width as i32
                        && ny < height as i32
                    {
                        let nx = nx as u32;
                        let ny = ny as u32;
                        let pixel = img.get_pixel(nx, ny);
                        // Include any pixel that's notably brighter than expected
                        if pixel[0] > 200 || pixel[1] > 200 || pixel[2] > 200 {
                            detected_pixels.insert((nx, ny));
                        }
                    }
                }
            }
        }

        // Pass 4: Directional expansion to catch dark edges perpendicular to lines
        let detected_vec: Vec<(u32, u32)> = detected_pixels.iter().cloned().collect();
        for &(x, y) in &detected_vec {
            // Check if this pixel is part of a horizontal line
            let is_horizontal = (x > 0 && detected_pixels.contains(&(x - 1, y)))
                || (x < width - 1 && detected_pixels.contains(&(x + 1, y)));

            // Check if this pixel is part of a vertical line
            let is_vertical = (y > 0 && detected_pixels.contains(&(x, y - 1)))
                || (y < height - 1 && detected_pixels.contains(&(x, y + 1)));

            if is_horizontal {
                // Expand up and down for horizontal lines (including dark edges)
                // Increased range to capture full box top line
                for dy in -8i32..=8 {
                    let ny = y as i32 + dy;
                    // Allow more aggressive expansion above search area to capture top lines
                    if ny >= 0 && ny < height as i32 {
                        detected_pixels.insert((x, ny as u32));
                    }
                }
            }

            if is_vertical {
                // Expand left and right for vertical lines (including dark edges)
                for dx in -4i32..=4 {
                    let nx = x as i32 + dx;
                    // Allow expansion slightly outside search area for better edge handling
                    if nx >= 0 && nx < (max_x as i32 + 5).min(width as i32) {
                        detected_pixels.insert((nx as u32, y));
                    }
                }
            }
        }

        // Pass 5: Edge completion - ensure top edges of detected regions are fully captured
        let current_pixels: Vec<(u32, u32)> = detected_pixels.iter().cloned().collect();
        for &(x, y) in &current_pixels {
            // For each detected pixel, check if there are bright pixels above it
            for dy in 1..=20 {
                let ny = y as i32 - dy;
                if ny >= 0 {
                    let ny = ny as u32;
                    let pixel = img.get_pixel(x, ny);
                    // If we find a bright pixel above, include it and all pixels in between
                    if pixel[0] > 200 || pixel[1] > 200 || pixel[2] > 200 {
                        for fill_y in ny..=y {
                            detected_pixels.insert((x, fill_y));
                        }
                        break;
                    }
                }
            }
        }

        // Pass 6: Connected component filling - find and fill corners
        let detected_vec: Vec<(u32, u32)> = detected_pixels.iter().cloned().collect();
        for &(x, y) in &detected_vec {
            // Check for corner patterns (L-shaped regions)
            // Check if we have horizontal and vertical components meeting
            let has_horizontal = detected_pixels.contains(&(x.saturating_sub(1), y))
                || detected_pixels.contains(&(x + 1, y));
            let has_vertical = detected_pixels.contains(&(x, y.saturating_sub(1)))
                || detected_pixels.contains(&(x, y + 1));

            if has_horizontal && has_vertical {
                // This might be a corner - fill in a larger area
                for dy in -6i32..=6 {
                    for dx in -6i32..=6 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                            let nx = nx as u32;
                            let ny = ny as u32;
                            let pixel = img.get_pixel(nx, ny);
                            // Include any reasonably bright pixel near corners
                            if pixel[0] > 180 || pixel[1] > 180 || pixel[2] > 180 {
                                detected_pixels.insert((nx, ny));
                            }
                        }
                    }
                }
            }
        }

        detected_pixels.into_iter().collect()
    }

    /// Check if a pixel is part of a high-contrast edge
    fn is_high_contrast_edge(img: &RgbImage, x: u32, y: u32) -> bool {
        let center = img.get_pixel(x, y);
        let center_lum = (center[0] as u32 + center[1] as u32 + center[2] as u32) / 3;

        // Check surrounding pixels for significant contrast
        let mut max_diff = 0u32;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x as i32 + dx) as u32;
                let ny = (y as i32 + dy) as u32;
                let neighbor = img.get_pixel(nx, ny);
                let neighbor_lum =
                    (neighbor[0] as u32 + neighbor[1] as u32 + neighbor[2] as u32) / 3;

                let diff = if center_lum > neighbor_lum {
                    center_lum - neighbor_lum
                } else {
                    neighbor_lum - center_lum
                };

                if diff > max_diff {
                    max_diff = diff;
                }
            }
        }

        // High contrast threshold
        max_diff > 50 && center_lum > 180
    }

    /// Multi-pass inpainting with different strategies
    fn multi_pass_inpaint(img: &mut RgbImage, box_pixels: &Vec<(u32, u32)>) {
        let box_set: HashSet<(u32, u32)> = box_pixels.iter().cloned().collect();

        // Create an expanded mask that includes a border around the box
        // Increased vertical expansion to better handle top edges
        let mut mask = HashSet::new();
        for &(x, y) in box_pixels {
            for dy in -10i32..=10 {
                for dx in -7i32..=7 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < img.width() as i32 && ny < img.height() as i32 {
                        mask.insert((nx as u32, ny as u32));
                    }
                }
            }
        }

        // Pass 1: Initial fill from far neighbors
        let original = img.clone();
        for &(x, y) in box_pixels {
            if let Some(color) = Self::sample_far_neighbors(&original, x, y, &mask) {
                img.put_pixel(x, y, color);
            }
        }

        // Pass 2: Smooth using bilateral-like filtering
        let intermediate = img.clone();
        for &(x, y) in box_pixels {
            if let Some(color) = Self::bilateral_sample(&intermediate, x, y, &box_set) {
                img.put_pixel(x, y, color);
            }
        }

        // Pass 3: Final cleanup - remove any remaining bright spots
        for &(x, y) in box_pixels {
            let pixel = img.get_pixel(x, y);
            if pixel[0] > 220 || pixel[1] > 220 || pixel[2] > 220 {
                if let Some(color) = Self::aggressive_cleanup(&intermediate, x, y) {
                    img.put_pixel(x, y, color);
                }
            }
        }
    }

    /// Sample from far neighbors, avoiding the masked area
    fn sample_far_neighbors(
        img: &RgbImage,
        x: u32,
        y: u32,
        mask: &HashSet<(u32, u32)>,
    ) -> Option<Rgb<u8>> {
        let mut samples = Vec::new();
        let (width, height) = img.dimensions();

        // Sample in rays from the center point
        let directions = [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ];

        for &(dx, dy) in &directions {
            // Start sampling from distance 12 to avoid the box area and dark edges
            for dist in 12..25 {
                let nx = x as f32 + dx * dist as f32;
                let ny = y as f32 + dy * dist as f32;

                if nx >= 0.0 && ny >= 0.0 && nx < width as f32 && ny < height as f32 {
                    let nx = nx as u32;
                    let ny = ny as u32;

                    if !mask.contains(&(nx, ny)) {
                        let pixel = img.get_pixel(nx, ny);
                        // Only use pixels that aren't too bright
                        if pixel[0] < 200 && pixel[1] < 200 && pixel[2] < 200 {
                            samples.push(*pixel);
                            break; // Found a good sample in this direction
                        }
                    }
                }
            }
        }

        if samples.len() >= 4 {
            Some(Self::median_pixel(&samples))
        } else if !samples.is_empty() {
            Some(Self::average_pixels(&samples))
        } else {
            None
        }
    }

    /// Bilateral-like sampling that considers color similarity
    fn bilateral_sample(
        img: &RgbImage,
        x: u32,
        y: u32,
        box_set: &HashSet<(u32, u32)>,
    ) -> Option<Rgb<u8>> {
        let (width, height) = img.dimensions();
        let center_pixel = img.get_pixel(x, y);

        let mut weighted_sum = [0.0f64; 3];
        let mut total_weight = 0.0f64;

        // Sample in a wider radius
        for dy in -15i32..=15 {
            for dx in -15i32..=15 {
                if dx.abs() < 5 && dy.abs() < 5 {
                    continue;
                } // Skip nearby pixels

                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                    let nx = nx as u32;
                    let ny = ny as u32;

                    if !box_set.contains(&(nx, ny)) {
                        let sample = img.get_pixel(nx, ny);

                        // Skip bright pixels
                        if sample[0] > 210 || sample[1] > 210 || sample[2] > 210 {
                            continue;
                        }

                        // Distance weight
                        let dist = ((dx * dx + dy * dy) as f64).sqrt();
                        let spatial_weight = (-dist / 10.0).exp();

                        // Color similarity weight
                        let color_dist = Self::color_distance(center_pixel, sample);
                        let color_weight = (-color_dist / 50.0).exp();

                        let weight = spatial_weight * color_weight;

                        weighted_sum[0] += sample[0] as f64 * weight;
                        weighted_sum[1] += sample[1] as f64 * weight;
                        weighted_sum[2] += sample[2] as f64 * weight;
                        total_weight += weight;
                    }
                }
            }
        }

        if total_weight > 0.0 {
            Some(Rgb([
                (weighted_sum[0] / total_weight).round().min(255.0) as u8,
                (weighted_sum[1] / total_weight).round().min(255.0) as u8,
                (weighted_sum[2] / total_weight).round().min(255.0) as u8,
            ]))
        } else {
            None
        }
    }

    /// Aggressive cleanup for stubborn bright pixels
    fn aggressive_cleanup(img: &RgbImage, x: u32, y: u32) -> Option<Rgb<u8>> {
        let (width, height) = img.dimensions();
        let mut samples = Vec::new();

        // Large radius sampling, only dark pixels
        for dy in -20i32..=20 {
            for dx in -20i32..=20 {
                if dx.abs() < 8 && dy.abs() < 8 {
                    continue;
                }

                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                    let pixel = img.get_pixel(nx as u32, ny as u32);
                    // Very strict: only use clearly non-box pixels
                    if pixel[0] < 180 && pixel[1] < 180 && pixel[2] < 180 {
                        samples.push(*pixel);
                    }
                }
            }
        }

        if samples.len() >= 10 {
            // Use the darker percentile to ensure we don't keep any brightness
            let percentile_idx = samples.len() * 25 / 100; // 25th percentile
            let mut sorted_by_brightness: Vec<_> = samples
                .iter()
                .map(|p| (p, p[0] as u32 + p[1] as u32 + p[2] as u32))
                .collect();
            sorted_by_brightness.sort_by_key(|&(_, brightness)| brightness);

            Some(*sorted_by_brightness[percentile_idx].0)
        } else if !samples.is_empty() {
            Some(Self::median_pixel(&samples))
        } else {
            None
        }
    }

    /// Calculate color distance between two pixels
    fn color_distance(p1: &Rgb<u8>, p2: &Rgb<u8>) -> f64 {
        let dr = p1[0] as f64 - p2[0] as f64;
        let dg = p1[1] as f64 - p2[1] as f64;
        let db = p1[2] as f64 - p2[2] as f64;
        (dr * dr + dg * dg + db * db).sqrt()
    }

    /// Calculate average of pixels
    fn average_pixels(pixels: &[Rgb<u8>]) -> Rgb<u8> {
        if pixels.is_empty() {
            return Rgb([128, 128, 128]);
        }

        let mut r_sum = 0u32;
        let mut g_sum = 0u32;
        let mut b_sum = 0u32;

        for pixel in pixels {
            r_sum += pixel[0] as u32;
            g_sum += pixel[1] as u32;
            b_sum += pixel[2] as u32;
        }

        let count = pixels.len() as u32;
        Rgb([
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
        ])
    }

    /// Calculate median pixel value
    fn median_pixel(pixels: &[Rgb<u8>]) -> Rgb<u8> {
        if pixels.is_empty() {
            return Rgb([128, 128, 128]);
        }

        let mut r_values: Vec<u8> = pixels.iter().map(|p| p[0]).collect();
        let mut g_values: Vec<u8> = pixels.iter().map(|p| p[1]).collect();
        let mut b_values: Vec<u8> = pixels.iter().map(|p| p[2]).collect();

        r_values.sort_unstable();
        g_values.sort_unstable();
        b_values.sort_unstable();

        let mid = pixels.len() / 2;
        Rgb([r_values[mid], g_values[mid], b_values[mid]])
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
}
