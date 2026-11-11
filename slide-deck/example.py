import pandas as pd
from scipy.signal import welch

FEM = "20251110_1617_zen_30_M1_202110_FSM_202305_Mount_202305_pier_202411_M1Fans_M1S1_150Hz"
case = "zen30az000_OS_7ms"
key = f"gmto.cfd.2025/FEM/{FEM}/zeta_1pc/cfd/{case}/segment_piston.parquet"

# S3 API, anonymous access (pip install pandas pyarrow s3fs scipy)
piston = pd.read_parquet(f"s3://{key}", storage_options={"anon": True}).astype("float64")
# or HTTPS: pd.read_parquet(f"https://s3.us-east-1.amazonaws.com/{key}").astype("float64")

open_loop = piston[[f"S{i}" for i in range(1, 8)]] * 1e9  # [nm]
fsm = piston[[f"S{i}_FSM" for i in range(1, 8)]] * 1e9  # [nm]
print(open_loop.std().round(1))  # segment piston std [nm]
f, psd = welch(open_loop.values.T, fs=1e3, nperseg=2**14)  # PSD [nm^2/Hz]
