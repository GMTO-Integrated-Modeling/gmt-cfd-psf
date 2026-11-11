#!/usr/bin/env bash
# Time-limited public read access to the segment piston of the 63 CFD cases (FEM at 1 % damping):
#   slide-deck/share-data.sh grant    # public until EXPIRY
#   slide-deck/share-data.sh revoke   # removes the policy and restores the bucket public access block
# Only the objects
#   s3://gmto.cfd.2025/FEM/<FEM>/zeta_1pc/cfd/<case>/segment_piston.parquet
# become readable, at https://s3.us-east-1.amazonaws.com/gmto.cfd.2025/<key> (path-style: the dots of the
# bucket name break the TLS certificate of the virtual-hosted URL), until EXPIRY (UTC).
# The bucket has no other bucket policy; ACLs stay blocked.
set -euo pipefail

BUCKET=gmto.cfd.2025
REGION=us-east-1
FEM=20251110_1617_zen_30_M1_202110_FSM_202305_Mount_202305_pier_202411_M1Fans_M1S1_150Hz
EXPIRY=${EXPIRY:-2026-12-26T00:00:00Z}

case "${1:-}" in
grant)
  if aws s3api get-bucket-policy --bucket $BUCKET --region $REGION >/dev/null 2>&1; then
    echo "$BUCKET already has a bucket policy, merge it by hand" >&2
    exit 1
  fi
  # public bucket policies must be allowed; public ACLs stay blocked
  aws s3api put-public-access-block --bucket $BUCKET --region $REGION --public-access-block-configuration \
    BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=false,RestrictPublicBuckets=false
  aws s3api put-bucket-policy --bucket $BUCKET --region $REGION --policy "{
    \"Version\": \"2012-10-17\",
    \"Statement\": [{
      \"Sid\": \"SegmentPistonPublicReadUntil\",
      \"Effect\": \"Allow\",
      \"Principal\": \"*\",
      \"Action\": \"s3:GetObject\",
      \"Resource\": \"arn:aws:s3:::$BUCKET/FEM/$FEM/zeta_1pc/cfd/*/segment_piston.parquet\",
      \"Condition\": { \"DateLessThan\": { \"aws:CurrentTime\": \"$EXPIRY\" } }
    }]
  }"
  echo "public read of FEM/$FEM/zeta_1pc/cfd/*/segment_piston.parquet until $EXPIRY"
  ;;
revoke)
  aws s3api delete-bucket-policy --bucket $BUCKET --region $REGION
  aws s3api put-public-access-block --bucket $BUCKET --region $REGION --public-access-block-configuration \
    BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true
  echo "public access to $BUCKET revoked"
  ;;
*)
  echo "usage: $0 grant|revoke" >&2
  exit 1
  ;;
esac
