"""Figures of windloads-segement-piston.tex

python3 make_figures.py <data> <fem>
  <data>: directory with segment_piston_summary.parquet (63 cases, fans FEM, 1 % damping),
          <case>.parquet segment piston time series at 1 % damping and
          zeta_2pc/zen30az000_OS_7ms.parquet at 2 % damping
  <fem>:  fans FEM directory with modal_state_space_model_2ndOrder.zip
"""
import io
import sys
import tempfile
import zipfile
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import scipy.io as sio
from scipy.signal import csd, periodogram, welch

DATA, FEM = Path(sys.argv[1]), Path(sys.argv[2])
OUT = Path(__file__).parent / "figures"
FS, FMAX = 1000.0, 153.0
INK, MUTED, GRID = "#23373B", "#5B6475", "#E4E2DA"
BLUE, ORANGE, PURPLE, GREEN, GRAY = "#2A78D6", "#E2622A", "#7353BA", "#2E9E6B", "#4A4A4A"
plt.rcParams.update({
    "font.family": ["Fira Sans", "DejaVu Sans"], "font.size": 11, "axes.edgecolor": MUTED,
    "axes.labelcolor": INK, "text.color": INK, "xtick.color": MUTED, "ytick.color": MUTED,
    "axes.grid": True, "grid.color": GRID, "grid.linewidth": 0.6, "axes.spines.top": False,
    "axes.spines.right": False, "legend.frameon": False, "savefig.bbox": "tight",
})


def piston(name):
    return pd.read_parquet(DATA / f"{name}.parquet")


def psd(x, nperseg=2**14):
    return welch(x * 1e9, fs=FS, nperseg=nperseg)


def segment_grid(series, ylabel, fname, transform, xlim, logy=True, zoom=False):
    fig, axes = plt.subplots(2, 4, figsize=(10, 4.6), sharex=True, sharey=not zoom, layout="constrained")
    for i in range(7):
        ax = axes.flat[i]
        lims = []
        for label, color, lw, x in series(i):
            f, y = transform(x)
            ax.plot(f, y, color=color, lw=lw, label=label)
            lims.append(y[f >= xlim[0]])
        ax.set_xscale("log")
        if logy:
            ax.set_yscale("log")
        if zoom:
            lo, hi = min(l.min() for l in lims), max(l.max() for l in lims)
            ax.set_ylim(lo - 0.15 * (hi - lo), hi + 0.15 * (hi - lo))
        ax.set_xlim(*xlim)
        ax.set_title(f"S{i + 1}", loc="left", fontsize=11)
    lax = axes.flat[7]
    lax.axis("off")
    h, l = axes.flat[0].get_legend_handles_labels()
    lax.legend(h, l, loc="center", fontsize=12)
    for ax in axes[1]:
        ax.set_xlabel("frequency [Hz]")
    axes[0, 3].set_xlabel("frequency [Hz]")
    axes[0, 3].tick_params(labelbottom=True)
    for ax in axes[:, 0]:
        ax.set_ylabel(ylabel)
    fig.savefig(OUT / fname)
    plt.close(fig)


# FEM modes
with zipfile.ZipFile(FEM / "modal_state_space_model_2ndOrder.zip") as z, tempfile.NamedTemporaryFile(suffix=".mat") as f:
    f.write(z.read("rust/modal_state_space_model_2ndOrder_mat.mat"))
    f.flush()
    modes = np.ravel(sio.loadmat(f.name, variable_names=["eigenfrequencies"])["eigenfrequencies"])
fig, ax = plt.subplots(figsize=(6, 3.2), layout="constrained")
ax.hist(modes, bins=np.arange(0, 156, 3), color=BLUE, rwidth=0.85)
ax.set_xlabel("natural frequency [Hz]")
ax.set_ylabel("modes per 3 Hz")
ax.set_xlim(0, 155)
fig.savefig(OUT / "fem_modes.pdf")
plt.close(fig)
print(f"FEM: {modes.size} modes, {modes.min():.3f} to {modes.max():.1f} Hz, {np.sum(modes < 10)} below 10 Hz")

