/*!
# Wind loads M1 & M2 rigid body motions

Applies the CFD wind loads to the GMT FEM in closed loop with the mount, M1 and M2
servo-mechanisms (zero setpoints) and saves the M1 & M2 rigid body motions (RBMs)
sampled at 1kHz, for trade studies on the FEM (model, proportional damping coefficient).

The model is the same as the one that generated the baseline `<FEM>/cfd/<case>/m1_m2_rbms.parquet`
(`windloads` crate of [parse-monitors](https://github.com/rconan/parse-monitors)):
 - the 20Hz CFD loads (`CASES/<case>/monitors.csv.z` in the bucket) are upsampled to 1kHz
   with a first order hold and ramped up with a sigmoid,
 - the FEM modal state space model is discretized at 1kHz with the given proportional damping
   and static gain compensation,
 - the mount (`MOUNT_MODEL`), M1 actuators (12.5Hz) and M2 positioners controllers hold the
   zero setpoints.

The FEM inputs and outputs are generated at compile time from the FEM in `FEM_REPO`
(`MOUNT_MODEL=MOUNT_FDR_1kHz` too), at runtime the FEM is loaded from `FEM_REPO`,
or downloaded from `<FEM>/modal_state_space_model_2ndOrder.zip` in the bucket if missing.
Another FEM with the same inputs and outputs can be used without recompiling.

Output (in the S3 bucket, or in `--local` directory):
`<FEM>/<STUDY>/cfd/<case>/m1_m2_rbms.parquet`, the default `STUDY` is derived from the damping
coefficient (e.g. `zeta_0.5pc`); the segment piston, open loop and with the FSM and ASM loops,
is then derived with:
```shell
piston-batch --fem <FEM>/<STUDY> ...
```

```shell
MOUNT_MODEL=MOUNT_FDR_1kHz FEM_REPO=<path/to/FEM> cargo r -r -p piston-batch --features windloads --bin windloads -- --help
```
*/

use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, anyhow};
use clap::Parser;
use futures::StreamExt;
use gmt_dos_actors::{
    actor::Terminator,
    model::Model,
    prelude::{AddActorOutput, AddOuput, IntoLogs, TryIntoInputs, vec_box},
    system::Sys,
};
use gmt_dos_clients_arrow::Arrow;
use gmt_dos_clients_io::{
    cfd_wind_loads::{CFDM1WindLoads, CFDM2WindLoads, CFDMountWindLoads},
    gmt_m1::M1RigidBodyMotions,
    gmt_m2::M2RigidBodyMotions,
};
use gmt_dos_clients_servos::{GmtFem, GmtServoMechanisms, WindLoads};
use gmt_dos_clients_windloads::{
    CfdLoads,
    system::{M1, M2, Mount, SigmoidCfdLoads},
};
use gmt_fem::FEM;
use object_store::{ObjectStore, ObjectStoreExt, path::Path as ObjectPath};
use tokio::io::AsyncWriteExt;

/// M1 actuators sampling rate divider (1kHz/80=12.5Hz)
const ACTUATOR_RATE: usize = 80;
const FEM_ZIP: &str = "modal_state_space_model_2ndOrder.zip";
const RBMS: &str = "m1_m2_rbms.parquet";

#[derive(Parser)]
#[command(name = "windloads")]
#[command(about = "M1 & M2 RBMs from the CFD wind loads applied to the GMT FEM")]
struct Args {
    /// FEM model path in the bucket, the outputs are written in `<FEM>/<STUDY>/cfd`
    #[arg(long, env = "FEM")]
    fem: String,

    /// Local FEM directory with `modal_state_space_model_2ndOrder.zip`
    #[arg(long, env = "FEM_REPO")]
    fem_repo: PathBuf,

    /// FEM proportional damping coefficient
    #[arg(long, default_value_t = 0.02)]
    damping: f64,

    /// Disables the FEM static gain compensation
    #[arg(long)]
    no_static_gain_compensation: bool,

    /// Trade study name [default: zeta_<damping in %>pc]
    #[arg(long, env = "STUDY")]
    study: Option<String>,

    /// Restricts the computation to these CFD cases (e.g. zen30az000_OS_7ms)
    #[arg(long = "case")]
    cases: Vec<String>,

    /// CFD cases path in the bucket
    #[arg(long, default_value = "CASES")]
    cfd: String,

    /// Local directory where the CFD wind loads are cached
    #[arg(long, env = "CFD_REPO", default_value = "CASES")]
    cfd_repo: PathBuf,

