/*!
#  CFD Dome Seeing & Wind Loads PSFs

```shell
export CUDACXX=/usr/local/cuda/bin/nvcc
export FEM_REPO=~/mnt/20250506_1715_zen_30_M1_202110_FSM_202305_Mount_202305_pier_202411_M1_actDamping/
export CFD_REPO=~/maua/CASES/
export GMT_MODES_PATH=~/Dropbox/AWS/CEO/gmtMirrors/
cargo r -r -- --help
```
*/

use std::{env, fs::create_dir_all, path::Path, time::Instant};

use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use parse_monitors::{
    CFD_YEAR,
    cfd::{Baseline, BaselineTrait, CfdCase},
};
use psf::{AzimuthAngle, GmtOpticalModel, PSFs, WindSpeed, ZenithAngle, get_enclosure_config};

const N_SAMPLE: usize = 100;

#[derive(Debug, Clone, ValueEnum)]
enum Exposure {
    Short,
    Long,
}

#[derive(Parser)]
#[command(name = "psf")]
#[command(about = "Generate PSF frames from GMT CFD dome seeing data")]
struct Args {
    /// Enable dome seeing turbulence effects
    #[arg(long)]
    domeseeing: bool,

    /// Enable wind loads effects
    #[arg(long)]
    windloads: bool,

    /// Zenith angle in degrees (0, 30, or 60)
    #[arg(long, value_enum, default_value_t = ZenithAngle::Thirty)]
    zenith_angle: ZenithAngle,

    /// Azimuth angle in degrees (0, 45, 90, 135, or 180)
    #[arg(long, value_enum, default_value_t = AzimuthAngle::Zero)]
    azimuth_angle: AzimuthAngle,

    /// Wind speed in m/s (2, 7, 12, or 17)
    #[arg(long, value_enum, default_value_t = WindSpeed::Seven)]
    wind_speed: WindSpeed,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Setup GMT optics and imaging
    let mut gmt = GmtOpticalModel::new()?;

    // Generate reference frame (no turbulence)
    gmt.ray_trace().read_detector().save("psf.png")?;
    println!("Saved frame0 as psf.png");

    // Generate turbulence effects string
    let turbulence_effects = match (args.domeseeing, args.windloads) {
        (true, true) => Some("Dome Seeing + Wind Loads"),
        (true, false) => Some("Dome Seeing"),
        (false, true) => Some("Wind Loads"),
        (false, false) => return Ok(()),
    };

    turbulence_effects.map(|value| gmt.set_config(gmt.get_config().turbulence_effects(value)));

    // CFD case - extract values from arguments
    let zenith = u32::from(args.zenith_angle);
    let azimuth = u32::from(args.azimuth_angle);
    let wind_speed = u32::from(args.wind_speed);
    let enclosure = get_enclosure_config(wind_speed, args.zenith_angle);

    println!("CFD Configuration:");
    println!("  Zenith angle: {}°", zenith);
    println!("  Azimuth angle: {}°", azimuth);
    println!("  Wind speed: {} m/s", wind_speed);
    println!("  Enclosure: {}", enclosure);

    let cfd_case = CfdCase::<CFD_YEAR>::colloquial(zenith, azimuth, enclosure, wind_speed)?;
    gmt.set_config(gmt.get_config().cfd_case(cfd_case));

    let gmt = if args.domeseeing {
        let cfd_path = Baseline::<CFD_YEAR>::path()?.join(cfd_case.to_string());
        gmt.domeseeing(cfd_path)?
    } else {
        gmt
    };

    let mut gmt = if args.windloads {
        let rbms_path = Path::new(&env::var("FEM_REPO")?)
            .join("cfd")
            .join(cfd_case.to_string())
            .join("m1_m2_rbms.parquet");
        gmt.windloads(rbms_path)?
    } else {
        gmt
    };

    // Setup output directory
    let frames_dir = Path::new("frames");
    create_dir_all(frames_dir)?;

    // Process turbulence-affected frames
    let now = Instant::now();
    let mut psfs = PSFs::from(&gmt);

    // Create progress bar for frame processing
    let process_pb = ProgressBar::new(N_SAMPLE as u64);
    process_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    process_pb.set_message("Processing PSF frames");

    for _ in 0..N_SAMPLE {
        psfs.push(
            gmt.ray_trace()
                .read_detector()
                .pssn_value(gmt.compute_pssn()),
        );
        process_pb.inc(1);
    }

    process_pb.finish_with_message("PSF processing complete");
    let frame_count = psfs.len();

    // Save all turbulence frames with consistent normalization
    psfs.save_all_frames("frames")?;
    psfs.sum().save("long_exposure_psf.png")?;

    println!();
    println!(
        "✅ Processing completed in {:.2}s",
        now.elapsed().as_secs_f64()
    );
    println!("📁 Saved {} frames to ./frames/ directory", frame_count);
    println!("🖼️  Reference PSF saved as psf.png");
    println!("🖼️  Long exposure PSF saved as long_exposure_psf.png");
    println!();
    println!("🎬 To create an animated GIF at 5Hz, run:");
    println!("   convert -delay 20 -loop 0 frames/frame_*.png psf_animation.gif");

    Ok(())
}
