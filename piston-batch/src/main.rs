/*!
# Wind loads segment piston batch

Computes the GMT segment piston time series in the exit pupil from the wind loads
M1 & M2 rigid body motions (RBMs) sampled at 1kHz, without (`m1_m2_rbms.parquet`)
and with (`m1_m2_rbms.FSM.parquet`) FSM tip-tilt compensation, for all the CFD cases
of a FEM model.

The RBMs are converted into segment piston with the GMT linear optical model
([gmt_lom]).
The first `--skip` samples (wind loads ramp-up) are discarded.

Outputs (in the S3 bucket, or in `--local` directory):
 - `<FEM>/cfd/<case>/segment_piston.parquet`: segment piston time series in meter,
   columns `time` [s], `S1`..`S7` (no FSM) and `S1_FSM`..`S7_FSM` (with FSM)
 - `<FEM>/segment_piston_summary.parquet`: segment piston mean and standard deviation
   in meter, per case and segment

The S3 bucket is set with the environment variables `BUCKET` and `REGION`
(or `AWS_BUCKET` and `AWS_REGION`), possibly from a `.env_s3` file.

```shell
cargo r -r -p piston-batch -- --help
```
*/

use std::{collections::HashMap, env, path::Path, sync::Arc};

use anyhow::{Context, anyhow};
use arrow::{
    array::{ArrayRef, Float32Array, Float64Array, StringArray, UInt8Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use clap::Parser;
use futures::{StreamExt, stream};
use gmt_lom::{LOM, RigidBodyMotions, Table};
use object_store::{ObjectStore, PutPayload, path::Path as ObjectPath};
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, ZstdLevel},
    file::properties::WriterProperties,
};

const N_SEGMENT: usize = 7;

#[derive(Parser)]
#[command(name = "piston-batch")]
#[command(about = "Segment piston from wind loads M1 & M2 RBMs, without and with FSM")]
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
}

/// Segment piston mean and standard deviation of a CFD case
struct Stats {
    case: String,
    mean: [Vec<f64>; 2],
    std: [Vec<f64>; 2],
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
    let lom = Arc::new(LOM::try_from(bytes.as_ref())?);
    drop(bytes);

    // CFD cases
    let fem = ObjectPath::from(args.fem.as_str());
    let mut cases: Vec<String> = store
        .list_with_delimiter(Some(&fem.child("cfd")))
        .await?
        .common_prefixes
        .iter()
        .filter_map(|prefix| prefix.filename().map(String::from))
        .filter(|case| args.cases.is_empty() || args.cases.contains(case))
        .collect();
    cases.sort();
    if cases.is_empty() {
        return Err(anyhow!("no CFD case found in {}", fem.child("cfd")));
    }
    println!("Processing {} CFD cases from {fem}", cases.len());

