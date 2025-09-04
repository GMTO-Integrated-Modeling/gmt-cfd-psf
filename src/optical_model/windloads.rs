use gmt_lom::{LinearOpticalModelError, RigidBodyMotions, Table};
use nalgebra::DMatrix;
use object_store::{ObjectStore, path::Path};

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
    pub async fn new(storage: impl ObjectStore, path: impl Into<Path>) -> Result<Self> {
        let table = Table::from_stored_parquet(storage, path.into()).await?;
        let rbms = RigidBodyMotions::from_table(
            &table,
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
