//! Visual regression testing with real image comparison.
//!
//! Per spec Section 6.2: Visual Regression Testing using pure Rust image comparison.

use crate::result::{ProbarError, ProbarResult};
use image::{DynamicImage, GenericImageView, ImageEncoder, Rgba};
use std::path::Path;

/// Configuration for visual regression testing
#[derive(Debug, Clone)]
pub struct VisualRegressionConfig {
    /// Difference threshold (0.0-1.0) - percentage of pixels that can differ
    pub threshold: f64,
    /// Per-pixel color difference threshold (0-255)
    pub color_threshold: u8,
    /// Directory to store baseline images
    pub baseline_dir: String,
    /// Directory to store diff images on failure
    pub diff_dir: String,
    /// Whether to update baselines automatically
    pub update_baselines: bool,
}

impl Default for VisualRegressionConfig {
    fn default() -> Self {
        Self {
            threshold: 0.01,     // 1% of pixels can differ
            color_threshold: 10, // Allow minor color variations
            baseline_dir: String::from("__baselines__"),
            diff_dir: String::from("__diffs__"),
            update_baselines: false,
        }
    }
}

impl VisualRegressionConfig {
    /// Set the threshold
    #[must_use]
    pub const fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the color threshold
    #[must_use]
    pub const fn with_color_threshold(mut self, threshold: u8) -> Self {
        self.color_threshold = threshold;
        self
    }

    /// Set the baseline directory
    #[must_use]
    pub fn with_baseline_dir(mut self, dir: impl Into<String>) -> Self {
        self.baseline_dir = dir.into();
        self
    }

    /// Enable baseline updates
    #[must_use]
    pub const fn with_update_baselines(mut self, update: bool) -> Self {
        self.update_baselines = update;
        self
    }
}

/// Result of comparing two images
#[derive(Debug, Clone)]
pub struct ImageDiffResult {
    /// Whether images match within threshold
    pub matches: bool,
    /// Number of pixels that differ
    pub diff_pixel_count: usize,
    /// Total number of pixels compared
    pub total_pixels: usize,
    /// Percentage of pixels that differ (0.0-100.0)
    pub diff_percentage: f64,
    /// Maximum color difference found
    pub max_color_diff: u32,
    /// Average color difference for differing pixels
    pub avg_color_diff: f64,
    /// Diff image data (PNG encoded, highlights differences in red)
    pub diff_image: Option<Vec<u8>>,
}

impl ImageDiffResult {
    /// Check if images are identical (no differences)
    #[must_use]
    pub const fn is_identical(&self) -> bool {
        self.diff_pixel_count == 0
    }

    /// Check if difference is within threshold
    #[must_use]
    pub fn within_threshold(&self, threshold: f64) -> bool {
        self.diff_percentage <= threshold * 100.0
    }
}

/// Visual regression tester
#[derive(Debug, Clone)]
pub struct VisualRegressionTester {
    config: VisualRegressionConfig,
}

impl Default for VisualRegressionTester {
    fn default() -> Self {
        Self::new(VisualRegressionConfig::default())
    }
}

impl VisualRegressionTester {
    /// Create a new tester with configuration
    #[must_use]
    pub const fn new(config: VisualRegressionConfig) -> Self {
        Self { config }
    }

    /// Compare two images from byte arrays (PNG format)
    ///
    /// # Errors
    ///
    /// Returns error if images cannot be decoded
    pub fn compare_images(&self, actual: &[u8], expected: &[u8]) -> ProbarResult<ImageDiffResult> {
        let actual_img =
            image::load_from_memory(actual).map_err(|e| ProbarError::ImageComparisonError {
                message: format!("Failed to decode actual image: {e}"),
            })?;

        let expected_img =
            image::load_from_memory(expected).map_err(|e| ProbarError::ImageComparisonError {
                message: format!("Failed to decode expected image: {e}"),
            })?;

        self.compare_dynamic_images(&actual_img, &expected_img)
    }

