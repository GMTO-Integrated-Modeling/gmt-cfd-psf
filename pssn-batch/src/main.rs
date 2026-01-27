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

use std::{collections::BTreeMap, env, fs::File, sync::Arc};

use indicatif::{MultiProgress, ProgressBar};
use object_store::{ObjectStore, path::Path as ObjectPath};
use parse_monitors::{
    CFD_YEAR,
    cfd::{Baseline, BaselineTrait},
};
use psf::{GmtOpticalModel, StorePath};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    dotenvy::from_filename(".env_s3")?;

    let store: Arc<dyn ObjectStore> = Arc::new(
        object_store::aws::AmazonS3Builder::from_env()
            .with_region(env::var("REGION")?)
            .with_bucket_name(env::var("BUCKET")?)
            .build()?,
    );

    let mut pssns = BTreeMap::<String, f64>::new();
    for cfd_case_chunk in Baseline::<CFD_YEAR>::default()
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(8)
    {
        let mpb = MultiProgress::new();
        let mut h = vec![];
        for cfd_case in cfd_case_chunk.into_iter().cloned() {
            let clone_store = store.clone();
            // let cfd_case = cfd_case_.clone();
            let pb = mpb.add(ProgressBar::new_spinner().with_message(cfd_case.to_string()));
            h.push(tokio::spawn(async move {
                // println!("{}", cfd_case);
                // Setup GMT optics and imaging
                let gmt = GmtOpticalModel::new()?;

                let gmt = {
                    let cfd_path =
                        ObjectPath::from(Baseline::<CFD_YEAR>::path()?.to_str().unwrap())
                            .join(cfd_case.to_string());
                    gmt.domeseeing(clone_store.clone(), cfd_path).await?
                };

                let mut gmt = {
                    let object = "m1_m2_rbms.parquet";
                    let rbms_path = ObjectPath::new(env::var("FEM")?)
                        .join("cfd")
                        .join(cfd_case.to_string())
                        .join(object);
                    gmt.windloads(clone_store, rbms_path).await?
                };

                while gmt.ray_trace_all().is_some() {
                    pb.tick();
                }
                pb.finish();
                Result::<_, anyhow::Error>::Ok((cfd_case.to_string(), gmt.compute_pssn()))
            }));
        }
        mpb.clear()?;
        for h in h {
            let (case, pssn) = h.await??;
            pssns.insert(case, pssn);
        }
    }
    serde_pickle::to_writer(
        &mut File::create("cfd_domeseeing-windloads_v-pssn.pkl")?,
        &pssns,
        Default::default(),
    )?;
    Ok(())
}
