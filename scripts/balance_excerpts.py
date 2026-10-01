#!/usr/bin/env python3
"""Cut exact PCM frames in OLD/A/B order; no gain changes and no playback.

Example: python3 scripts/balance_excerpts.py OLD.wav A.wav B.wav new-dir 12 36 72
Only standard-library modules are used. New output directory is required.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import wave


def inspect(path):
    peak = 0
    sum_squares = 0
    samples = 0
    with wave.open(str(path), "rb") as reader:
        params = reader.getparams()
        if params.sampwidth != 3 or params.nchannels != 2 or params.comptype != "NONE":
            raise ValueError("Expected finished 24-bit stereo PCM")
        while data := reader.readframes(65536):
            for i in range(0, len(data), 3):
                x = int.from_bytes(data[i:i + 3], "little", signed=True)
                peak = max(peak, abs(x))
                sum_squares += x * x
                samples += 1
    scale = 8388608
    return params, {
        "sample_peak_dbfs": 20 * math.log10(max(peak / scale, 1e-12)),
        "rms_dbfs": 10 * math.log10(max(sum_squares / max(samples, 1) / scale**2, 1e-24)),
        "samples": samples,
        "sha256": hashlib.file_digest(path.open("rb"), "sha256").hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("old", type=Path)
    parser.add_argument("a", type=Path)
    parser.add_argument("b", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("starts", type=float, nargs="+")
    args = parser.parse_args()
    sources = [args.old, args.a, args.b]
    info = [inspect(p) for p in sources]
    if any(p != info[0][0] for p, _ in info):
        raise ValueError("OLD/A/B formats or frame counts differ")
    params = info[0][0]
    for _, measured in info:
        if abs(measured["sample_peak_dbfs"] + 0.01) > 0.00001:
            raise ValueError("Finished file is not independently finalized to -0.01 dBFS")
    starts = []
    for t in args.starts:
        if not math.isfinite(t) or t < 0:
            raise ValueError("Invalid excerpt start")
        frame = round(t * params.framerate)
        if frame + 12 * params.framerate > params.nframes:
            raise ValueError("Excerpt exceeds source duration")
        starts.append(frame)
    args.out.mkdir()  # Refuse overwrite, even for partial evidence.
    manifest = {"order": "OLD -> A -> B at each timestamp", "gain_changes": False,
                "playback": False, "true_peak_protection": False,
                "full_songs": [{"label": label, "path": str(path.resolve()), **m}
                               for label, path, (_, m) in zip(["OLD", "A", "B"], sources, info)],
                "excerpts": []}
    for section, frame in enumerate(starts, 1):
        for ordinal, (label, path) in enumerate(zip(["OLD", "A", "B"], sources), 1):
            out = args.out / f"{section:02d}-{ordinal}-{label}-{frame / params.framerate:07.3f}s.wav"
            with wave.open(str(path), "rb") as reader:
                reader.setpos(frame)
                data = reader.readframes(12 * params.framerate)
            with wave.open(str(out), "wb") as writer:
                writer.setparams(params)
                writer.writeframes(data)
            with wave.open(str(out), "rb") as check:
                assert check.getnframes() == 12 * params.framerate
                assert check.readframes(check.getnframes()) == data
            manifest["excerpts"].append({"label": label, "start_frame": frame,
                "start_seconds": frame / params.framerate, "duration_seconds": 12,
                "path": str(out.resolve()), "identical_pcm_slice_verified": True})
    with (args.out / "manifest.json").open("x") as f:
        json.dump(manifest, f, indent=2)
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