# FSM rejection transfer function: port of the stage 2 FSM loop (piston-batch/src/rejection.rs)
def fsm_residual(u):
    a, b, c, d = (1.77908235, -0.80116713, 1.0, 0.0), (1.0, 0.0), (0.02086505, 0.00109779), 0.00552119
    acc, n_acc, i_mem, pi_state, pi_mem = 0.0, 0, 0.0, 0.0, 0.0
    delay = [0.0] * 7
    x0 = x1 = 0.0
    command = 0.0
    e = np.empty_like(u)
    for k, uk in enumerate(u):
        ek = uk - command
        e[k] = ek
        acc += ek
        n_acc += 1
        if n_acc == 5:
            i_mem += 0.1376 * acc / 5
            pi_state += 0.0719 * i_mem
            pi_mem = i_mem + pi_state
            acc, n_acc = 0.0, 0
        delay[0] = pi_mem
        delay = delay[-1:] + delay[:-1]
        v = delay[0]
        command = c[0] * x0 + c[1] * x1 + d * v
        x0, x1 = a[0] * x0 + a[1] * x1 + b[0] * v, a[2] * x0 + a[3] * x1 + b[1] * v
    return e

u = np.random.default_rng(0).standard_normal(2**20)
e = fsm_residual(u)
f, puu = welch(u, fs=FS, nperseg=2**12)
_, peu = csd(u, e, fs=FS, nperseg=2**12)
m = f > 0.5
f, g = f[m], 20 * np.log10(np.abs(peu[m] / puu[m]))
fig, ax = plt.subplots(figsize=(6, 3.2), layout="constrained")
ax.semilogx(f, g, color=BLUE, lw=2)
ax.axhline(0, color=MUTED, lw=0.8)
f0, k = f[np.argmax(g > 0)], g.argmax()
ax.plot([f0], [0], "o", color=INK, ms=5)
ax.annotate(f"0 dB at {f0:.1f} Hz", (f0, 0), (0.6, 6), arrowprops=dict(arrowstyle="-", color=MUTED))
ax.plot([f[k]], [g[k]], "o", color=ORANGE, ms=5)
ax.annotate(f"+{g[k]:.1f} dB at {f[k]:.1f} Hz", (f[k], g[k]), (f[k] * 2, g[k] + 3), arrowprops=dict(arrowstyle="-", color=MUTED))
ax.set_xlim(0.5, 500)
ax.set_ylim(-35, 12)
ax.set_xlabel("frequency [Hz]")
ax.set_ylabel("tip-tilt rejection [dB]")
fig.savefig(OUT / "fsm_rtf.pdf")
plt.close(fig)
print(f"FSM RTF: 0 dB at {f0:.2f} Hz, max {g[k]:.1f} dB at {f[k]:.1f} Hz, {np.interp(1, f, g):.1f} dB at 1 Hz")

# Fans FEM, 2 % against 1 % damping
p2, p1 = pd.read_parquet(DATA / "zeta_2pc" / "zen30az000_OS_7ms.parquet"), piston("zen30az000_OS_7ms")
pair = lambda i: [("ζ = 2 %", BLUE, 1.8, p2[f"S{i + 1}"]), ("ζ = 1 %", ORANGE, 0.9, p1[f"S{i + 1}"])]

def band(x, lo, hi):
    f, p = psd(x)
    m = (f > lo) & (f <= hi)
    return np.sqrt(np.trapezoid(p[m], f[m]) if hasattr(np, "trapezoid") else np.trapz(p[m], f[m]))

def positive_psd(x):
    f, p = psd(x)
    m = (f > 0) & (f <= FMAX)
    return f[m], p[m]

def cumulative_std(x):
    # periodogram of the whole record (0.0025 Hz resolution): its sum is the variance of the time series,
    # whereas the Welch PSD misses the drift slower than its 0.06 Hz resolution
    f, p = periodogram(x * 1e9, fs=FS)
    m = f <= FMAX
    return f[m], np.sqrt(np.cumsum(p[m]) * f[1])

