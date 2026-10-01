#!/usr/bin/env python3
"""Offline review aid; colours mean energy, not errors. Requires analysis-requirements.txt."""
import argparse
import json
from pathlib import Path
import numpy as np
from scipy.io import wavfile
from scipy import signal
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def read(path):
    rate, data = wavfile.read(path)
    if data.dtype.kind == 'i':
        data = data.astype(np.float64) / 2 ** (data.dtype.itemsize * 8 - 1)
    else:
        data = data.astype(np.float64)
    if data.ndim == 1:
        data = data[:, None]
    if not np.isfinite(data).all():
        raise ValueError('nonfinite audio')
    return rate, data


def analyze(path):
    rate, data = read(path)
    # Welch averages channel POWERS; summing L/R first would conceal cancellation.
    f, power = signal.welch(data, rate, nperseg=8192, noverlap=4096, axis=0)
    power = power.mean(axis=1)
    df = f[1] - f[0]
    centres = 1000 * 2 ** (np.arange(-17, 14) / 3)
    bands = []
    for hz in centres:
        bins = (f >= hz / 2 ** (1 / 6)) & (f < hz * 2 ** (1 / 6))
        bands.append(10 * np.log10(max(float(power[bins].sum() * df), 1e-24)))
    # Frequency-local prominence flags resonances for inspection, not auto-EQ.
    smooth = np.convolve(power, np.ones(5) / 5, mode='same')
    peaks, props = signal.find_peaks(10 * np.log10(np.maximum(smooth, 1e-24)), prominence=5)
    ranked = sorted(zip(peaks, props['prominences']), key=lambda a: smooth[a[0]], reverse=True)
    hotspots = [{'hz': round(float(f[i]), 1), 'prominence_db': round(float(p), 2)} for i, p in ranked if 40 < f[i] < 12000][:12]
    n = rate
    rms = [float(np.sqrt(np.mean(data[i:i+n] ** 2))) for i in range(0, len(data), n)]
    f2, t, psd = signal.spectrogram(data, rate, nperseg=4096, noverlap=2048, axis=0)
    # scipy axis=0 yields [frequency, channels, time].
    spect = 10 * np.log10(np.maximum(psd.mean(axis=1), 1e-24))
    report = {'path': str(path), 'rate': rate, 'frames': len(data),
              'sample_peak_dbfs': float(20*np.log10(max(float(np.max(np.abs(data))), 1e-12))),
              'rms_dbfs': float(10*np.log10(max(float(np.mean(data**2)), 1e-24))),
              'mono_power_ratio_db': float(10*np.log10(max(float(np.mean(data.mean(axis=1)**2)/max(np.mean(data**2), 1e-24)), 1e-24))),
              'third_octave_hz': centres.tolist(), 'third_octave_dbfs': bands,
              'frequency_local_prominence_candidates': hotspots,
              'one_second_rms_dbfs': [20*np.log10(max(x, 1e-12)) for x in rms]}
    return report, (f, power, f2, t, spect)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('wavs', nargs='+', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    reports, plots = zip(*(analyze(p) for p in args.wavs))
    fig, axes = plt.subplots(len(plots)+1, 1, figsize=(13, 4+3*len(plots)))
    for p, r, (f, power, f2, t, spect) in zip(args.wavs, reports, plots):
        axes[0].semilogx(r['third_octave_hz'], r['third_octave_dbfs'], label=str(p.parent.name)+'/'+p.name)
    axes[0].set(xlim=(30, 16000), ylabel='Third-octave power (dBFS)', title='Band energy; compare equal-loudness files. No universal ideal curve.')
    axes[0].grid(True, alpha=.3); axes[0].legend(fontsize=8)
    # Fixed scale shared across panels; magma uses red/pink for high energy.
    for ax, p, (_, _, f2, t, spect) in zip(axes[1:], args.wavs, plots):
        mesh = ax.pcolormesh(t, f2, spect, shading='auto', cmap='magma', vmin=-100, vmax=-40, rasterized=True)
        ax.set(yscale='log', ylim=(30, 16000), xlabel='Time (s)', ylabel='Hz', title=str(p))
        fig.colorbar(mesh, ax=ax, label='PSD dBFS/Hz: red/pink = energy, not a defect')
    fig.tight_layout(); fig.savefig(args.output/'spectrum.png', dpi=150); fig.savefig(args.output/'spectrum.pdf'); plt.close(fig)
    (args.output/'analysis.json').write_text(json.dumps(reports, indent=2)+'\n')
    for r in reports:
        print(r['path'], 'peak', round(r['sample_peak_dbfs'], 2), 'RMS', round(r['rms_dbfs'], 2), 'hotspots', r['frequency_local_prominence_candidates'][:6])


if __name__ == '__main__':
    main()
