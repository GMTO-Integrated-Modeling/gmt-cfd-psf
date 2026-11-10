/*!
# Wind loads segment piston batch

Computes the GMT segment piston time series in the exit pupil from the wind loads
M1 & M2 rigid body motions (RBMs) sampled at 1kHz (`m1_m2_rbms.parquet`), without
M2 correction, with the FSM tip-tilt loop and with the ASM tip-tilt and piston loops
(see [rejection]), for all the CFD cases of a FEM model.

The RBMs are converted into segment piston with the GMT linear optical model
([gmt_lom]).
The first `--skip` samples (wind loads ramp-up) are discarded.

Outputs (in the S3 bucket, or in `--local` directory):
 - `<FEM>/cfd/<case>/segment_piston.parquet`: segment piston time series in meter,
   columns `time` [s], `S1`..`S7` (open loop), `S1_FSM`..`S7_FSM` (FSM)
   and `S1_ASM`..`S7_ASM` (ASM)
 - `<FEM>/segment_piston_summary.parquet`: segment piston mean and standard deviation
   in meter, per case and segment
 - with `--save-rbms`, `<FEM>/cfd/<case>/m1_m2_rbms.fsm-rtf.parquet` and
   `m1_m2_rbms.asm-rtf.parquet`: the RBMs corrected by the FSM and ASM loops

The S3 bucket is set with the environment variables `BUCKET` and `REGION`
(or `AWS_BUCKET` and `AWS_REGION`), possibly from a `.env_s3` file.

```shell
cargo r -r -p piston-batch -- --help
```
*/

use std::{collections::HashMap, env, path::Path, sync::Arc};

mod rejection;
use rejection::{M2, Rejection};

use anyhow::{Context, anyhow};
use arrow::{
    array::{ArrayRef, Float32Array, Float64Array, StringArray, UInt8Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use clap::Parser;
use futures::{StreamExt, stream};
use gmt_lom::{OpticalSensitivities, RigidBodyMotions, Table};
use nalgebra as na;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, path::Path as ObjectPath};
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, ZstdLevel},
    file::properties::WriterProperties,
};

const N_SEGMENT: usize = 7;

#[derive(Parser)]
#[command(name = "piston-batch")]
#[command(
    about = "Segment piston from wind loads M1 & M2 RBMs, without M2 correction and with the FSM or ASM loops"
)]
struct Args {
    /// FEM model path in the bucket, the CFD cases are in `<FEM>/cfd`
    #[arg(long, env = "FEM")]
    fem: String,

    /// LOM optical sensitivities: a local file or an object path in the bucket
    #[arg(long, env = "LOM_SENSITIVITIES")]
    sensitivities: String,

    /// Restricts the computation to these CFD cases (e.g. zen30az000_OS_7ms)
    #[arg(long = "case")]
    cases: Vec<String>,

    /// Number of CFD cases processed concurrently
    #[arg(short, long, default_value_t = 4)]
    concurrency: usize,

    /// Number of initial samples discarded (wind loads ramp-up)
    #[arg(long, default_value_t = 5000)]
    skip: usize,

    /// RBMs sampling frequency in Hz
    #[arg(long, default_value_t = 1000f64)]
    sampling_frequency: f64,

    /// Writes the outputs to this local directory instead of the bucket
    #[arg(long)]
    local: Option<String>,

    /// M2 loops applied to the open loop RBMs
    #[arg(
        long = "loop",
        value_enum,
        value_delimiter = ',',
        default_value = "fsm,asm"
    )]
    loops: Vec<M2>,

    /// Also saves the RBMs corrected by the M2 loops
    #[arg(long)]
    save_rbms: bool,
}

/// Segment piston mean and standard deviation of a CFD case
struct Stats {
    case: String,
    mean: Vec<Vec<f64>>,
    std: Vec<Vec<f64>>,
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
    let output: Arc<dyn ObjectStore> = match &args.local {
        Some(dir) => {
            std::fs::create_dir_all(dir)?;
            Arc::new(object_store::local::LocalFileSystem::new_with_prefix(dir)?)
        }
        None => store.clone(),
    };

    // Optical sensitivities
    let bytes = if Path::new(&args.sensitivities).is_file() {
        std::fs::read(&args.sensitivities)?.into()
    } else {
        store
            .get(&ObjectPath::from(args.sensitivities.as_str()))
            .await
            .with_context(|| format!("failed to get {}", args.sensitivities))?
            .bytes()
            .await?
    };
    let sensitivities: OpticalSensitivities = bincode::deserialize(bytes.as_ref())?;
    drop(bytes);
    let rejection = Arc::new(Rejection::new(&sensitivities)?);
    drop(sensitivities);

