use std::{env, path::PathBuf};

use leptos::prelude::*;

use crate::components::{form_controls::PsfConfig, psf_generator::GeneratedImage};

#[cfg(feature = "ssr")]
static FRAME_ID: std::sync::LazyLock<std::sync::atomic::AtomicUsize> =
    std::sync::LazyLock::new(|| std::sync::atomic::AtomicUsize::new(0));

#[server]
pub async fn psf_generation(
    config: PsfConfig,
    session_id: String,
) -> Result<Vec<GeneratedImage>, ServerFnError> {
    use crate::N_SAMPLE;
    use object_store::{path::Path, ObjectStore};
    use parse_monitors::{
        cfd::{Baseline, BaselineTrait, CfdCase},
        CFD_YEAR,
    };
    use psf::{get_enclosure_config, GmtOpticalModel, PSFs, StorePath, ZenithAngle};
    use std::{
        env,
        fs::create_dir_all,
        sync::{atomic::Ordering, Arc},
        time::Instant,
    };

    let store: Arc<dyn ObjectStore> =
        Arc::new(object_store::local::LocalFileSystem::new_with_prefix(
            format!("{}/maua", env::var("HOME")?),
        )?);

    let now = Instant::now();

    let mut images = Vec::new();

    // Setup GMT optics and imaging
    let mut gmt = GmtOpticalModel::new()?;

    // Generate reference frame (no turbulence)
    let output_dir = format!("target/site/generated/{}", session_id);
    create_dir_all(&output_dir)?;

    let psf_path = format!("{}/psf.png", output_dir);
    gmt.ray_trace().read_detector().save(&psf_path)?;

    images.push(GeneratedImage {
        name: "Diffraction Limited".to_string(),
        path: format!("generated/{}/psf.png", session_id),
        description: "GMT diffraction limited PSF".to_string(),
    });

    // Generate turbulence effects string
    let turbulence_effects = match (config.domeseeing, config.windloads) {
        (true, true) => Some(format!("Dome Seeing + {}", config.rbm_time_series)),
        (true, false) => Some("Dome Seeing".to_string()),
        (false, true) => Some(config.rbm_time_series.to_string()),
        (false, false) => return Ok(vec![]),
    };

    if let Some(effects) = turbulence_effects {
        gmt.set_config(gmt.get_config().turbulence_effects(effects));
    }

    // CFD case configuration
    let zenith = ZenithAngle::from(config.elevation_angle).as_u32();
    let azimuth = config.azimuth_angle.as_u32();
    let wind_speed = config.wind_speed.as_u32();
    let enclosure = get_enclosure_config(wind_speed, config.elevation_angle);

    let cfd_case = CfdCase::<CFD_YEAR>::colloquial(zenith, azimuth, enclosure, wind_speed)?;
    gmt.set_config(gmt.get_config().cfd_case(cfd_case.to_string()));

    // Setup dome seeing if requested
    let gmt = if config.domeseeing {
        let cfd_path =
            Path::from(Baseline::<CFD_YEAR>::path()?.to_str().unwrap()).join(cfd_case.to_string());
        gmt.domeseeing(store.clone(), cfd_path).await?
    } else {
        gmt
    };

    // Setup wind loads if requested
    let mut gmt = if config.windloads {
        let rbms_path = Path::new(env::var("FEM")?)
            .join("cfd")
            .join(cfd_case)
            .join(config.rbm_time_series.file_name());
        leptos::logging::log!("{}", rbms_path);
        gmt.windloads(store.clone(), rbms_path).await?
    } else {
        gmt
    };

    // Process turbulence-affected frames
    let mut psfs = PSFs::from(&gmt);

    for i in 0..N_SAMPLE {
        FRAME_ID.store(i, Ordering::Relaxed);
        psfs.push(
            gmt.async_ray_trace()
                .await
                .read_detector()
                .opd(gmt.get_opd())
                .pssn_value(gmt.compute_pssn()),
        );
    }

    // Setup output directory for frames
    let frames_dir = format!("{}/frames", output_dir);
    // Save all turbulence frames with consistent normalization
    psfs.save_all_frames(frames_dir, &*FRAME_ID)?;

    let long_exposure_path = format!("{}/long_exposure_psf.png", output_dir);
    psfs.sum().save(&long_exposure_path)?;
    images.push(GeneratedImage {
        name: "Long exposure PSF".to_string(),
        path: format!("generated/{}/long_exposure_psf.png", session_id),
        description: "GMT long exposure CFD PSF".to_string(),
    });

    println!(
        "✅ Processing completed in {:.2}s for session {}",
        now.elapsed().as_secs_f64(),
        session_id
    );
    println!(
        "📁 Saved {} frames to {}/frames/ directory",
        psfs.len(),
        output_dir
    );
    Ok(dbg!(images))
}
#[server]
pub async fn psf_animation(output_dir: PathBuf) -> Result<GeneratedImage, ServerFnError> {
    use std::{path::Path, process::Command};
    println!("   convert -delay 20 -loop 0 frames/frame_*.png psf_animation.gif");
    let root = Path::new("target").join("site").join(&output_dir);
    Command::new("/usr/bin/convert")
        .arg("-delay")
        .arg("20")
        .arg("-loop")
        .arg("0")
        .arg(root.join("frames").join("frame_*.png"))
        .arg(root.join("psf_animation.gif"))
        .output()?;
    Ok(GeneratedImage {
        name: "Short exposure PSFs animation".to_string(),
        path: format!(
            "{:}",
            output_dir.join("psf_animation.gif").to_str().unwrap()
        ),

        description: "GMT short exposure CFD PSFs animation".to_string(),
    })
}
#[server]
pub async fn opd_animation(output_dir: PathBuf) -> Result<GeneratedImage, ServerFnError> {
    use std::{path::Path, process::Command};
    println!("   convert -delay 20 -loop 0 frames/opd_*.png opd_animation.gif");
    let root = Path::new("target").join("site").join(&output_dir);
    Command::new("/usr/bin/convert")
        .arg("-delay")
        .arg("20")
        .arg("-loop")
        .arg("0")
        .arg(root.join("frames").join("opd_*.png"))
        .arg(root.join("opd_animation.gif"))
        .output()?;
    Ok(GeneratedImage {
        name: "Short exposure OPDs animation".to_string(),
        path: format!(
            "{:}",
            output_dir.join("opd_animation.gif").to_str().unwrap()
        ),

        description: "GMT CFD OPDs animation".to_string(),
    })
}

#[server]
pub async fn get_frame_id() -> Result<usize, ServerFnError> {
    Ok(FRAME_ID.load(std::sync::atomic::Ordering::Relaxed))
}