segment_grid(pair, "PSD [nm²/Hz]", "damping_psd.pdf", positive_psd, (0.06, FMAX))
segment_grid(pair, "std below f [nm]", "damping_cumstd.pdf", cumulative_std, (1, FMAX), logy=False, zoom=True)
print("damping table: segment, std 2 %, std 1 %, 3-153 Hz 2 %, 3-153 Hz 1 %, 0-1 Hz 2 %, 0-1 Hz 1 %")
for i in range(7):
    s = f"S{i + 1}"
    print(s, *(f"{v:.1f}" for v in (p2[s].std() * 1e9, p1[s].std() * 1e9, band(p2[s], 3, FMAX), band(p1[s], 3, FMAX),
                                    band(p2[s], 0, 1), band(p1[s], 0, 1))))
for i in range(7):
    s = f"S{i + 1}"
    print(s, "std / cumulative std at 153 Hz:", *(f"{x[s].std() * 1e9:.1f}/{cumulative_std(x[s])[1][-1]:.1f}" for x in (p2, p1)))
print("FSM 1 %:", [round(p1[f"S{i + 1}_FSM"].std() * 1e9, 1) for i in range(7)])

# 63 cases
d = pd.read_parquet(DATA / "segment_piston_summary.parquet")
for c in ["mean", "std", "mean_FSM", "std_FSM"]:
    d[c] *= 1e9
c = d.case.str.extract(r"zen(\d+)az(\d+)_(\w\w)_(\d+)ms")
d["zen"], d["az"], d["cfg"], d["ws"] = c[0].astype(int), c[1].astype(int), c[2], c[3].astype(int)
order = d.drop_duplicates("case").sort_values(["zen", "ws", "az"]).case.tolist()
w = d.pivot(index="segment", columns="case", values="std")[order]
fig, ax = plt.subplots(figsize=(10, 3.3), layout="constrained")
im = ax.imshow(np.log10(w.values), aspect="auto", cmap="Blues", vmin=0, vmax=np.log10(6000), interpolation="nearest")
ax.grid(False)
ax.set_yticks(range(7), [f"S{i}" for i in range(1, 8)])
cases = d.drop_duplicates("case").set_index("case").loc[order]
starts = [k for k in range(len(order)) if k == 0 or (cases.zen.iloc[k], cases.ws.iloc[k]) != (cases.zen.iloc[k - 1], cases.ws.iloc[k - 1])]
for k in starts[1:]:
    ax.axvline(k - 0.5, color="white", lw=1.5 if cases.zen.iloc[k] == cases.zen.iloc[k - 1] else 4)
centers = [(a + b - 1) / 2 for a, b in zip(starts, starts[1:] + [len(order)])]
ax.set_xticks(centers, [f"{cases.cfg.iloc[k]}\n{cases.ws.iloc[k]}" for k in starts], fontsize=8)
for z in (0, 30, 60):
    ks = [k for k in range(len(order)) if cases.zen.iloc[k] == z]
    ax.text((ks[0] + ks[-1]) / 2, -0.9, f"zenith {z}°", ha="center", va="bottom", fontsize=10)
cb = fig.colorbar(im, ax=ax, pad=0.01)
cb.set_ticks([0, 1, 2, 3], labels=["1", "10", "100", "1000"])
cb.set_label("open-loop piston std [nm]")
ax.set_xlabel("enclosure configuration and wind speed [m/s], azimuth 0° to 180° within each block")
fig.savefig(OUT / "cases_heatmap.pdf")
plt.close(fig)