    // CFD cases
    let fem = ObjectPath::from(args.fem.as_str());
    let mut cases: Vec<String> = store
        .list_with_delimiter(Some(&fem.clone().join("cfd")))
        .await?
        .common_prefixes
        .iter()
        .filter_map(|prefix| prefix.filename().map(String::from))
        .filter(|case| args.cases.is_empty() || args.cases.contains(case))
        .collect();
    cases.sort();
    if cases.is_empty() {
        return Err(anyhow!("no CFD case found in {}", fem.clone().join("cfd")));
    }
    println!("Processing {} CFD cases from {fem}", cases.len());

    let results: Vec<_> = stream::iter(cases)
        .map(|case| {
            let store = store.clone();
            let output = output.clone();
            let rejection = rejection.clone();
            let case_path = fem.clone().join("cfd").join(case.as_str());
            let options = Options {
                skip: args.skip,
                sampling_frequency: args.sampling_frequency,
                save_rbms: args.save_rbms,
                loops: args.loops.clone(),
            };
            tokio::spawn(async move {
                let stats =
                    process_case(store, output, &rejection, &case, case_path, options).await;
                (case, stats)
            })
        })
        .buffer_unordered(args.concurrency)
        .collect()
        .await;

    let mut summary = vec![];
    let mut failures = vec![];
    for result in results {
        let (case, stats) = result?;
        match stats {
            Ok(stats) => {
                let std: Vec<_> = ["open loop".to_string()]
                    .into_iter()
                    .chain(args.loops.iter().map(|m2| format!("{m2:?}")))
                    .zip(&stats.std)
                    .map(|(name, std)| format!("{name} {:?}", nm(std)))
                    .collect();
                println!("{case}: piston std [nm] {}", std.join(", "));
                summary.push(stats);
            }
            Err(e) => {
                log::error!("{case}: {e:?}");
                failures.push(case);
            }
        }
    }
    summary.sort_by(|a, b| a.case.cmp(&b.case));

    if !summary.is_empty() {
        let path = fem.clone().join("segment_piston_summary.parquet");
        output
            .put(
                &path,
                write_parquet(summary_record(&summary, &suffixes(&args.loops))?)?,
            )
            .await?;
        println!("Saved summary of {} cases to {path}", summary.len());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{} cases failed: {failures:?}", failures.len()))
    }
}

#[derive(Clone)]
struct Options {
    skip: usize,
    sampling_frequency: f64,
    save_rbms: bool,
    loops: Vec<M2>,
}

/// Computes and saves the segment piston, open loop and with the M2 loops,
/// for a CFD case
async fn process_case(
    store: Arc<dyn ObjectStore>,
    output: Arc<dyn ObjectStore>,
    rejection: &Rejection,
    case: &str,
    case_path: ObjectPath,
    options: Options,
) -> anyhow::Result<Stats> {
    let Options {
        skip,
        sampling_frequency,
        save_rbms,
        loops,
    } = options;
    let path = case_path.clone().join("m1_m2_rbms.parquet");
    log::info!("{case}: loading {path}");
    let table = Table::from_stored_parquet(store.clone(), path.clone())
        .await
        .with_context(|| format!("failed to load {path}"))?;
    let rbm = RigidBodyMotions::from_table(
        &table,
        Some("M1RigidBodyMotions"),
        Some("M2RigidBodyMotions"),
    )?;
    drop(table);
    let n = rbm.len();
    if n <= skip {
        return Err(anyhow!("{path}: {n} samples, not more than {skip} to skip"));
    }
    // RBMs sample indices
    let time: Vec<f64> = rbm
        .time()
        .into_iter()
        .skip(skip)
        .map(|k| k / sampling_frequency)
        .collect();
    let rbm = rbm.into_data();

    // [p1,...,p7] for each sample, discarding the first `skip` samples
    let trim = |piston: na::DMatrix<f64>| -> Vec<f64> {
        piston.columns(skip, n - skip).iter().copied().collect()
    };
    let mut pistons = vec![trim(rejection.segment_piston(&rbm))];
    let mut rbms = vec![];
    for &m2 in &loops {
        log::info!("{case}: {m2:?} loops");
        let mut rbm = rbm.clone();
        rejection.tiptilt_loop(&mut rbm, m2);
        let piston = match m2 {
            M2::FSM => rejection.segment_piston(&rbm),
            M2::ASM => {
                let piston = rejection.piston_loop(&rbm);
                if save_rbms {
                    rejection.fit_m2_piston(&mut rbm, &piston)?;
                }
                piston
            }
        };
        pistons.push(trim(piston));
        if save_rbms {
            rbms.push((m2, rbm));
        }
    }
    drop(rbm);

    let (mean, std): (Vec<_>, Vec<_>) = pistons.iter().map(|p| mean_std(p)).unzip();

    let path = case_path.clone().join("segment_piston.parquet");
    output
        .put(
            &path,
            write_parquet(piston_record(
                &time,
                &pistons,
                &suffixes(&loops),
                skip,
                sampling_frequency,
            )?)?,
        )
        .await?;
    log::info!("{case}: saved {path}");

    for (m2, rbm) in rbms {
        let path = case_path.clone().join(format!(
            "m1_m2_rbms.{}-rtf.parquet",
            format!("{m2:?}").to_lowercase()
        ));
        let bytes = RigidBodyMotions::from(rbm)
            .to_table(Some("M1RigidBodyMotions"), Some("M2RigidBodyMotions"))?
            .to_mem()?;
        output.put(&path, bytes.into()).await?;
        log::info!("{case}: saved {path}");
    }

    Ok(Stats {
        case: case.to_string(),
        mean,
        std,
    })
}

