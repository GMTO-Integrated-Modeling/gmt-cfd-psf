/*!
# Individual PSF Frame

This module provides the [`PSF`] type for representing individual Point Spread Function
frames with associated metadata and rendering capabilities.

## Features

- CUBEHELIX colormap visualization
- Seeing and diffraction limit circle overlays
- PSSN and metadata text overlays
- Flexible normalization (global or local)
- PNG export with comprehensive annotations
*/

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use image::{ImageBuffer, ImageError, Rgb};
use imageproc::drawing::draw_hollow_circle_mut;

use super::find_global_extrema;
use crate::{Config, DETECTOR_SIZE, config::ConfigError};

/// Errors that can occur during PSF operations
#[derive(Debug, thiserror::Error)]
pub enum PSFError {
    /// Failed to create RGB image buffer from PSF data
    #[error("Failed to create image buffer")]
    Image,
    /// Failed to save PSF image to file
    #[error("Failed to save PSD to png file {1:?}")]
    Save(#[source] ImageError, PathBuf),
    /// Configuration error during text overlay rendering
    #[error("Failed to invoke config")]
    Config(#[from] ConfigError),
}
type Result<T> = std::result::Result<T, PSFError>;

/// Individual PSF frame with intensity data and associated metadata
///
/// Represents a single Point Spread Function with optional PSSN value,
/// frame numbering, and shared configuration for consistent rendering.
///
/// # Example
///
/// ```rust,no_run
/// use psf::{Config, PSF};
///
/// let config = Config::new(50.0, 25.0, 500.0);
/// let psf = PSF::new(&config, frame_data)
///     .pssn_value(0.85)
///     .frame_number(42);
///
/// psf.save("output.png")?;
/// ```
#[derive(Debug, Default)]
pub struct PSF {
    pub(crate) frame: Vec<f32>,
    pub(crate) pssn_value: Option<f64>,
    pub(crate) frame_number: Option<usize>,
    pub(crate) config: Rc<Config>,
}
impl PSF {
    /// Create a new PSF frame with intensity data and shared configuration
    ///
    /// # Parameters
    ///
    /// - `config` - Shared rendering configuration
    /// - `frame` - PSF intensity data as flat vector (DETECTOR_SIZE²)
    ///
    /// # Returns
    ///
    /// PSF instance ready for metadata assignment and rendering
    pub fn new(config: &Rc<Config>, frame: Vec<f32>) -> Self {
        Self {
            frame,
            config: config.clone(),
            ..Default::default()
        }
    }

    /// Assign PSSN (Normalized point source sensitivity) value to this frame
    ///
    /// # Parameters
    ///
    /// - `value` - PSSN value typically between 0.0 and 1.0
    ///
    /// # Returns
    ///
    /// PSF instance with PSSN metadata for text overlay rendering
    pub fn pssn_value(mut self, value: f64) -> Self {
        self.pssn_value = Some(value);
        self
    }

    /// Assign frame number for animated sequence identification
    ///
    /// # Parameters
    ///
    /// - `value` - Zero-based frame index
    ///
    /// # Returns
    ///
    /// PSF instance with frame number metadata for text overlay rendering
    pub fn frame_number(mut self, value: usize) -> Self {
        self.frame_number = Some(value);
        self
    }
    /// Convert PSF intensity data to RGB image data using CUBEHELIX colormap
    ///
    /// Normalizes intensity values to 0.0-1.0 range using provided min/max bounds,
    /// then applies scientific CUBEHELIX colormap for perceptually uniform visualization.
    ///
    /// # Parameters
    ///
    /// - `min_val` - Minimum intensity value for normalization
    /// - `max_val` - Maximum intensity value for normalization
    ///
    /// # Returns
    ///
    /// RGB pixel data as flat byte vector (3 × DETECTOR_SIZE²)
    fn frame_to_rgb(&self, min_val: f32, max_val: f32) -> Vec<u8> {
        let range = max_val - min_val;
        let normalized: Vec<f64> = if range > 0.0 {
            self.frame
                .iter()
                .map(|&x| ((x - min_val) / range) as f64)
                .collect()
        } else {
            vec![0.5f64; self.frame.len()]
        };

        normalized
            .iter()
            .flat_map(|&value| {
                let color = colorous::CUBEHELIX.eval_continuous(value);
                [color.r, color.g, color.b]
            })
            .collect()
    }
    /// Export PSF frame as annotated PNG image with local normalization
    ///
    /// Renders the PSF with CUBEHELIX colormap, circle overlays for seeing and
    /// diffraction limits, and text overlays for PSSN and metadata. Uses local
    /// min/max normalization for this frame only.
    ///
    /// # Parameters
    ///
    /// - `filename` - Output PNG file path
    ///
    /// # Returns
    ///
    /// Result indicating success or rendering/save error
    pub fn save(&self, filename: impl AsRef<Path>) -> Result<()> {
        self.save_frame_as_png(filename, None)
    }

    /// Export PSF frame as annotated PNG image with optional global normalization
    ///
    /// Comprehensive PSF rendering including:
    /// - CUBEHELIX colormap with configurable normalization bounds
    /// - White hollow circles for atmospheric seeing and GMT diffraction limits
    /// - Text overlays for CFD case, turbulence effects, PSSN, and frame number
    ///
    /// # Parameters
    ///
    /// - `filename` - Output PNG file path  
    /// - `minmax` - Optional (min, max) bounds for normalization; uses local bounds if None
    ///
    /// # Returns
    ///
    /// Result indicating success or rendering/save error
    pub fn save_frame_as_png(
        &self,
        filename: impl AsRef<Path>,
        minmax: Option<(f32, f32)>,
    ) -> Result<()> {
        // let Self {
        //     frame,
        //     pssn_value,
        //     frame_number,
        //     config,
        // } = self;
        let &Config {
            seeing_radius_pixels,
            segment_diff_lim_radius_pixels,
            ..
        } = &*self.config;

        let (min_val, max_val) =
            minmax.unwrap_or_else(|| find_global_extrema(&[self.frame.as_slice()]));

        let rgb_data = self.frame_to_rgb(min_val, max_val);
        let mut image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(
            DETECTOR_SIZE as u32,
            DETECTOR_SIZE as u32,
            rgb_data,
        )
        .ok_or_else(|| PSFError::Image)?;

        let center = (DETECTOR_SIZE as i32 / 2, DETECTOR_SIZE as i32 / 2);

        // Draw seeing circle (hollow) if radius is provided
        let white = Rgb([255u8, 255u8, 255u8]);
        draw_hollow_circle_mut(&mut image, center, seeing_radius_pixels as i32, white);

        // Draw GMT segment diffraction limit circle (hollow) if radius is provided
        draw_hollow_circle_mut(
            &mut image,
            center,
            segment_diff_lim_radius_pixels as i32,
            white,
        );

        // Draw PSSN text if values are provided
        if let Some(pssn) = self.pssn_value {
            self.config
                .draw_pssn_text(&mut image, pssn, self.frame_number)?;
        }

        image
            .save(&filename)
            .map_err(|e| PSFError::Save(e, filename.as_ref().to_path_buf()))?;
        Ok(())
    }
}
