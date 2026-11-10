#!/usr/bin/env bash
# Deploys the wind loads simulation on AWS Batch and submits a run, from the workspace root:
#   FEM_DIR=<path/to/FEM> piston-batch/deploy-windloads.sh [windloads arguments]
# e.g. the baseline (2% damping) of one case, to check the deployment against the baseline RBMs:
#   FEM_DIR=... piston-batch/deploy-windloads.sh --case zen30az000_OS_7ms
# Environment:
#   FEM_DIR  local FEM directory (with modal_state_space_model_2ndOrder.zip), compiled in the image
#   REGION   compute region [us-west-2], QUEUE Batch job queue [WindCFDJobs]
#   ARRAY    array job size (one CFD case per child) [none]
#   DOCKER   docker command [docker], e.g. DOCKER="sudo docker"
#   SKIP_BUILD=1 reuses the image already in ECR, SKIP_SUBMIT=1 only builds and registers
set -euo pipefail

REGION=${REGION:-us-west-2}
QUEUE=${QUEUE:-WindCFDJobs}
DOCKER=${DOCKER:-docker}
ROLE=piston-batch-job
REPO=gmto.im/windloads
ACCOUNT=$(aws sts get-caller-identity --query Account --output text)
REGISTRY=$ACCOUNT.dkr.ecr.$REGION.amazonaws.com
IMAGE=$REGISTRY/$REPO:latest
HERE=$(dirname "$0")

# IAM job role: read the FEM, LOM and CFD loads, write only the trade studies outputs
if ! aws iam get-role --role-name $ROLE >/dev/null 2>&1; then
  echo "Creating the IAM role $ROLE"
  aws iam create-role --role-name $ROLE \
    --description "windloads and piston-batch AWS Batch jobs" \
    --assume-role-policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ecs-tasks.amazonaws.com"},"Action":"sts:AssumeRole"}]}' >/dev/null
fi
aws iam put-role-policy --role-name $ROLE --policy-name gmto-cfd-2025-trade-studies --policy-document '{
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
}'

# Container image
if [ -z "${SKIP_BUILD:-}" ]; then
  : "${FEM_DIR:?set FEM_DIR to the local FEM directory}"
  aws ecr describe-repositories --region $REGION --repository-names $REPO >/dev/null 2>&1 ||
    aws ecr create-repository --region $REGION --repository-name $REPO >/dev/null
  aws ecr get-login-password --region $REGION | $DOCKER login --username AWS --password-stdin $REGISTRY
  # FEM zip stub with only the inputs and outputs tables, used at compile time
  mkdir -p .fem-io
  python3 - "$FEM_DIR/modal_state_space_model_2ndOrder.zip" .fem-io/modal_state_space_model_2ndOrder.zip <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1]) as src, zipfile.ZipFile(sys.argv[2], "w") as dst:
    for io in ("in", "out"):
        name = f"rust/modal_state_space_model_2ndOrder_{io}.parquet"
        dst.writestr(name, src.read(name))
PY
  $DOCKER build -f $HERE/Dockerfile.windloads -t $IMAGE .
  $DOCKER push $IMAGE
fi

# Job definition and submission
sed -e "s/<ACCOUNT>/$ACCOUNT/g" -e "s/<REGION>/$REGION/g" $HERE/job-definition-windloads.yaml > /tmp/windloads-job-definition.yaml
aws batch register-job-definition --region $REGION --cli-input-yaml file:///tmp/windloads-job-definition.yaml \
  --query '[jobDefinitionName,revision]' --output text
if [ -n "${SKIP_SUBMIT:-}" ]; then exit 0; fi
COMMAND=$(printf '%s\n' "$@" | python3 -c 'import json,sys; print(json.dumps([a for a in sys.stdin.read().split("\n") if a]))')
ARGS=(--region $REGION --job-name "windloads-$(date +%Y%m%d-%H%M%S)" --job-queue $QUEUE --job-definition windloads
      --container-overrides "{\"command\":$COMMAND}")
if [ -n "${ARRAY:-}" ]; then ARGS+=(--array-properties size=$ARRAY); fi
aws batch submit-job "${ARGS[@]}" --query '[jobName,jobId]' --output text
echo "aws batch describe-jobs --region $REGION --jobs <jobId>"