    let results: Vec<_> = stream::iter(cases)
        .map(|case| {
            let store = store.clone();
            let output = output.clone();
            let lom = lom.clone();
            let case_path = fem.child("cfd").child(case.as_str());
            let (skip, sampling_frequency) = (args.skip, args.sampling_frequency);
            tokio::spawn(async move {
                let stats = process_case(
                    store,
                    output,
                    &lom,
                    &case,
                    case_path,
                    skip,
                    sampling_frequency,
                )
                .await;
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
                println!(
                    "{case}: piston std [nm] no FSM {:?}, with FSM {:?}",
                    nm(&stats.std[0]),
                    nm(&stats.std[1])
                );
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
        let path = fem.child("segment_piston_summary.parquet");
        output
            .put(&path, write_parquet(summary_record(&summary)?)?)
            .await?;
        println!("Saved summary of {} cases to {path}", summary.len());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{} cases failed: {failures:?}", failures.len()))
    }
}

/// Computes and saves the segment piston without and with FSM for a CFD case
async fn process_case(
    store: Arc<dyn ObjectStore>,
    output: Arc<dyn ObjectStore>,
    lom: &LOM,
    case: &str,
    case_path: ObjectPath,
    skip: usize,
    sampling_frequency: f64,
) -> anyhow::Result<Stats> {
    let mut lom = lom.clone();
    let mut time: Option<Vec<f64>> = None;
    let mut pistons = vec![];
    for object in ["m1_m2_rbms.parquet", "m1_m2_rbms.FSM.parquet"] {
        let path = case_path.child(object);
        log::info!("{case}: loading {path}");
        let table = Table::from_stored_parquet(store.clone(), path.clone())
            .await
            .with_context(|| format!("failed to load {path}"))?;
        lom.rbm = RigidBodyMotions::from_table(
            &table,
            Some("M1RigidBodyMotions"),
            Some("M2RigidBodyMotions"),
        )?;
        drop(table);
        let n = lom.len();
        if n <= skip {
            return Err(anyhow!("{path}: {n} samples, not more than {skip} to skip"));
        }
        // RBMs sample indices
        let t: Vec<f64> = lom.time().into_iter().skip(skip).collect();
        match &time {
            Some(time) if *time != t => {
                return Err(anyhow!("{path}: sample indices differ from no FSM RBMs"));
            }
            Some(_) => (),
            None => time = Some(t),
        }
        // [p1,...,p7] for each sample
        let mut piston: Vec<f64> = lom.segment_piston().into();
        pistons.push(piston.split_off(skip * N_SEGMENT));
    }
    let time: Vec<f64> = time
        .unwrap()
        .into_iter()
        .map(|k| k / sampling_frequency)
        .collect();

    let (mean, std): (Vec<_>, Vec<_>) = pistons.iter().map(|p| mean_std(p)).unzip();

    let path = case_path.child("segment_piston.parquet");
    output
        .put(
            &path,
            write_parquet(piston_record(&time, &pistons, skip, sampling_frequency)?)?,
        )
        .await?;
    log::info!("{case}: saved {path}");

    Ok(Stats {
        case: case.to_string(),
        mean: [mean[0].clone(), mean[1].clone()],
        std: [std[0].clone(), std[1].clone()],
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

fn nm(values: &[f64]) -> Vec<f64> {
    values.iter().map(|x| (x * 1e10).round() / 10.).collect()
}

/// Segment piston time series record: `time`, `S1`..`S7`, `S1_FSM`..`S7_FSM`
fn piston_record(
    time: &[f64],
    pistons: &[Vec<f64>],
    skip: usize,
    sampling_frequency: f64,
) -> anyhow::Result<RecordBatch> {
    let mut fields = vec![Field::new("time", DataType::Float64, false)];
    let mut columns: Vec<ArrayRef> = vec![Arc::new(Float64Array::from(time.to_vec()))];
    for (piston, suffix) in pistons.iter().zip(["", "_FSM"]) {
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

/// Summary record: `case`, `segment`, `mean`, `std`, `mean_FSM`, `std_FSM`
fn summary_record(summary: &[Stats]) -> anyhow::Result<RecordBatch> {
    let case = StringArray::from(
        summary
            .iter()
            .flat_map(|s| std::iter::repeat_n(s.case.as_str(), N_SEGMENT))
            .collect::<Vec<_>>(),
    );
    let segment = UInt8Array::from_iter_values(
        summary
            .iter()
            .flat_map(|_| 1..=N_SEGMENT as u8),
    );
    let column = |f: fn(&Stats) -> &Vec<f64>| -> ArrayRef {
        Arc::new(Float64Array::from_iter_values(
            summary.iter().flat_map(|s| f(s).iter().cloned()),
        ))
    };
    let schema = Schema::new_with_metadata(
        vec![
            Field::new("case", DataType::Utf8, false),
            Field::new("segment", DataType::UInt8, false),
            Field::new("mean", DataType::Float64, false),
            Field::new("std", DataType::Float64, false),
            Field::new("mean_FSM", DataType::Float64, false),
            Field::new("std_FSM", DataType::Float64, false),
        ],
        HashMap::from([("units".to_string(), "m".to_string())]),
    );
    Ok(RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(case),
            Arc::new(segment),
            column(|s| &s.mean[0]),
            column(|s| &s.std[0]),
            column(|s| &s.mean[1]),
            column(|s| &s.std[1]),
        ],
    )?)
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
