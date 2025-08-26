#  CFD Dome Seeing & Wind Loads PSFs

## Cli

```shell
mount-s3 --cache ~/.s3-cache/ maua.cfd.2025 ~/maua/
mount-s3 --cache ~/.s3-cache/ gmto.im.grim ~/mnt
export CUDACXX=/usr/local/cuda/bin/nvcc
export FEM_REPO=~/mnt/20250506_1715_zen_30_M1_202110_FSM_202305_Mount_202305_pier_202411_M1_actDamping/
export CFD_REPO=~/maua/CASES/
export GMT_MODES_PATH=~/Dropbox/AWS/CEO/gmtMirrors/
cargo r -r -- --help
```
