import pandas as pd
from scipy.signal import welch

FEM = "20251110_1617_zen_30_M1_202110_FSM_202305_Mount_202305_pier_202411_M1Fans_M1S1_150Hz"
case = "zen30az000_OS_7ms"
url = ("https://s3.us-east-1.amazonaws.com/gmto.cfd.2025"
       f"/FEM/{FEM}/zeta_1pc/cfd/{case}/segment_piston.parquet")

# HTTPS download, no AWS account needed (pip install pandas pyarrow scipy)
piston = pd.read_parquet(url).astype("float64")

open_loop = piston[[f"S{i}" for i in range(1, 8)]] * 1e9  # [nm]
fsm = piston[[f"S{i}_FSM" for i in range(1, 8)]] * 1e9  # [nm]
print(open_loop.std().round(1))  # segment piston std [nm]
f, psd = welch(open_loop.values.T, fs=1e3, nperseg=2**14)  # PSD [nm^2/Hz]
