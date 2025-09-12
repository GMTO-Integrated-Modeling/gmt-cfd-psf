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

cfg_if::cfg_if! {
    if #[cfg(feature="ssr")] {
        use std::fmt::Display;
        mod config;
        mod optical_model;
        mod psfs;
        pub use config::Config;
        pub use optical_model::GmtOpticalModel;
        pub use psfs::{PSF, PSFs};

        pub trait StorePath {
            fn new(path: impl Into<Self>) -> Self
            where
                Self: Sized,
            {
                path.into()
            }
            fn join(&self, path: impl Display) -> Self;
            fn set_extension(&mut self, ext: impl Display);
        }

        impl StorePath for object_store::path::Path {
            fn join(&self, path: impl Display) -> Self {
                Self::new(format!("{}/{}", self, path))
            }
            fn set_extension(&mut self, ext: impl Display) {
                *self = Self::new(format!("{}.{}", self, ext));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ZenithAngle {
    #[cfg_attr(feature = "clap", value(name = "0"))]
    Zero = 0,
    #[cfg_attr(feature = "clap", value(name = "30"))]
    Thirty = 30,
    #[cfg_attr(feature = "clap", value(name = "60"))]
    Sixty = 60,
}

impl From<ZenithAngle> for u32 {
    fn from(zen: ZenithAngle) -> u32 {
        zen as u32
    }
}

impl ZenithAngle {
    pub fn all() -> Vec<Self> {
        vec![Self::Zero, Self::Thirty, Self::Sixty]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zero => "0°",
            Self::Thirty => "30°",
            Self::Sixty => "60°",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ElevationAngle {
    #[cfg_attr(feature = "clap", value(name = "90"))]
    Ninety = 90,
    #[cfg_attr(feature = "clap", value(name = "30"))]
    Thirty = 30,
    #[cfg_attr(feature = "clap", value(name = "60"))]
    Sixty = 60,
}

impl From<ElevationAngle> for u32 {
    fn from(zen: ElevationAngle) -> u32 {
        zen as u32
    }
}

impl ElevationAngle {
    pub fn all() -> Vec<Self> {
        vec![Self::Ninety, Self::Sixty, Self::Thirty]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ninety => "90°",
            Self::Thirty => "30°",
            Self::Sixty => "60°",
        }
    }
}

impl From<ElevationAngle> for ZenithAngle {
    fn from(value: ElevationAngle) -> Self {
        match value {
            ElevationAngle::Ninety => ZenithAngle::Zero,
            ElevationAngle::Thirty => ZenithAngle::Sixty,
            ElevationAngle::Sixty => ZenithAngle::Thirty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AzimuthAngle {
    #[cfg_attr(feature = "clap", value(name = "0"))]
    Zero = 0,
    #[cfg_attr(feature = "clap", value(name = "45"))]
    FortyFive = 45,
    #[cfg_attr(feature = "clap", value(name = "90"))]
    Ninety = 90,
    #[cfg_attr(feature = "clap", value(name = "135"))]
    OneThirtyFive = 135,
    #[cfg_attr(feature = "clap", value(name = "180"))]
    OneEighty = 180,
}

impl From<AzimuthAngle> for u32 {
    fn from(az: AzimuthAngle) -> u32 {
        az as u32
    }
}

impl AzimuthAngle {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Zero,
            Self::FortyFive,
            Self::Ninety,
            Self::OneThirtyFive,
            Self::OneEighty,
        ]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zero => "0°",
            Self::FortyFive => "45°",
            Self::Ninety => "90°",
            Self::OneThirtyFive => "135°",
            Self::OneEighty => "180°",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WindSpeed {
    #[cfg_attr(feature = "clap", value(name = "2"))]
    Two = 2,
    #[cfg_attr(feature = "clap", value(name = "7"))]
    Seven = 7,
    #[cfg_attr(feature = "clap", value(name = "12"))]
    Twelve = 12,
    #[cfg_attr(feature = "clap", value(name = "17"))]
    Seventeen = 17,
}

impl From<WindSpeed> for u32 {
    fn from(ws: WindSpeed) -> u32 {
        ws as u32
    }
}

impl WindSpeed {
    pub fn all() -> Vec<Self> {
        vec![Self::Two, Self::Seven, Self::Twelve, Self::Seventeen]
    }

    pub fn as_u32(&self) -> u32 {
        *self as u32
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Two => "2 m/s",
            Self::Seven => "7 m/s",
            Self::Twelve => "12 m/s",
            Self::Seventeen => "17 m/s",
        }
    }
}

/// Determine enclosure configuration based on wind speed and zenith angle
pub fn get_enclosure_config(
    wind_speed: u32,
    telescope_pointing: impl Into<ZenithAngle>,
) -> &'static str {
    if wind_speed <= 7 {
        "os" // open sky for wind <= 7 m/s
    } else if telescope_pointing.into().as_u32() < 60 {
        "cd" // closed dome for wind > 7 m/s and zenith < 60°
    } else {
        "cs" // closed sky for wind > 7 m/s and zenith >= 60°
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use skyangle::Conversion;

    use crate::GmtOpticalModel;

    #[test]
    fn gmt_segment_identification() -> Result<(), Box<dyn Error>> {
        let mut gmt = GmtOpticalModel::new()?;
        gmt.gmt()
            .m1_segment_state(1, vec![0.; 3].as_slice(), &[1f64.from_arcsec(), 0f64, 0f64]);
        gmt.gmt().m1_segment_state(
            2,
            vec![0.; 3].as_slice(),
            &[0f64, 1f64.from_arcsec(), 0f64, 0f64],
        );
        let psf = gmt.ray_trace().read_detector().opd(gmt.get_opd());
        psf.save_opd_as_png("gmt_segment_identification.png", None)?;
        Ok(())
    }
}
