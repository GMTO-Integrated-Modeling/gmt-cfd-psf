#  Wind loads segment piston batch

Computes the segment piston time series (1kHz) from the wind loads M1 & M2 rigid body motions,
without (`m1_m2_rbms.parquet`) and with (`m1_m2_rbms.FSM.parquet`) FSM compensation,
for all the CFD cases of a FEM model in the `gmto.cfd.2025` S3 bucket.

Outputs:
 - `<FEM>/cfd/<case>/segment_piston.parquet`: columns `time` [s], `S1`..`S7` and `S1_FSM`..`S7_FSM` [m]
 - `<FEM>/segment_piston_summary.parquet`: columns `case`, `segment`, `mean`, `std`, `mean_FSM`, `std_FSM` [m]

The first 5000 samples (wind loads ramp-up) are discarded (`--skip`).

## Local run

The bucket settings and AWS credentials are read from `.env_s3` (see [cli](../cli/README.md)),
`--local` writes the outputs to a local directory instead of the bucket:

```shell
cargo r -r -p piston-batch -- \
  --sensitivities optical_sensitivities.rs.bin \
  --case zen30az000_OS_7ms --local piston
```

## AWS Batch

1. Upload the LOM optical sensitivities to the bucket:
   ```shell
   aws s3 cp optical_sensitivities.rs.bin s3://gmto.cfd.2025/LOM/optical_sensitivities.rs.bin
   ```
2. Build the container from the workspace root and push it to ECR:
   ```shell
   aws ecr create-repository --repository-name piston-batch --region us-east-1
   aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com
   docker build -f piston-batch/Dockerfile -t <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com/piston-batch:latest .
   docker push <ACCOUNT>.dkr.ecr.us-east-1.amazonaws.com/piston-batch:latest
   ```
3. Create the job IAM role `piston-batch-job` (trusted by `ecs-tasks.amazonaws.com`) with the policy:
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       { "Effect": "Allow", "Action": "s3:ListBucket", "Resource": "arn:aws:s3:::gmto.cfd.2025" },
       { "Effect": "Allow", "Action": "s3:GetObject",
         "Resource": ["arn:aws:s3:::gmto.cfd.2025/FEM/*", "arn:aws:s3:::gmto.cfd.2025/LOM/*"] },
       { "Effect": "Allow", "Action": "s3:PutObject",
         "Resource": "arn:aws:s3:::gmto.cfd.2025/FEM/*/segment_piston*.parquet" }
     ]
   }
   ```
4. Register the [job definition](job-definition.yaml) and submit the job to a Fargate job queue:
   ```shell
   aws batch register-job-definition --cli-input-yaml file://piston-batch/job-definition.yaml
   aws batch submit-job --job-name piston-batch --job-queue <FARGATE_QUEUE> --job-definition piston-batch
   ```
   A subset of cases is processed by overriding the command, e.g.
   `--container-overrides '{"command":["--case","zen30az000_OS_7ms"]}'`.
