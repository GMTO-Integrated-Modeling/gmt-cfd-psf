use std::{path::Path, rc::Rc};

use crseo::{
    Atmosphere, Builder, CrseoError, FromBuilder, Gmt, Imaging, PSSn, PSSnEstimates, Source,
    imaging::Detector,
    pssn::{PSSnBuilder, TelescopeError},
};
use gmt_dos_clients_domeseeing::{DomeSeeing, DomeSeeingError};
use skyangle::Conversion;

use crate::{Config, DETECTOR_SIZE, PSF, PSFs, optical_model::windloads::WindLoadsError};

mod windloads;
pub use windloads::WindLoads;

#[derive(Debug, thiserror::Error)]
pub enum GmtOpticalModelError {
    #[error("crseo API failed")]
    Crseo(#[from] CrseoError),
    #[error("failed to build dome seeing")]
    DomeSeeing(#[from] DomeSeeingError),
    #[error("failed to build wind loads")]
    WindLoads(#[from] WindLoadsError),
}
type Result<T> = std::result::Result<T, GmtOpticalModelError>;

pub struct GmtOpticalModel {
    gmt: Gmt,
    src: Source,
    imgr: Imaging,
    pssn: PSSn<TelescopeError>,
    domeseeing: Option<DomeSeeing>,
    windloads: Option<WindLoads>,
    config: Rc<Config>,
}

impl GmtOpticalModel {
    pub fn new() -> Result<Self> {
        // Setup GMT optics and imaging
        let gmt = Gmt::builder().build()?;
        let src = Source::builder().band("Vs");
        let pssn = PSSnBuilder::<TelescopeError>::default()
            .source(src.clone())
            .build()?;

        let src = src.build()?;

        // Get wavelength in nanometers for PSSN display
        // let wavelength_nm = src.wavelength() * 1e9; // Convert meters to nanometers

        let imgr = Imaging::builder()
            .detector(
                Detector::default()
                    .n_px_imagelet(DETECTOR_SIZE)
                    .n_px_framelet(DETECTOR_SIZE)
                    .osf(4),
            )
            .build()?;

        let gmt_diff_lim = (1.22 * src.wavelength() / 25.5).to_mas();
        let gmt_segment_diff_lim = (1.22 * src.wavelength() / 8.365).to_mas() as f32;
        println!("GMT diffraction limited FWHM: {:.0}mas", gmt_diff_lim);
        // pixel scale
        let px = imgr.pixel_scale(&src).to_mas();
        println!(
            "Detector: pixel scale: {:.0}mas, FOV: {:.2}arcsec",
            px,
            imgr.field_of_view(&src).to_arcsec()
        );

        let atm = Atmosphere::builder().build()?;
        let seeing = (0.98 * src.wavelength() / atm.r0()).to_mas() as f32;
        println!("Atmosphere seeing: {:.0}mas", seeing);

        // Calculate seeing radius in pixels (diameter = 2 * radius, so radius = seeing / 2 / px)
        let seeing_radius_pixels = (seeing / 2.0) / px;
        // Calculate GMT segment diff lim radius in pixels
        let segment_diff_lim_radius_pixels = (gmt_segment_diff_lim / 2.0) / px;
        // println!("Seeing radius in pixels: {:.1}px", seeing_radius_pixels);
        // println!("GMT segment diff lim radius in pixels: {:.1}px", segment_diff_lim_radius_pixels);
        let config = Config::new(
            seeing_radius_pixels,
            segment_diff_lim_radius_pixels,
            src.wavelength() * 1e9,
        );
        Ok(Self {
            gmt,
            src,
            imgr,
            pssn,
            domeseeing: None,
            windloads: None,
            config,
        })
    }
    pub fn get_config(&self) -> Rc<Config> {
        self.config.clone()
    }
    pub fn set_config(&mut self, config: Rc<Config>) {
        self.config = config;
    }
    pub fn domeseeing(mut self, cfd_path: impl AsRef<Path>) -> Result<Self> {
        self.domeseeing = Some(DomeSeeing::builder(cfd_path).build()?);
        Ok(self)
    }
    pub fn windloads(mut self, rbms_path: impl AsRef<Path>) -> Result<Self> {
        self.windloads = Some(WindLoads::new(rbms_path)?);
        Ok(self)
    }
    pub fn ray_trace(&mut self) -> &mut Self {
        // updating M1 & M2 rigid body motions
        self.windloads.as_mut().map(|windloads| {
            windloads.next().map(|rbms| {
                let (m1_rbms, m2_rbms) = rbms.split_at(42);
                self.gmt.update42(Some(m1_rbms), Some(m2_rbms), None, None);
            })
        });

        self.src.through(&mut self.gmt).xpupil();

        // adding dome seeing OPD map to the wavefront
        self.domeseeing
            .as_mut()
            .map(|domeseeing| domeseeing.next().map(|opd| self.src.add(opd.as_slice())));

        self.src.through(&mut self.imgr);
        self
    }
    pub fn compute_pssn(&mut self) -> f64 {
        self.src.through(&mut self.pssn);
        self.pssn.estimates()[0]
    }
    pub fn read_detector(&mut self) -> PSF {
        let frame: Vec<f32> = self.imgr.frame().into();
        self.imgr.reset();
        PSF::new(&self.config, frame)
    }
}
impl From<&GmtOpticalModel> for PSFs {
    fn from(gmt: &GmtOpticalModel) -> Self {
        Self::new(&gmt.config)
    }
}