/// Segment piston mean and standard deviation
fn mean_std(piston: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = (piston.len() / N_SEGMENT) as f64;
    (0..N_SEGMENT)
        .map(|i| {
            let p = piston.iter().skip(i).step_by(N_SEGMENT);
            let mean = p.clone().sum::<f64>() / n;
            let var = p.map(|x| (x - mean).powi(2)).sum::<f64>() / n;
            (mean, var.sqrt())
        })
        .unzip()
}

/// Time series suffixes: open loop and M2 loops
fn suffixes(loops: &[M2]) -> Vec<&'static str> {
    [""].into_iter()
        .chain(loops.iter().map(M2::suffix))
        .collect()
}

fn nm(values: &[f64]) -> Vec<f64> {
    values.iter().map(|x| (x * 1e10).round() / 10.).collect()
}

/// Segment piston time series record: `time`, `S1`..`S7` and `S1_<M2>`..`S7_<M2>` for each M2 loop
fn piston_record(
    time: &[f64],
    pistons: &[Vec<f64>],
    suffixes: &[&str],
    skip: usize,
    sampling_frequency: f64,
) -> anyhow::Result<RecordBatch> {
    let mut fields = vec![Field::new("time", DataType::Float64, false)];
    let mut columns: Vec<ArrayRef> = vec![Arc::new(Float64Array::from(time.to_vec()))];
    for (piston, suffix) in pistons.iter().zip(suffixes) {
        for i in 0..N_SEGMENT {
            fields.push(Field::new(
                format!("S{}{suffix}", i + 1),
                DataType::Float32,
                false,
            ));
            columns.push(Arc::new(Float32Array::from_iter_values(
                piston.iter().skip(i).step_by(N_SEGMENT).map(|&x| x as f32),
            )));
        }
    }
    let metadata = HashMap::from([
        ("units".to_string(), "time: s, piston: m".to_string()),
        (
            "sampling_frequency".to_string(),
            sampling_frequency.to_string(),
        ),
        ("skipped_samples".to_string(), skip.to_string()),
    ]);
    Ok(RecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(fields, metadata)),
        columns,
    )?)
}

/// Summary record: `case`, `segment`, `mean`, `std` and `mean_<M2>`, `std_<M2>` for each M2 loop
fn summary_record(summary: &[Stats], suffixes: &[&str]) -> anyhow::Result<RecordBatch> {
    let case = StringArray::from(
        summary
            .iter()
            .flat_map(|s| std::iter::repeat_n(s.case.as_str(), N_SEGMENT))
            .collect::<Vec<_>>(),
    );
    let segment = UInt8Array::from_iter_values(summary.iter().flat_map(|_| 1..=N_SEGMENT as u8));
    let mut fields = vec![
        Field::new("case", DataType::Utf8, false),
        Field::new("segment", DataType::UInt8, false),
    ];
    let mut columns: Vec<ArrayRef> = vec![Arc::new(case), Arc::new(segment)];
    for (k, suffix) in suffixes.iter().enumerate() {
        let stats: [(&str, fn(&Stats) -> &Vec<Vec<f64>>); 2] =
            [("mean", |s| &s.mean), ("std", |s| &s.std)];
        for (name, stat) in stats {
            fields.push(Field::new(
                format!("{name}{suffix}"),
                DataType::Float64,
                false,
            ));
            columns.push(Arc::new(Float64Array::from_iter_values(
                summary.iter().flat_map(|s| stat(s)[k].iter().copied()),
            )));
        }
    }
    let schema = Schema::new_with_metadata(
        fields,
        HashMap::from([("units".to_string(), "m".to_string())]),
    );
    Ok(RecordBatch::try_new(Arc::new(schema), columns)?)
}

fn write_parquet(record: RecordBatch) -> anyhow::Result<PutPayload> {
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();
    let mut buffer = vec![];
    let mut writer = ArrowWriter::try_new(&mut buffer, record.schema(), Some(props))?;
    writer.write(&record)?;
    writer.close()?;
    Ok(buffer.into())
}
