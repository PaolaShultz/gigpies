#!/usr/bin/env python3
"""Prepare a complete unmatched listening set from frozen production renders.

Usage: python3 scripts/listening_checkpoint.py specification.json NEW_DIRECTORY
The specification supplies expected_ids and examples with id, source_render,
source_settings_sha256, source_wav_sha256, final_render and starts (seconds).
The SOURCE hashes must come from the established initial comparison. SOURCE uses the source render's bypass, so
later artistic faders cannot silently redefine the original comparison.
Only standard-library modules are needed. This tool never plays audio.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import shutil
import wave

from balance_excerpts import inspect


def read(path):
    return json.loads(Path(path).read_text())


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_excerpt(parent, clip, start, count):
    """Check format, complete duration and every PCM byte at the stated offset."""
    with wave.open(str(parent), "rb") as original, wave.open(str(clip), "rb") as cut:
        a, b = original.getparams(), cut.getparams()
        if (a.nchannels, a.sampwidth, a.framerate, a.comptype) != (
            b.nchannels, b.sampwidth, b.framerate, b.comptype
        ) or b.nframes != count:
            raise ValueError("excerpt format or duration differs")
        if start < 0 or count <= 0 or start + count > a.nframes:
            raise ValueError("excerpt outside parent")
        original.setpos(start)
        expected = original.readframes(count)
        actual = cut.readframes(count)
        if len(expected) != count * a.nchannels * a.sampwidth or actual != expected:
            raise ValueError("excerpt is truncated, shifted or has changed gain/samples")
    return hashlib.sha256(actual).hexdigest()


def prepare(spec, output):
    output = Path(output)
    if output.exists():
        raise FileExistsError(output)
    examples = spec["examples"]
    ids = [e["id"] for e in examples]
    if (not ids or len(set(ids)) != len(ids)
            or len(set(spec["expected_ids"])) != len(spec["expected_ids"])
            or set(ids) != set(spec["expected_ids"])):
        raise ValueError("incomplete or duplicate example inventory")
    checked = []
    for entry in examples:
        identifier = entry["id"]
        if not identifier or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in identifier):
            raise ValueError("invalid example id")
        renders = [Path(entry[key]).resolve() for key in ("source_render", "final_render")]
        pins = (entry.get("source_settings_sha256"), entry.get("source_wav_sha256"))
        if any(not isinstance(pin, str) or len(pin) != 64
               or any(c not in "0123456789abcdef" for c in pin) for pin in pins):
            raise ValueError("pin established SOURCE settings and WAV hashes before preparation")
        if (digest(renders[0] / "prepared.json") != pins[0]
                or digest(renders[0] / "bypass.wav") != pins[1]):
            raise ValueError("SOURCE differs from pinned initial comparison")
        metadata_hashes = [{name: digest(p / name) for name in ("prepared.json", "measurements.json")}
                           for p in renders]
        settings = [read(p / "prepared.json") for p in renders]
        reports = [read(p / "measurements.json") for p in renders]
        for s, r in zip(settings, reports):
            if s["output_mode"] != "unmatched" or s["ceiling_db"] != -0.01:
                raise ValueError("requires independently peak-finalized unmatched renders")
            if s["master_db"] != 0 or s["calibration"]["initial_trim_db"] != 0 or any(
                g["trim_db"] != 0 for g in s["groups"]
            ):
                raise ValueError("SOURCE convention requires unity input trims/master")
            if r["output_mode"] != "unmatched":
                raise ValueError("render mode differs from settings")
        routing = lambda s: [(c["file"], c["pan"]) for c in s["channels"]]
        timeline = lambda r: [(c["file"], c["bwf_time_reference"], c["offset_frames"],
                               c["source_frames"]) for c in r["channels"]]
        if routing(settings[0]) != routing(settings[1]) or timeline(reports[0]) != timeline(reports[1]):
            raise ValueError("SOURCE and FINAL routing, pan or alignment differs")
        for report in reports:
            for channel in report["channels"]:
                padding = report["frames"] - channel["offset_frames"] - channel["source_frames"]
                if padding < 0 or channel["tail_padding_frames"] != padding:
                    raise ValueError("source duration or tail padding differs from render timeline")
        extra = reports[1]["frames"] - reports[0]["frames"]
        if extra:
            fx = reports[1].get("effects") or {}
            config = settings[1].get("effects") or {}
            if (not entry.get("allow_fx_tail", False) or extra < 0
                    or fx.get("source_frames") != reports[0]["frames"]
                    or fx.get("tail_frames") != extra
                    or not 0 < config.get("tail_seconds", 0) <= 15
                    or round(config["tail_seconds"] * reports[1]["sample_rate"]) != extra):
                raise ValueError("different lengths require an explicit, verified production FX tail")
        parents = [renders[0] / "bypass.wav", renders[1] / "processed.wav"]
        measurements = [inspect(p) for p in parents]
        params = measurements[0][0]
        if params._replace(nframes=0) != measurements[1][0]._replace(nframes=0):
            raise ValueError("SOURCE and FINAL formats or timelines differ")
        for (own_params, meter), report in zip(measurements, reports):
            if (meter["samples"] != own_params.nframes * own_params.nchannels
                    or abs(meter["sample_peak_dbfs"] + 0.01) > 0.00001
                    or report["frames"] != own_params.nframes
                    or report["sample_rate"] != own_params.framerate):
                raise ValueError("truncated export, incorrect peak or render metadata")
        starts = entry["starts"]
        if not 1 <= len(starts) <= 2 or (len(starts) == 2 and not entry.get("second_passage_reason")):
            raise ValueError("supply one excerpt, or explain the second passage")
        frames = []
        for start in starts:
            if not math.isfinite(start) or start < 0:
                raise ValueError("invalid excerpt start")
            frame = round(start * params.framerate)
            if frame + 12 * params.framerate > params.nframes:
                raise ValueError("excerpt exceeds parent")
            frames.append(frame)
        checked.append((entry, renders, parents, reports, measurements, frames, metadata_hashes))
    # No output or ready manifest exists until every example has passed preflight.
    output.mkdir()
    manifest = {"order": "SOURCE → FINAL MIX for each passage in listed order",
                "playback": False, "loudness_matching": False, "true_peak_compliance": False,
                "source_definition": "selected originals through SOURCE-settings pan/faders; no DSP; independent sample-peak finalization",
                "examples": []}
    for entry, renders, parents, reports, measurements, frames, metadata_hashes in checked:
        folder = output / entry["id"]
        folder.mkdir()
        record = {**entry, "exports": [], "excerpts": []}
        params = measurements[0][0]
        for label, stage, render, parent, report, (own_params, meter) in zip(
            ("SOURCE", "FINAL-MIX"), ("bypass", "processed"), renders, parents, reports, measurements
        ):
            # Link complete retained exports without another audio copy or mutation.
            (folder / f"{label}.wav").symlink_to(parent)
            shutil.copyfile(render / "prepared.json", folder / f"{label}-settings.json")
            gain = report[f"{stage}_export_gain_db"]
            record["exports"].append({"label": label, "file": str((folder / f"{label}.wav").resolve()),
                **meter, "export_gain_db": gain, "export_gain_effect": "constant gain on every sample; preserves relative balance and crest",
                "bus_lufs_before_export": report[f"{stage}_lufs"],
                "loudness_note": "bus LUFS is before export; final PCM RMS/peak above are measured from the file",
                "render_report": str(render / "measurements.json"),
                "settings_sha256": digest(render / "prepared.json"),
                "sample_rate": params.framerate, "channels": params.nchannels, "bits": 8 * params.sampwidth,
                "frames": own_params.nframes,
                "fx_tail_frames_beyond_source": own_params.nframes - params.nframes})
        for number, start in enumerate(frames, 1):
            for ordinal, (label, parent) in enumerate(zip(("SOURCE", "FINAL-MIX"), parents), 1):
                clip = folder / f"{number:02}-{ordinal}-{label}-{start / params.framerate:07.3f}s.wav"
                with wave.open(str(parent), "rb") as reader:
                    reader.setpos(start)
                    data = reader.readframes(12 * params.framerate)
                with wave.open(str(clip), "wb") as writer:
                    writer.setparams(params)
                    writer.writeframes(data)
                pcm_hash = verify_excerpt(parent, clip, start, 12 * params.framerate)
                record["excerpts"].append({"file": str(clip.resolve()), "parent": str(parent),
                    "start_frame": start, "frames": 12 * params.framerate,
                    "gain_change_db": 0, "pcm_sha256": pcm_hash})
        manifest["examples"].append(record)
    # Recheck every parent and metadata file immediately before publishing the
    # completion manifest, including examples prepared earlier in the set.
    for _, renders, parents, _, measurements, _, metadata_hashes in checked:
        for parent, (_, meter) in zip(parents, measurements):
            if digest(parent) != meter["sha256"]:
                raise ValueError("parent export changed during preparation")
        for render, hashes in zip(renders, metadata_hashes):
            if any(digest(render / name) != expected for name, expected in hashes.items()):
                raise ValueError("render metadata changed during preparation")
    manifest["complete_verified"] = True
    with (output / "manifest.json").open("x") as stream:
        json.dump(manifest, stream, indent=2)
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("specification", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = prepare(read(args.specification), args.output)
    print(f"Verified {len(result['examples'])} complete SOURCE → FINAL MIX comparisons: {args.output}")
