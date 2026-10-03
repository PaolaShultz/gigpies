#!/usr/bin/env python3
"""Independently check final PCM with FFmpeg/libsoxr; never plays or writes audio.

Usage: python3 scripts/verify_delivery.py DELIVERY_DIRECTORY
Requires FFmpeg with libsoxr. Writes independent-verification.json only on success.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import wave

ALGORITHM = "ffmpeg-libsoxr-32x-precision33-zero-padded-v1"


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def independent_peak(path, rate):
    # 128 zero frames at both boundaries expose interpolation outside the original
    # interval. This is a measurement stream, never an output audio file.
    command = ["ffmpeg", "-hide_banner", "-nostdin", "-nostats", "-threads", "1",
               "-filter_threads", "1", "-i", str(path), "-af",
               f"aformat=sample_fmts=dbl,adelay=128S:all=1,apad=pad_len=128,aresample={rate * 32}:"
               "resampler=soxr:precision=33:cutoff=1:cheby=1:osf=dbl,astats=metadata=0:reset=0",
               "-f", "null", "-"]
    result = subprocess.run(command, capture_output=True, text=True, check=True)
    peaks = re.findall(r"Peak level dB:\s*([-+\w.]+)", result.stderr)
    if not peaks:
        raise ValueError("independent meter returned no peak")
    values = [float(v) for v in peaks]
    if any(math.isnan(v) or v == math.inf for v in values):
        raise ValueError("nonfinite independent peak")
    return max(values), command


def verify(directory, meter=independent_peak):
    directory = Path(directory)
    destination = directory / "independent-verification.json"
    if destination.exists():
        raise FileExistsError(destination)
    ready = json.loads((directory / "delivery-ready.json").read_text())
    if ready.get("contract") != "gigpies-delivery-v1" or ready.get("version") != 1:
        raise ValueError("requires policy-aware delivery completion")
    required = {"delivery.json", "delivery-policy.json", "prepared.json", "measurements.json",
                "sources.json", "report.txt", "processed-unity-float.wav", "processed.wav"}
    if not required.issubset(ready.get("files", {})):
        raise ValueError("incomplete delivery identities")
    for name, pin in ready["files"].items():
        if Path(name).name != name or digest(directory / name) != pin:
            raise ValueError("stale or unsafe delivery identity")
    policy = json.loads((directory / "delivery-policy.json").read_text())
    report = json.loads((directory / "delivery.json").read_text())
    if policy.get("version") != 1 or policy.get("peak_basis") != "true":
        raise ValueError("independent true-peak check requires true-peak policy")
    ceiling = policy["ceiling_db"]
    if not math.isfinite(ceiling) or not -60 <= ceiling <= -0.01:
        raise ValueError("invalid ceiling")
    records = {}
    exports = {"processed.wav": report["primary"], **report["comparison_copies"]}
    for name, export in exports.items():
        if name not in ready["files"] or Path(name).name != name:
            raise ValueError("unbound output")
        path = directory / name
        with wave.open(str(path), "rb") as wav:
            if (wav.getnchannels() != 2 or wav.getsampwidth() != 3
                    or wav.getnframes() != export["output"]["frames"]
                    or wav.getframerate() != export["output"]["sample_rate"]):
                raise ValueError("output format/timeline mismatch")
            rate = wav.getframerate()
        peak, command = meter(path, rate)
        if peak > ceiling:
            raise ValueError(f"independent peak exceeds ceiling: {peak} > {ceiling}")
        difference = peak - export["output"]["true_peak_dbtp"] if math.isfinite(peak) else None
        if difference is not None and abs(difference) > 0.2:
            raise ValueError(f"unresolved meter disagreement exceeds 0.2 dB: {difference}")
        if export["pcm_sha256"] != digest(path):
            raise ValueError("PCM differs from delivery report")
        records[name] = {"sha256": digest(path), "true_peak_dbtp": peak if math.isfinite(peak) else None,
                         "digital_silence": peak == -math.inf, "ceiling_db": ceiling,
                         "margin_db": ceiling - peak if math.isfinite(peak) else None,
                         "independent_minus_primary_db": difference,
                         "command": command}
    for name, pin in ready["files"].items():
        if digest(directory / name) != pin:
            raise ValueError("delivery changed during verification")
    result = {"version": 1, "contract": "gigpies-independent-delivery-v1", "algorithm": ALGORITHM,
              "ffmpeg_version": subprocess.run(["ffmpeg", "-version"], capture_output=True, text=True, check=True).stdout.splitlines()[0],
              "delivery_ready_sha256": digest(directory / "delivery-ready.json"),
              "peak_display_precision_db": 0.000001, "files": records, "passed": True}
    with destination.open("x") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    print(json.dumps(verify(parser.parse_args().directory), indent=2))
