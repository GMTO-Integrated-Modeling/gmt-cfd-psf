#  Wind loads segment piston batch

Data pipeline for trade studies of the GMT wind loads on the FEM (FEM model, damping coefficient, ...):

1. `windloads` (feature `windloads`): applies the CFD wind loads to the GMT FEM in closed loop with
   the mount, M1 and M2 servo-mechanisms and saves the M1 & M2 rigid body motions (RBMs) at 1kHz:
   `<FEM>/<STUDY>/cfd/<case>/m1_m2_rbms.parquet`,
2. `piston-batch`: applies the FSM and ASM rejection transfer functions to the RBMs and computes
   the segment piston time series, open loop and with the FSM or ASM loops.

The baseline RBMs (FEM `20250506_1715_…_M1_actDamping`, 2% damping) are in `<FEM>/cfd/<case>/m1_m2_rbms.parquet`
of the `gmto.cfd.2025` S3 bucket.

## FSM and ASM rejection

The M2 feedback loops ([rejection.rs](src/rejection.rs)) are ported from
[wind2optics](https://github.com/GMTO-Integrated-Modeling/wind2optics) and run at the RBMs sampling rate:

| loop | sensor | latency | controller | M2 dynamics | correction |
|------|--------|---------|------------|-------------|------------|
| FSM tip-tilt | LOM segment tip-tilt, 5 samples average | 6 samples | double integrator (0.1376, 0.0719) | 25Hz, ζ=0.707 | M2 Rx, Ry |
| ASM tip-tilt | LOM segment tip-tilt, 2 samples average | 1 sample | double integrator (0.35, 0.2738) | 800Hz, ζ=0.7 | M2 Rx, Ry |
| ASM piston, edge sensors | segment piston, 2 samples average | 0 | proportional (0.8) | 800Hz, ζ=0.75 | segment piston |
| ASM piston, optical sensor | residual segment piston, 100 samples average | 50 samples | integral (0.5) | 800Hz, ζ=0.75 | segment piston |

With the baseline RBMs of `zen30az000_OS_7ms`, the FSM corrected RBMs are identical to the wind2optics
`m1_m2_rbms.FSM.parquet` and the ASM corrected RBMs match `m1_m2_rbms.ASM.parquet` within 1e-5
(the ASM piston loop uses the LOM segment piston instead of the segment wavefront mean).

## Wind loads simulation

The FEM inputs and outputs are generated at compile time from the FEM in `FEM_REPO`;
another FEM with the same inputs and outputs can be used at runtime without recompiling:

```shell
export MOUNT_MODEL=MOUNT_FDR_1kHz FEM_REPO=<path/to/FEM>
cargo r -r -p piston-batch --features windloads --bin windloads -- \
  --fem FEM/20250506_1715_zen_30_M1_202110_FSM_202305_Mount_202305_pier_202411_M1_actDamping \
  --damping 0.005 --case zen30az000_OS_7ms --local trade
```
 - `--damping`: FEM proportional damping coefficient (default: 0.02, as the baseline),
 - `--study`: output sub-directory of `<FEM>` (default: `zeta_<damping in %>pc`, e.g. `zeta_0.5pc`),
 - `--no-static-gain-compensation`: disables the FEM static gain compensation,
 - `--duration`: simulation duration in seconds (default: 400, as the baseline), the CFD loads are the last `duration` seconds of the CFD record,
 - `--fem-repo`: FEM directory, the FEM is downloaded from `<FEM>/modal_state_space_model_2ndOrder.zip` if missing,
 - `--cfd-repo`: CFD loads cache directory, `monitors.csv.z` is downloaded from `CASES/<case>/` if missing,
 - the cases are processed one after the other, or the case `#AWS_BATCH_JOB_ARRAY_INDEX` only in an AWS Batch array job,
 - the feature `cuda` solves the FEM state space model on the GPU.

On 20 CPU threads, the simulation runs 6.7 times slower than real time (~45mn for 400s) with 9GB of memory.

Then the segment piston of the trade study is computed with
```shell
cargo r -r -p piston-batch -- --fem <FEM>/<STUDY> --sensitivities optical_sensitivities.rs.bin
```

## Segment piston

Outputs:
 - `<FEM>/cfd/<case>/segment_piston.parquet`: columns `time` [s], `S1`..`S7` (open loop), `S1_FSM`..`S7_FSM` and `S1_ASM`..`S7_ASM` [m]
 - `<FEM>/segment_piston_summary.parquet`: columns `case`, `segment`, `mean`, `std`, `mean_FSM`, `std_FSM`, `mean_ASM`, `std_ASM` [m]
 - with `--save-rbms`, the corrected RBMs `<FEM>/cfd/<case>/m1_m2_rbms.fsm-rtf.parquet` and `m1_m2_rbms.asm-rtf.parquet`

The first 5000 samples (wind loads ramp-up) are discarded (`--skip`).
`--loop fsm` (or `--loop asm`) restricts the M2 loops, by default both FSM and ASM are computed.

### Local run

The bucket settings and AWS credentials are read from `.env_s3` (see [cli](../cli/README.md)),
`--local` writes the outputs to a local directory instead of the bucket:

```shell
cargo r -r -p piston-batch -- \
  --sensitivities optical_sensitivities.rs.bin \
  --case zen30az000_OS_7ms --local piston
```

## AWS Batch

The wind loads simulation is deployed on the `WindCFDJobs` queue (us-west-2) with
[deploy-windloads.sh](deploy-windloads.sh), which creates the IAM job role and the ECR repository,
builds and pushes the image, registers the job definition and submits the job:
```shell
FEM_DIR=<path/to/FEM> DOCKER="sudo docker" piston-batch/deploy-windloads.sh --case zen30az000_OS_7ms
FEM_DIR=<path/to/FEM> SKIP_BUILD=1 ARRAY=63 piston-batch/deploy-windloads.sh --damping 0.005
```

The steps below detail the manual deployment.

1. Upload the LOM optical sensitivities to the bucket:
   ```shell
   aws s3 cp optical_sensitivities.rs.bin s3://gmto.cfd.2025/LOM/optical_sensitivities.rs.bin
   ```
2. Build the containers from the workspace root and push them to ECR:
   ```shell
   aws ecr create-repository --repository-name piston-batch --region us-east-1
   aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com
   docker build -f piston-batch/Dockerfile -t <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com/piston-batch:latest .
   docker push <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com/piston-batch:latest
   ```
   and for the wind loads simulation (it compiles the FEM inputs and outputs in the image):
   ```shell
   FEM_DIR=<path/to/FEM> SKIP_SUBMIT=1 piston-batch/deploy-windloads.sh
   ```
3. Create the job IAM role `piston-batch-job` (trusted by `ecs-tasks.amazonaws.com`) with the policy:
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       { "Effect": "Allow", "Action": "s3:ListBucket", "Resource": "arn:aws:s3:::gmto.cfd.2025" },
       { "Effect": "Allow", "Action": "s3:GetObject",
         "Resource": ["arn:aws:s3:::gmto.cfd.2025/FEM/*", "arn:aws:s3:::gmto.cfd.2025/LOM/*",
                      "arn:aws:s3:::gmto.cfd.2025/CASES/*/monitors.csv.z"] },
       { "Effect": "Allow", "Action": "s3:PutObject",
         "Resource": ["arn:aws:s3:::gmto.cfd.2025/FEM/*/segment_piston*.parquet",
                      "arn:aws:s3:::gmto.cfd.2025/FEM/*/cfd/*/m1_m2_rbms.*-rtf.parquet",
                      "arn:aws:s3:::gmto.cfd.2025/FEM/*/*/cfd/*/m1_m2_rbms.parquet"] }
     ]
   }
   ```
   The last statement lets `windloads` write only in a trade study sub-directory, never over the baseline RBMs.
4. Register the job definitions and submit the jobs:
   ```shell
   aws batch register-job-definition --cli-input-yaml file://piston-batch/job-definition.yaml
   aws batch register-job-definition --cli-input-yaml file://piston-batch/job-definition-windloads.yaml
   # 63 CFD cases, one per array child
   aws batch submit-job --job-name windloads-zeta-0.5pc --job-queue <EC2_QUEUE> --job-definition windloads \
     --array-properties size=63 --container-overrides '{"command":["--damping","0.005"]}'
   # then, once completed
   aws batch submit-job --job-name piston-batch --job-queue <FARGATE_QUEUE> --job-definition piston-batch \
     --container-overrides '{"environment":[{"name":"FEM","value":"FEM/<FEM>/zeta_0.5pc"}]}'
   ```
   A subset of cases is processed by overriding the command, e.g.
   `--container-overrides '{"command":["--case","zen30az000_OS_7ms"]}'`.
