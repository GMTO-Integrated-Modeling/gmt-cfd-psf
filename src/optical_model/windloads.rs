use std::path::Path;

use gmt_lom::{LinearOpticalModelError, RigidBodyMotions};
use nalgebra::DMatrix;

#[derive(Debug, thiserror::Error)]
pub enum WindLoadsError {
    #[error("failed to load M1 & M1 rigid body motion time series")]
    Loading(#[from] LinearOpticalModelError),
}
type Result<T> = std::result::Result<T, WindLoadsError>;

pub struct WindLoads {
    rbms: DMatrix<f64>,
    step: usize,
    count: usize,
}
impl WindLoads {
    // M1 & M2 RBMs iterator `N_SAMPLE` @ 5Hz
    // The RBMs are sampled at 1kHz and ramped up from zero
    // reaching steady state after 3s
    // The 1st 5s (5000 samples) are skipped and the RBMs are
    // downsampled by a factor 1000Hz/5Hz=200
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let rbms = RigidBodyMotions::from_parquet(
            path,
            Some("M1RigidBodyMotions"),
            Some("M2RigidBodyMotions"),
        )?
        .into_data();
        Ok(Self {
            rbms,
            step: 200,
            count: 5000,
        })
    }
}
impl Iterator for WindLoads {
    type Item = Box<[f64]>;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.count;
        if i < self.rbms.ncols() {
            self.count += self.step;
            Some(self.rbms.column(i).as_slice().into())
        } else {
            None
        }
    }
}
