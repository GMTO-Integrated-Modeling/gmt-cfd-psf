#!/usr/bin/env bash
# Deploys the segment piston on AWS Batch and submits a run, from the workspace root:
#   FEM=FEM/<FEM>/<STUDY> piston-batch/deploy-piston-batch.sh [piston-batch arguments]
# e.g. after the wind loads array job <jobId> of a trade study:
#   FEM=FEM/<FEM>/zeta_1pc DEPENDS_ON=<jobId> DOCKER="sudo docker" piston-batch/deploy-piston-batch.sh
# Environment:
#   FEM         trade study path in the bucket, the CFD cases are in <FEM>/cfd [job definition default]
#   DEPENDS_ON  job ID the run waits for, e.g. the wind loads array job [none]
#   REGION      compute region [us-west-2], QUEUE Batch job queue [WindCFDJobs]
#   DOCKER      docker command [docker], e.g. DOCKER="sudo docker"
#   SKIP_BUILD=1 reuses the image already in ECR, SKIP_SUBMIT=1 only builds and registers
# The IAM job role piston-batch-job is created by deploy-windloads.sh
set -euo pipefail

REGION=${REGION:-us-west-2}
QUEUE=${QUEUE:-WindCFDJobs}
DOCKER=${DOCKER:-docker}
REPO=gmto.im/piston-batch
ACCOUNT=$(aws sts get-caller-identity --query Account --output text)
REGISTRY=$ACCOUNT.dkr.ecr.$REGION.amazonaws.com
IMAGE=$REGISTRY/$REPO:latest
HERE=$(dirname "$0")

# Container image
if [ -z "${SKIP_BUILD:-}" ]; then
  aws ecr describe-repositories --region $REGION --repository-names $REPO >/dev/null 2>&1 ||
    aws ecr create-repository --region $REGION --repository-name $REPO >/dev/null
  aws ecr get-login-password --region $REGION | $DOCKER login --username AWS --password-stdin $REGISTRY
  $DOCKER build -f $HERE/Dockerfile -t $IMAGE .
  $DOCKER push $IMAGE
fi

# Job definition and submission
sed -e "s/<ACCOUNT>/$ACCOUNT/g" -e "s/<REGION>/$REGION/g" $HERE/job-definition.yaml > /tmp/piston-batch-job-definition.yaml
aws batch register-job-definition --region $REGION --cli-input-yaml file:///tmp/piston-batch-job-definition.yaml \
  --query '[jobDefinitionName,revision]' --output text
if [ -n "${SKIP_SUBMIT:-}" ]; then exit 0; fi
OVERRIDES=$(python3 - "$@" <<'PY'
import json, os, sys
overrides = {}
if sys.argv[1:]:
    overrides["command"] = sys.argv[1:]
if os.environ.get("FEM"):
    overrides["environment"] = [{"name": "FEM", "value": os.environ["FEM"]}]
print(json.dumps(overrides))
PY
)
ARGS=(--region $REGION --job-name "piston-batch-$(date +%Y%m%d-%H%M%S)" --job-queue $QUEUE --job-definition piston-batch
      --container-overrides "$OVERRIDES")
if [ -n "${DEPENDS_ON:-}" ]; then ARGS+=(--depends-on jobId=$DEPENDS_ON); fi
aws batch submit-job "${ARGS[@]}" --query '[jobName,jobId]' --output text
echo "aws batch describe-jobs --region $REGION --jobs <jobId>"
