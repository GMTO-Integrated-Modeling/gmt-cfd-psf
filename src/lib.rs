/*!
# PSF Library

This library provides tools for generating Point Spread Function (PSF) visualizations
from GMT CFD data using CRSEO optical modeling.

## Key Components

- [`Config`] - Configuration for PSF rendering with metadata overlays
- [`PSF`] - Individual PSF frame with associated metadata
- [`PSFs`] - Collection of PSF frames with batch processing capabilities

## Usage

```rust,no_run
use psf::{Config, PSF, PSFs, DETECTOR_SIZE};
use std::rc::Rc;

// Create configuration
let config = Config::new(seeing_radius, diff_limit_radius, wavelength_nm)
    .cfd_case("30deg_0deg_os_7ms")
    .turbulence_effects("dome seeing");

// Create PSF collection
let mut psfs = PSFs::new(&config);

// Add PSF frames
psfs.push(frame_data, pssn_value);

// Save all frames with global normalization
psfs.save_all_frames()?;
```
*/

/// Default detector size in pixels (760x760)
pub const DETECTOR_SIZE: usize = 760;

mod config;
mod optical_model;
mod psfs;
pub use config::Config;
pub use optical_model::GmtOpticalModel;
pub use psfs::{PSF, PSFs};