    /// Simulation duration in seconds, the last `duration` seconds of the CFD record
    #[arg(long, default_value_t = 400)]
    duration: usize,

    /// Simulation sampling frequency in Hz
    #[arg(long, default_value_t = 1000)]
    sampling_frequency: usize,

    /// Recomputes the RBMs even if they already exist
    #[arg(long)]
    overwrite: bool,

    /// Writes the outputs to this local directory instead of the bucket
    #[arg(long)]
    local: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let _ = dotenvy::from_filename(".env_s3");
    let args = Args::parse();

    let mut builder = object_store::aws::AmazonS3Builder::from_env();
    if let Ok(region) = env::var("REGION") {
        builder = builder.with_region(region);
    }
    if let Ok(bucket) = env::var("BUCKET") {
        builder = builder.with_bucket_name(bucket);
    }
    let store: Arc<dyn ObjectStore> = Arc::new(builder.build()?);

    let study = args
        .study
        .clone()
        .unwrap_or_else(|| format!("zeta_{}pc", (args.damping * 1e4).round() / 1e2));
    let output = ObjectPath::from(args.fem.as_str())
        .clone()
        .join(study.as_str())
        .clone()
        .join("cfd");

    // CFD cases, one per AWS Batch array job child
    let mut cases: Vec<String> = store
        .list_with_delimiter(Some(&ObjectPath::from(args.cfd.as_str())))
        .await?
        .common_prefixes
        .iter()
        .filter_map(|prefix| prefix.filename().map(String::from))
        .filter(|case| is_cfd_case(case))
        .filter(|case| args.cases.is_empty() || args.cases.contains(case))
        .collect();
    cases.sort();
    if let Ok(index) = env::var("AWS_BATCH_JOB_ARRAY_INDEX") {
        let index: usize = index.parse()?;
        cases = vec![
            cases
                .get(index)
                .ok_or_else(|| anyhow!("no CFD case #{index} in {} cases", cases.len()))?
                .clone(),
        ];
    }
    if cases.is_empty() {
        return Err(anyhow!("no CFD case found in {}", args.cfd));
    }
    println!(
        "Computing the RBMs of {} CFD cases with ζ={} into {output}",
        cases.len(),
        args.damping
    );

    // FEM
    let fem_zip = args.fem_repo.join(FEM_ZIP);
    if !fem_zip.is_file() {
        let path = ObjectPath::from(args.fem.as_str()).join(FEM_ZIP);
        println!("Downloading {path} to {fem_zip:?}");
        fs::create_dir_all(&args.fem_repo)?;
        download(&store, &path, &fem_zip).await?;
    }
    unsafe { env::set_var("FEM_REPO", &args.fem_repo) };
    // the FEM name is the last part of its path in the bucket
    let fem_tag = args
        .fem
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
    log::info!("loading the FEM {fem_tag}");
    let fem = FEM::from_env()?;

    // the RBMs are written by the Arrow logger into DATA_REPO
    let data_repo = match &args.local {
        Some(dir) => dir.join(output.as_ref()),
        None => env::temp_dir().join("windloads"),
    };
    fs::create_dir_all(&data_repo)?;
    unsafe { env::set_var("DATA_REPO", &data_repo) };

