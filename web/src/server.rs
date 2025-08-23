#[cfg(feature = "ssr")]
pub mod api {
    use std::{env, fs::create_dir_all, path::Path, time::Instant};
    use actix_web::{web, HttpResponse, Result as ActixResult};
    use serde::Deserialize;
    use psf::{GmtOpticalModel, PSFs};
    use parse_monitors::{
        CFD_YEAR,
        cfd::{Baseline, BaselineTrait, CfdCase},
    };
    
    use crate::components::{
        form_controls::PsfConfig,
        psf_generator::{GenerationStatus, ProcessingStatus},
    };

    const N_SAMPLE: usize = 100;

    #[derive(Debug, Deserialize)]
    pub struct GenerateRequest {
        pub config: PsfConfig,
        pub session_id: String,
    }

    pub async fn generate_psf(
        req: web::Json<GenerateRequest>,
    ) -> ActixResult<HttpResponse> {
        let config = req.config.clone();
        let session_id = req.session_id.clone();

        println!("generate psf");

        // Validate that at least one turbulence effect is selected
        if !config.domeseeing && !config.windloads {
            return Ok(HttpResponse::BadRequest().json(GenerationStatus {
                session_id,
                status: ProcessingStatus::Error,
                message: "At least one turbulence effect (dome seeing or wind loads) must be selected".to_string(),
                progress: None,
                images: Vec::new(),
            }));
        }

        // Start processing in background task
        tokio::spawn(async move {
            if let Err(e) = process_psf_generation(config, session_id).await {
                eprintln!("PSF generation error: {}", e);
            }
        });

        Ok(HttpResponse::Ok().json(GenerationStatus {
            session_id: req.session_id.clone(),
            status: ProcessingStatus::Processing,
            message: "PSF generation started".to_string(),
            progress: Some(0.0),
            images: Vec::new(),
        }))
    }

    async fn process_psf_generation(
        config: PsfConfig,
        session_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = Instant::now();

        // Setup GMT optics and imaging
        let mut gmt = GmtOpticalModel::new()?;

        // Generate reference frame (no turbulence)
        let output_dir = format!("target/site/generated/{}", session_id);
        create_dir_all(&output_dir)?;
        
        let psf_path = format!("{}/psf.png", output_dir);
        gmt.ray_trace().read_detector().save(&psf_path)?;

        // Generate turbulence effects string
        let turbulence_effects = match (config.domeseeing, config.windloads) {
            (true, true) => Some("Dome Seeing + Wind Loads"),
            (true, false) => Some("Dome Seeing"),
            (false, true) => Some("Wind Loads"),
            (false, false) => return Ok(()),
        };

        if let Some(effects) = turbulence_effects {
            gmt.set_config(gmt.get_config().turbulence_effects(effects));
        }

        // CFD case configuration
        let zenith = config.zenith_angle.as_u32();
        let azimuth = config.azimuth_angle.as_u32();  
        let wind_speed = config.wind_speed.as_u32();
        let enclosure = get_enclosure_config(wind_speed, zenith);

        let cfd_case = CfdCase::<CFD_YEAR>::colloquial(zenith, azimuth, enclosure, wind_speed)?;
        gmt.set_config(gmt.get_config().cfd_case(cfd_case.to_string()));

        // Setup dome seeing if requested
        let gmt = if config.domeseeing {
            let cfd_path = Baseline::<CFD_YEAR>::path()?.join(cfd_case.to_string());
            gmt.domeseeing(cfd_path)?
        } else {
            gmt
        };

        // Setup wind loads if requested
        let mut gmt = if config.windloads {
            let rbms_path = Path::new(&env::var("FEM_REPO")?)
                .join("cfd")
                .join(cfd_case.to_string())
                .join("m1_m2_rbms.parquet");
            gmt.windloads(rbms_path)?
        } else {
            gmt
        };

        // create_dir_all(&frames_dir)?;

        // Process turbulence-affected frames
        let mut psfs = PSFs::from(&gmt);

        for i in 0..N_SAMPLE {
            psfs.push(
                gmt.ray_trace()
                    .read_detector()
                    .pssn_value(gmt.compute_pssn()),
            );
            
            // Could emit progress updates here via websocket or polling endpoint
            let progress = ((i + 1) as f32 / N_SAMPLE as f32) * 100.0;
            println!("Progress: {:.1}%", progress);
        }

        // Setup output directory for frames
        let frames_dir = format!("{}/frames", output_dir);
        // Save all turbulence frames with consistent normalization
        psfs.save_all_frames(frames_dir)?;
        
        let long_exposure_path = format!("{}/long_exposure_psf.png", output_dir);
        psfs.sum().save(&long_exposure_path)?;

        println!(
            "✅ Processing completed in {:.2}s for session {}",
            now.elapsed().as_secs_f64(),
            session_id
        );
        println!("📁 Saved {} frames to {}/frames/ directory", psfs.len(), output_dir);

        Ok(())
    }

    /// Determine enclosure configuration based on wind speed and zenith angle
    fn get_enclosure_config(wind_speed: u32, zenith_angle: u32) -> &'static str {
        if wind_speed <= 7 {
            "os" // open sky for wind <= 7 m/s
        } else if zenith_angle < 60 {
            "cd" // closed dome for wind > 7 m/s and zenith < 60°
        } else {
            "cs" // closed sky for wind > 7 m/s and zenith >= 60°
        }
    }

    pub fn config_routes(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::resource("/api/generate")
                .route(web::post().to(generate_psf))
        );
    }
}

#[cfg(not(feature = "ssr"))]
pub mod api {
    // Stub for client-side compilation
}