d["outer"] = d.segment < 7
fig, axes = plt.subplots(1, 2, figsize=(10, 3.6), sharey=True, layout="constrained")
for ax, outer, title in zip(axes, (True, False), ("outer segments S1–S6", "centre segment S7")):
    for z, color, dx in ((0, BLUE, -0.25), (30, ORANGE, 0), (60, PURPLE, 0.25)):
        s = d[(d.zen == z) & (d.outer == outer)].groupby("ws")["std"]
        ws = np.array(s.mean().index)
        ax.errorbar(ws + dx, s.mean(), yerr=[s.mean() - s.min(), s.max() - s.mean()], color=color, marker="o",
                    ms=5, lw=1.5, capsize=3, label=f"zenith {z}°")
    ax.set_yscale("log")
    ax.set_xticks([0, 2, 7, 12, 17])
    ax.axvspan(9.5, 18.5, color=GRID, alpha=0.5, lw=0)
    ax.text(9.8, 1.2, "closed vents", color=MUTED, fontsize=9)
    ax.text(0.2, 1.2, "open vents", color=MUTED, fontsize=9)
    ax.set_xlim(-1, 18.5)
    ax.set_ylim(0.8, 1e4)
    ax.set_title(title, loc="left", fontsize=11)
    ax.set_xlabel("wind speed [m/s]")
axes[0].set_ylabel("open-loop piston std [nm]")
axes[0].legend(loc="upper left")
fig.savefig(OUT / "std_vs_wind.pdf")
plt.close(fig)

cases_psd = [("zen30az000_OS_2ms", "zen 30°, OS 2 m/s", GREEN), ("zen30az000_OS_7ms", "zen 30°, OS 7 m/s", BLUE),
             ("zen30az000_CD_12ms", "zen 30°, CD 12 m/s", ORANGE), ("zen30az000_CD_17ms", "zen 30°, CD 17 m/s", PURPLE),
             ("zen60az000_CS_17ms", "zen 60°, CS 17 m/s", GRAY)]
fig, axes = plt.subplots(1, 2, figsize=(10, 3.6), layout="constrained")
for name, label, color in cases_psd:
    x = piston(name)["S1"]
    f, p = positive_psd(x)
    axes[0].loglog(f, p, color=color, lw=1, label=label)
    f, s = cumulative_std(x)
    axes[1].semilogx(f, s / s[-1], color=color, lw=1.5, label=label)
axes[0].set_xlim(0.06, FMAX)
axes[0].set_ylabel("S1 piston PSD [nm²/Hz]")
axes[1].set_xlim(0.06, FMAX)
axes[1].set_ylim(0, 1.02)
axes[1].set_ylabel("S1 std below f / total")
for ax in axes:
    ax.set_xlabel("frequency [Hz]")
axes[1].legend(loc="lower right", fontsize=9)
fig.savefig(OUT / "cases_psd.pdf")
plt.close(fig)
for name, *_ in cases_psd:
    x = piston(name)["S1"]
    f, s = cumulative_std(x)
    print(name, f"S1 std {x.std() * 1e9:.0f} nm, fraction below 1 Hz {np.interp(1, f, s) / s[-1]:.3f}, below 3 Hz {np.interp(3, f, s) / s[-1]:.3f}")

d["ratio"] = 100 * (d.std_FSM / d["std"] - 1)
fig, ax = plt.subplots(figsize=(6, 3.2), layout="constrained")
for outer, color, label in ((True, BLUE, "S1–S6"), (False, ORANGE, "S7")):
    s = d[d.outer == outer]
    ax.semilogx(s["std"], s.ratio, "o", color=color, ms=4, alpha=0.8, label=label, mec="white", mew=0.5)
ax.axhline(0, color=MUTED, lw=0.8)
ax.set_xlabel("open-loop piston std [nm]")
ax.set_ylabel("FSM / open loop − 1 [%]")
ax.legend(loc="upper left")
fig.savefig(OUT / "fsm_ratio.pdf")
plt.close(fig)

# Tables
g = d.groupby(["zen", "cfg", "ws", "outer"])["std"].agg(["mean", "max"]).unstack("outer")
print(g.round(0))
print("FSM ratio outer:", d[d.outer].ratio.describe().round(3).to_dict())
print("FSM ratio S7:", d[~d.outer].ratio.describe().round(3).to_dict())
top = d.sort_values("std", ascending=False).head(6)[["case", "segment", "std", "std_FSM", "mean"]]
print(top.round(0))
print("mean piston |max| [um]:", (d["mean"].abs().groupby(d.case).max() / 1e3).sort_values().tail(5).round(1).to_dict())
print("all-case outer std: median", round(d[d.outer]["std"].median()), "max", round(d[d.outer]["std"].max()))