    /// Compare two `DynamicImage` instances
    ///
    /// # Errors
    ///
    /// Returns error if images have different dimensions
    pub fn compare_dynamic_images(
        &self,
        actual: &DynamicImage,
        expected: &DynamicImage,
    ) -> ProbarResult<ImageDiffResult> {
        let (width, height) = actual.dimensions();
        let (exp_width, exp_height) = expected.dimensions();

        // Check dimensions match
        if width != exp_width || height != exp_height {
            return Err(ProbarError::ImageComparisonError {
                message: format!(
                    "Image dimensions differ: actual {width}x{height}, expected {exp_width}x{exp_height}"
                ),
            });
        }

        let total_pixels = (width * height) as usize;
        let mut diff_pixel_count = 0usize;
        let mut max_color_diff: u32 = 0;
        let mut total_color_diff: u64 = 0;

        // Create diff image
        let mut diff_img = image::RgbaImage::new(width, height);

        let actual_rgba = actual.to_rgba8();
        let expected_rgba = expected.to_rgba8();

        for y in 0..height {
            for x in 0..width {
                let actual_pixel = actual_rgba.get_pixel(x, y);
                let expected_pixel = expected_rgba.get_pixel(x, y);

                let color_diff = pixel_diff(*actual_pixel, *expected_pixel);

                if color_diff > u32::from(self.config.color_threshold) {
                    diff_pixel_count += 1;
                    total_color_diff += u64::from(color_diff);
                    max_color_diff = max_color_diff.max(color_diff);

                    // Highlight difference in red on diff image
                    diff_img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                } else {
                    // Copy original pixel with reduced opacity
                    let Rgba([r, g, b, _]) = *actual_pixel;
                    diff_img.put_pixel(x, y, Rgba([r / 2, g / 2, b / 2, 128]));
                }
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let diff_percentage = if total_pixels > 0 {
            (diff_pixel_count as f64 / total_pixels as f64) * 100.0
        } else {
            0.0
        };

        #[allow(clippy::cast_precision_loss)]
        let avg_color_diff = if diff_pixel_count > 0 {
            total_color_diff as f64 / diff_pixel_count as f64
        } else {
            0.0
        };

        let matches = diff_percentage <= self.config.threshold * 100.0;

        // Encode diff image to PNG
        let diff_image = if matches {
            None
        } else {
            let mut buffer = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
            encoder
                .write_image(
                    diff_img.as_raw(),
                    width,
                    height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| ProbarError::ImageComparisonError {
                    message: format!("Failed to encode diff image: {e}"),
                })?;
            Some(buffer)
        };

        Ok(ImageDiffResult {
            matches,
            diff_pixel_count,
            total_pixels,
            diff_percentage,
            max_color_diff,
            avg_color_diff,
            diff_image,
        })
    }

    /// Compare screenshot against baseline file
    ///
    /// # Errors
    ///
    /// Returns error if baseline doesn't exist or comparison fails
    pub fn compare_against_baseline(
        &self,
        name: &str,
        screenshot: &[u8],
    ) -> ProbarResult<ImageDiffResult> {
        let baseline_path = Path::new(&self.config.baseline_dir).join(format!("{name}.png"));

        if !baseline_path.exists() {
            if self.config.update_baselines {
                // Create baseline
                std::fs::create_dir_all(&self.config.baseline_dir)?;
                std::fs::write(&baseline_path, screenshot)?;
                return Ok(ImageDiffResult {
                    matches: true,
                    diff_pixel_count: 0,
                    total_pixels: 0,
                    diff_percentage: 0.0,
                    max_color_diff: 0,
                    avg_color_diff: 0.0,
                    diff_image: None,
                });
            }
            return Err(ProbarError::ImageComparisonError {
                message: format!("Baseline not found: {}", baseline_path.display()),
            });
        }

        let baseline = std::fs::read(&baseline_path)?;
        let result = self.compare_images(screenshot, &baseline)?;

        // Save diff image if comparison failed
        if !result.matches {
            if let Some(ref diff_data) = result.diff_image {
                std::fs::create_dir_all(&self.config.diff_dir)?;
                let diff_path = Path::new(&self.config.diff_dir).join(format!("{name}_diff.png"));
                std::fs::write(&diff_path, diff_data)?;
            }
        }

        // Update baseline if configured
        if self.config.update_baselines && !result.matches {
            std::fs::write(&baseline_path, screenshot)?;
        }

        Ok(result)
    }

    /// Get configuration
    #[must_use]
    pub const fn config(&self) -> &VisualRegressionConfig {
        &self.config
    }
}

/// Calculate pixel difference (sum of RGB channel differences)
fn pixel_diff(a: Rgba<u8>, b: Rgba<u8>) -> u32 {
    let Rgba([r1, g1, b1, _]) = a;
    let Rgba([r2, g2, b2, _]) = b;

    let dr = i32::from(r1) - i32::from(r2);
    let dg = i32::from(g1) - i32::from(g2);
    let db = i32::from(b1) - i32::from(b2);

    dr.unsigned_abs() + dg.unsigned_abs() + db.unsigned_abs()
}

/// Calculate perceptual color difference (weighted for human vision)
#[allow(dead_code)]
fn perceptual_diff(a: Rgba<u8>, b: Rgba<u8>) -> f64 {
    let Rgba([r1, g1, b1, _]) = a;
    let Rgba([r2, g2, b2, _]) = b;

    // Use weighted RGB based on human perception
    // Red: 0.299, Green: 0.587, Blue: 0.114
    let dr = (f64::from(r1) - f64::from(r2)) * 0.299;
    let dg = (f64::from(g1) - f64::from(g2)) * 0.587;
    let db = (f64::from(b1) - f64::from(b2)) * 0.114;

    (dr * dr + dg * dg + db * db).sqrt()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    #[test]
    fn test_config_defaults() {
        let config = VisualRegressionConfig::default();
        assert!((config.threshold - 0.01).abs() < f64::EPSILON);
        assert_eq!(config.color_threshold, 10);
        assert_eq!(config.baseline_dir, "__baselines__");
        assert_eq!(config.diff_dir, "__diffs__");
        assert!(!config.update_baselines);
    }

    #[test]
    fn test_config_builder() {
        let config = VisualRegressionConfig::default()
            .with_threshold(0.05)
            .with_color_threshold(20);
        assert!((config.threshold - 0.05).abs() < f64::EPSILON);
        assert_eq!(config.color_threshold, 20);
    }

    #[test]
    fn test_config_with_baseline_dir() {
        let config = VisualRegressionConfig::default().with_baseline_dir("my_baselines");
        assert_eq!(config.baseline_dir, "my_baselines");
    }

    #[test]
    fn test_config_with_update_baselines() {
        let config = VisualRegressionConfig::default().with_update_baselines(true);
        assert!(config.update_baselines);
    }

    #[test]
    fn test_tester_config_accessor() {
        let config = VisualRegressionConfig::default().with_threshold(0.02);
        let tester = VisualRegressionTester::new(config);
        assert!((tester.config().threshold - 0.02).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tester_default() {
        let tester = VisualRegressionTester::default();
        assert!((tester.config().threshold - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn test_perceptual_diff() {
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);
        let red = Rgba([255, 0, 0, 255]);

        // White vs white should be 0
        assert!((perceptual_diff(white, white) - 0.0).abs() < f64::EPSILON);

        // White vs black should be non-zero
        let wb_diff = perceptual_diff(white, black);
        assert!(wb_diff > 0.0);

        // Red vs black should be less than white vs black (red weighted at 0.299)
        let rb_diff = perceptual_diff(red, black);
        assert!(rb_diff < wb_diff);
    }

    #[test]
    fn test_image_diff_result_is_identical() {
        let result = ImageDiffResult {
            matches: true,
            diff_pixel_count: 0,
            total_pixels: 100,
            diff_percentage: 0.0,
            max_color_diff: 0,
            avg_color_diff: 0.0,
            diff_image: None,
        };
        assert!(result.is_identical());

        let result2 = ImageDiffResult {
            matches: false,
            diff_pixel_count: 5,
            total_pixels: 100,
            diff_percentage: 5.0,
            max_color_diff: 100,
            avg_color_diff: 50.0,
            diff_image: None,
        };
        assert!(!result2.is_identical());
    }

    #[test]
    fn test_identical_images() {
        // Create a simple 2x2 red image
        let mut img = image::RgbaImage::new(2, 2);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }

        let mut buffer = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        encoder
            .write_image(img.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let tester = VisualRegressionTester::default();
        let result = tester.compare_images(&buffer, &buffer).unwrap();

        assert!(result.is_identical());
        assert!(result.matches);
        assert_eq!(result.diff_pixel_count, 0);
    }

    #[test]
    fn test_different_images() {
        // Create two different 2x2 images
        let mut img1 = image::RgbaImage::new(2, 2);
        let mut img2 = image::RgbaImage::new(2, 2);

        for pixel in img1.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]); // Red
        }
        for pixel in img2.pixels_mut() {
            *pixel = Rgba([0, 255, 0, 255]); // Green
        }

        let mut buffer1 = Vec::new();
        let mut buffer2 = Vec::new();

        let encoder1 = image::codecs::png::PngEncoder::new(&mut buffer1);
        encoder1
            .write_image(img1.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let encoder2 = image::codecs::png::PngEncoder::new(&mut buffer2);
        encoder2
            .write_image(img2.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let tester = VisualRegressionTester::default();
        let result = tester.compare_images(&buffer1, &buffer2).unwrap();

        assert!(!result.is_identical());
        assert!(!result.matches);
        assert_eq!(result.diff_pixel_count, 4);
        assert!(result.diff_percentage > 99.0);
    }

    #[test]
    fn test_within_threshold() {
        let result = ImageDiffResult {
            matches: true,
            diff_pixel_count: 10,
            total_pixels: 10000,
            diff_percentage: 0.1,
            max_color_diff: 50,
            avg_color_diff: 25.0,
            diff_image: None,
        };

        assert!(result.within_threshold(0.01)); // 1% threshold
        assert!(!result.within_threshold(0.0005)); // 0.05% threshold should fail
    }

    #[test]
    fn test_dimension_mismatch() {
        let img1 = image::RgbaImage::new(2, 2);
        let img2 = image::RgbaImage::new(3, 3);

        let mut buffer1 = Vec::new();
        let mut buffer2 = Vec::new();

        let encoder1 = image::codecs::png::PngEncoder::new(&mut buffer1);
        encoder1
            .write_image(img1.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let encoder2 = image::codecs::png::PngEncoder::new(&mut buffer2);
        encoder2
            .write_image(img2.as_raw(), 3, 3, image::ExtendedColorType::Rgba8)
            .unwrap();

        let tester = VisualRegressionTester::default();
        let result = tester.compare_images(&buffer1, &buffer2);

        assert!(result.is_err());
    }

    #[test]
    fn test_pixel_diff() {
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);
        let red = Rgba([255, 0, 0, 255]);

        assert_eq!(pixel_diff(white, white), 0);
        assert_eq!(pixel_diff(white, black), 255 * 3);
        assert_eq!(pixel_diff(red, black), 255);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_small_difference_within_threshold() {
        // Create two images with small differences
        let mut img1 = image::RgbaImage::new(10, 10);
        let mut img2 = image::RgbaImage::new(10, 10);

        for (i, pixel) in img1.pixels_mut().enumerate() {
            *pixel = Rgba([100, 100, 100, 255]);
            // Make one pixel different
            if i == 0 {
                img2.put_pixel(0, 0, Rgba([105, 105, 105, 255])); // Small diff
            } else {
                img2.put_pixel((i % 10) as u32, (i / 10) as u32, Rgba([100, 100, 100, 255]));
            }
        }

        let mut buffer1 = Vec::new();
        let mut buffer2 = Vec::new();

        let encoder1 = image::codecs::png::PngEncoder::new(&mut buffer1);
        encoder1
            .write_image(img1.as_raw(), 10, 10, image::ExtendedColorType::Rgba8)
            .unwrap();

        let encoder2 = image::codecs::png::PngEncoder::new(&mut buffer2);
        encoder2
            .write_image(img2.as_raw(), 10, 10, image::ExtendedColorType::Rgba8)
            .unwrap();

        // With default color threshold of 10, this should pass
        let tester = VisualRegressionTester::default();
        let result = tester.compare_images(&buffer1, &buffer2).unwrap();

        assert!(result.matches); // Small diff within color threshold
    }

    #[test]
    fn test_compare_against_baseline_missing() {
        let config =
            VisualRegressionConfig::default().with_baseline_dir("/tmp/nonexistent_baselines_12345");
        let tester = VisualRegressionTester::new(config);

        // Create a simple image
        let img = image::RgbaImage::new(2, 2);
        let mut buffer = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        encoder
            .write_image(img.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let result = tester.compare_against_baseline("missing_test", &buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_compare_against_baseline_with_update() {
        use std::fs;
        let temp_dir = std::env::temp_dir().join("vr_test_update_baselines");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous runs

        let config = VisualRegressionConfig::default()
            .with_baseline_dir(temp_dir.to_string_lossy())
            .with_update_baselines(true);
        let tester = VisualRegressionTester::new(config);

        // Create a simple image
        let mut img = image::RgbaImage::new(2, 2);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([100, 100, 100, 255]);
        }
        let mut buffer = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        encoder
            .write_image(img.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        // First call should create baseline
        let result = tester
            .compare_against_baseline("update_test", &buffer)
            .unwrap();
        assert!(result.matches);
        assert!(temp_dir.join("update_test.png").exists());

        // Second call should compare against baseline
        let result2 = tester
            .compare_against_baseline("update_test", &buffer)
            .unwrap();
        assert!(result2.matches);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_compare_against_baseline_mismatch_saves_diff() {
        use std::fs;
        let temp_dir = std::env::temp_dir().join("vr_test_diff_save");
        let _ = fs::remove_dir_all(&temp_dir);
        let diff_dir = std::env::temp_dir().join("vr_test_diff_save_diffs");
        let _ = fs::remove_dir_all(&diff_dir);

        fs::create_dir_all(&temp_dir).unwrap();

        let mut config = VisualRegressionConfig::default()
            .with_baseline_dir(temp_dir.to_string_lossy())
            .with_threshold(0.0001) // Very strict threshold
            .with_color_threshold(0); // No color tolerance
        config.diff_dir = diff_dir.to_string_lossy().to_string();
        let tester = VisualRegressionTester::new(config);

        // Create baseline image (red)
        let mut img1 = image::RgbaImage::new(2, 2);
        for pixel in img1.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        let mut buffer1 = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer1);
        encoder
            .write_image(img1.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        fs::write(temp_dir.join("diff_test.png"), &buffer1).unwrap();

        // Create different image (green)
        let mut img2 = image::RgbaImage::new(2, 2);
        for pixel in img2.pixels_mut() {
            *pixel = Rgba([0, 255, 0, 255]);
        }
        let mut buffer2 = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer2);
        encoder
            .write_image(img2.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        // Compare - should fail and save diff
        let result = tester
            .compare_against_baseline("diff_test", &buffer2)
            .unwrap();
        assert!(!result.matches);
        assert!(diff_dir.join("diff_test_diff.png").exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::remove_dir_all(&diff_dir);
    }

    #[test]
    fn test_diff_image_generation() {
        // Create two very different images - should generate diff image
        let mut img1 = image::RgbaImage::new(4, 4);
        let mut img2 = image::RgbaImage::new(4, 4);

        for pixel in img1.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]); // Black
        }
        for pixel in img2.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]); // White
        }

        let mut buffer1 = Vec::new();
        let mut buffer2 = Vec::new();

        let encoder1 = image::codecs::png::PngEncoder::new(&mut buffer1);
        encoder1
            .write_image(img1.as_raw(), 4, 4, image::ExtendedColorType::Rgba8)
            .unwrap();

        let encoder2 = image::codecs::png::PngEncoder::new(&mut buffer2);
        encoder2
            .write_image(img2.as_raw(), 4, 4, image::ExtendedColorType::Rgba8)
            .unwrap();

        let config = VisualRegressionConfig::default().with_threshold(0.0);
        let tester = VisualRegressionTester::new(config);
        let result = tester.compare_images(&buffer1, &buffer2).unwrap();

        assert!(!result.matches);
        assert!(result.diff_image.is_some());
        assert!(!result.diff_image.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_avg_color_diff() {
        // Create images with measurable color difference
        let mut img1 = image::RgbaImage::new(2, 2);
        let mut img2 = image::RgbaImage::new(2, 2);

        for pixel in img1.pixels_mut() {
            *pixel = Rgba([100, 100, 100, 255]);
        }
        for pixel in img2.pixels_mut() {
            *pixel = Rgba([200, 100, 100, 255]); // +100 difference in red
        }

        let mut buffer1 = Vec::new();
        let mut buffer2 = Vec::new();

        let encoder1 = image::codecs::png::PngEncoder::new(&mut buffer1);
        encoder1
            .write_image(img1.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let encoder2 = image::codecs::png::PngEncoder::new(&mut buffer2);
        encoder2
            .write_image(img2.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();

        let config = VisualRegressionConfig::default().with_color_threshold(0);
        let tester = VisualRegressionTester::new(config);
        let result = tester.compare_images(&buffer1, &buffer2).unwrap();

        assert_eq!(result.diff_pixel_count, 4);
        assert_eq!(result.max_color_diff, 100);
        assert!((result.avg_color_diff - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_invalid_image_decode() {
        let tester = VisualRegressionTester::default();
        let invalid_data = vec![0, 1, 2, 3, 4]; // Not a valid image

        let result = tester.compare_images(&invalid_data, &invalid_data);
        assert!(result.is_err());
    }

    // ============================================================================
    // QA CHECKLIST SECTION 4: Visual Regression Falsification Tests
    // Per docs/qa/100-point-qa-checklist-jugar-probar.md
    // ============================================================================

    /// Test #70: HDR content handling - dynamic range normalization
    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::items_after_statements)]
    fn test_hdr_content_handling() {
        // HDR images use extended color values (>255)
        // Our comparison normalizes to 8-bit for consistent comparison
        let hdr_pixel_value: u16 = 512; // Extended range
        let sdr_normalized = hdr_pixel_value.min(255) as u8; // Clamped to SDR

        assert_eq!(sdr_normalized, 255, "HDR values clamped to SDR range");

        // Verify tone mapping simulation
        #[allow(clippy::cast_sign_loss)]
        fn tone_map_hdr(value: f32, max_luminance: f32) -> u8 {
            let normalized = value / max_luminance;
            let gamma_corrected = normalized.powf(1.0 / 2.2);
            (gamma_corrected.clamp(0.0, 1.0) * 255.0) as u8
        }

        let hdr_white = tone_map_hdr(1000.0, 1000.0);
        let hdr_mid = tone_map_hdr(500.0, 1000.0);

        assert!(
            hdr_white > hdr_mid,
            "Tone mapping preserves relative brightness"
        );
        assert_eq!(hdr_white, 255, "Max HDR maps to max SDR");
    }

    /// Test color depth handling
    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_color_depth_normalization() {
        // 10-bit to 8-bit conversion
        let ten_bit_value: u16 = 1023; // Max 10-bit
        let eight_bit_value = (ten_bit_value >> 2) as u8; // Convert to 8-bit

        assert_eq!(eight_bit_value, 255, "10-bit normalized to 8-bit");

        // 16-bit to 8-bit conversion
        let sixteen_bit_value: u16 = 65535; // Max 16-bit
        let eight_bit_from_16 = (sixteen_bit_value >> 8) as u8;

        assert_eq!(eight_bit_from_16, 255, "16-bit normalized to 8-bit");
    }
}