    let mut failures = vec![];
    for case in cases {
        let path = output.clone().join(case.as_str()).join(RBMS);
        let exists = match &args.local {
            Some(_) => data_repo.join(&case).join(RBMS).is_file(),
            None => store.head(&path).await.is_ok(),
        };
        if exists && !args.overwrite {
            println!("{case}: skipped, {path} already exists");
            continue;
        }
        match simulate(&args, &store, fem.clone(), &fem_tag, &case, &data_repo).await {
            Ok(file) => {
                if args.local.is_none() {
                    let bytes = fs::read(&file)?;
                    store.put(&path, bytes.into()).await?;
                    fs::remove_file(&file)?;
                }
                println!("{case}: saved {path}");
            }
            Err(e) => {
                log::error!("{case}: {e:?}");
                failures.push(case);
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{} cases failed: {failures:?}", failures.len()))
    }
}

/// Runs the wind loads simulation of a CFD case and returns the path to the RBMs file
async fn simulate(
    args: &Args,
    store: &Arc<dyn ObjectStore>,
    mut fem: FEM,
    fem_tag: &str,
    case: &str,
    data_repo: &Path,
) -> anyhow::Result<PathBuf> {
    // CFD wind loads
    let cfd_case = args.cfd_repo.join(case);
    let monitors = cfd_case.join("monitors.csv.z");
    if !monitors.is_file() {
        let path = ObjectPath::from(args.cfd.as_str())
            .clone()
            .join(case)
            .clone()
            .join("monitors.csv.z");
        log::info!("{case}: downloading {path}");
        fs::create_dir_all(&cfd_case)?;
        download(store, &path, &monitors).await?;
    }

    let n_step = args.sampling_frequency * args.duration;
    let mut cfd_loads: Sys<SigmoidCfdLoads> = CfdLoads::foh(
        cfd_case
            .to_str()
            .ok_or_else(|| anyhow!("invalid path {cfd_case:?}"))?,
        args.sampling_frequency,
    )
    .duration(args.duration as f64)
    .windloads(&mut fem, Default::default())
    .try_into()?;

    let servos = GmtServoMechanisms::<ACTUATOR_RATE, 1>::new(args.sampling_frequency as f64, fem)
        .wind_loads(WindLoads::default())
        .proportial_damping(args.damping);
    let mut gmt_servos: Sys<GmtServoMechanisms<ACTUATOR_RATE, 1>> =
        if args.no_static_gain_compensation {
            servos.no_dc_gain_compensation()
        } else {
            servos
        }
        .try_into()?;

    let metadata = HashMap::from([
        ("GMT FEM".to_string(), fem_tag.to_string()),
        ("CFD case".to_string(), case.to_string()),
        ("FEM damping".to_string(), args.damping.to_string()),
        (
            "FEM static gain compensation".to_string(),
            (!args.no_static_gain_compensation).to_string(),
        ),
        (
            "sampling frequency".to_string(),
            args.sampling_frequency.to_string(),
        ),
        (
            "mount model".to_string(),
            option_env!("MOUNT_MODEL").unwrap_or_default().to_string(),
        ),
    ]);
    fs::create_dir_all(data_repo.join(case))?;
    let filename = Path::new(case).join(RBMS);
    let mut rbms_logger: Terminator<Arrow> = Arrow::builder(n_step)
        .filename(filename.to_string_lossy().into_owned())
        .metadata(metadata)
        .build()
        .into();

    AddActorOutput::<Mount, _, _>::add_output(&mut cfd_loads)
        .build::<CFDMountWindLoads>()
        .into_input::<GmtFem>(&mut gmt_servos)?;
    AddActorOutput::<M1, _, _>::add_output(&mut cfd_loads)
        .build::<CFDM1WindLoads>()
        .into_input::<GmtFem>(&mut gmt_servos)?;
    AddActorOutput::<M2, _, _>::add_output(&mut cfd_loads)
        .build::<CFDM2WindLoads>()
        .into_input::<GmtFem>(&mut gmt_servos)?;
    AddActorOutput::<GmtFem, _, _>::add_output(&mut gmt_servos)
        .unbounded()
        .build::<M1RigidBodyMotions>()
        .log(&mut rbms_logger)?;
    AddActorOutput::<GmtFem, _, _>::add_output(&mut gmt_servos)
        .unbounded()
        .build::<M2RigidBodyMotions>()
        .log(&mut rbms_logger)?;

    println!("{case}: running {}s simulation", args.duration);
    // the RBMs are saved when the logger is dropped at the end of the run
    Model::new(vec_box!(cfd_loads, gmt_servos, rbms_logger))
        .quiet()
        .check()?
        .run()
        .await?;

    let file = data_repo.join(filename);
    if file.is_file() {
        Ok(file)
    } else {
        Err(anyhow!("{file:?} has not been written"))
    }
}

/// CFD case name: `zen<zenith>az<azimuth>_<OS|CD|CS>_<wind speed>ms`
fn is_cfd_case(case: &str) -> bool {
    let mut parts = case.split('_');
    let (Some(pointing), Some(config), Some(speed), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    pointing.len() == 10
        && pointing.starts_with("zen")
        && pointing[5..].starts_with("az")
        && ["OS", "CD", "CS"].contains(&config)
        && speed
            .strip_suffix("ms")
            .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

/// Streams an object of the bucket into a local file
async fn download(
    store: &Arc<dyn ObjectStore>,
    path: &ObjectPath,
    file: &Path,
) -> anyhow::Result<()> {
    let partial = file.with_extension("partial");
    let mut stream = store
        .get(path)
        .await
        .with_context(|| format!("failed to get {path}"))?
        .into_stream();
    let mut writer = tokio::fs::File::create(&partial).await?;
    while let Some(bytes) = stream.next().await {
        writer.write_all(&bytes?).await?;
    }
    writer.flush().await?;
    fs::rename(&partial, file)?;
    Ok(())
}
