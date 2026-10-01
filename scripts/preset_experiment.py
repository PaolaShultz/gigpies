#!/usr/bin/env python3
"""Local preset-reference mapping and bounded experiment evidence; no media discovery/downloads."""

import argparse
import copy
import csv
import hashlib
import json
import math
import statistics
import wave
from pathlib import Path


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    with Path(path).open("x") as f:
        json.dump(value, f, indent=2, allow_nan=False)
        f.write("\n")


def derive(session, inventory, assignments):
    """Map reviewed local records; unknown filters/dynamics remain in the audit, bypassed."""
    out = copy.deepcopy(session)
    decisions = []
    seen = set()
    for assignment in assignments:
        i = assignment["channel"]
        if not isinstance(i, int) or i < 0 or i >= len(out["channels"]) or i in seen:
            raise ValueError("invalid or duplicate channel assignment")
        seen.add(i)
        ch = out["channels"][i]
        if ch["file"] != assignment["file"]:
            raise ValueError("assignment file/index mismatch")
        if (
            ch["eq"]
            or ch["compressor"]["ratio"] != 1
            or ch["compressor"]["makeup_db"] != 0
        ):
            raise ValueError(
                "mapping requires neutral EQ/compression; preserve the prior experiment separately"
            )
        ch["eq"] = []
        ch["compressor"].update(ratio=1.0, makeup_db=0.0)
        for key in ("eq", "compressor"):
            if not assignment.get(key):
                continue
            record = inventory[assignment[key]]
            if not all(
                record.get(k)
                for k in (
                    "manufacturer",
                    "product",
                    "preset",
                    "source",
                    "page",
                    "original",
                )
            ):
                raise ValueError("incomplete source provenance")
            notes = []
            original = record["original"]
            if key == "eq":
                for band in original["bands"]:
                    kind = {
                        "PEAKING": "bell",
                        "L.SHELF": "low_shelf",
                        "H.SHELF": "high_shelf",
                    }.get(band["type"])
                    if kind is None:
                        notes.append(
                            {
                                "unsupported": band,
                                "action": "bypassed; no filter substitution",
                            }
                        )
                        continue
                    q = band["q"] if kind == "bell" else math.sqrt(0.5)
                    ch["eq"].append(
                        {"kind": kind, "hz": band["hz"], "q": q, "db": band["db"]}
                    )
                    if kind != "bell":
                        notes.append(
                            "Shelf uses our monotonic RBJ S=1; Yamaha shelf slope is unspecified."
                        )
                notes.append(
                    "Published Q used as RBJ peaking Q; Yamaha EQ type/transfer function equivalence unverified."
                )
            elif original["type"] != "COMP":
                notes.append(
                    {
                        "unsupported": original,
                        "action": "dynamics bypassed; no expander/compander substitution",
                    }
                )
            else:
                p = ch["compressor"]
                p.update(
                    {
                        k: original[k]
                        for k in ("threshold_db", "ratio", "attack_ms", "release_ms")
                    }
                )
                p["makeup_db"] = original["out_gain_db"]
                p["knee_db"] = 0.0
                if original["knee"] != "hard":
                    notes.append(
                        {
                            "unsupported": {"knee": original["knee"]},
                            "adaptation": "explicit hard knee; ordinal console knee has no verified dB width",
                        }
                    )
                notes.append(
                    "Peak linked detector and exponential dB gain smoothing are ours; detector/attack equivalence unknown."
                )
                notes.append(
                    "Published release number retained as our e-fold time, an explicit adaptation of console 6 dB recovery time; 44.1 kHz table milliseconds retained at native rate."
                )
                notes.append(
                    "Published output gain applied once as makeup; no additional loss compensation."
                )
            decisions.append(
                {
                    "channel": i,
                    "file": ch["file"],
                    "reference": assignment[key],
                    "source": record,
                    "mapping": notes,
                    "applied": copy.deepcopy(ch[key]),
                }
            )
    return out, decisions


def adapt(session, windows, section_seconds=12):
    """One training-only threshold correction, never a search toward a produced mix.

    Only reduce excessive compression. Quiet/inactive sources cannot induce boosting.
    Require eight high-confidence active windows and p90 reduction > 6 dB.
    """
    rows = [[] for _ in session["channels"]]
    for row in windows:
        seconds = int(row["start_frame"]) / session["sample_rate"]
        if (
            int(seconds / section_seconds) % 2 == 0
            and row["active"] == "true"
            and float(row["confidence"]) >= 0.65
        ):
            rows[int(row["channel"])].append(float(row["max_reduction_db"]))
    out = copy.deepcopy(session)
    decisions = []
    for i, (ch, values) in enumerate(zip(out["channels"], rows)):
        old = ch["compressor"]["threshold_db"]
        p90 = sorted(values)[int(0.9 * (len(values) - 1))] if values else None
        delta = 0.0
        if len(values) >= 8 and p90 > 6 and ch["compressor"]["ratio"] > 1:
            delta = min(6.0, (p90 - 6.0) / (1 - 1 / ch["compressor"]["ratio"]))
            ch["compressor"]["threshold_db"] = min(0.0, old + delta)
        decisions.append(
            {
                "channel": i,
                "training_windows": len(values),
                "training_p90_max_gr_db": p90,
                "old_threshold_db": old,
                "new_threshold_db": ch["compressor"]["threshold_db"],
                "reason": "one bounded threshold increase for excessive measured action"
                if delta
                else "retain; no supported excess",
                "tradeoff": "less level control; EQ, makeup, faders and timing fixed",
            }
        )
    return out, decisions


def review(neutral_rows, candidate_rows, rate):
    """Compare identical confident neutral windows; candidate activity cannot select evidence."""
    candidates = {(int(r["start_frame"]), int(r["channel"])): r for r in candidate_rows}
    grouped = {}
    for row in neutral_rows:
        key = (int(row["start_frame"]), int(row["channel"]))
        if key not in candidates:
            raise ValueError("candidate timeline/channel inventory differs")
        other = candidates.pop(key)
        if row["active"] != "true" or float(row["confidence"]) < 0.65:
            continue
        split = "held_out" if int(key[0] / rate / 12) % 2 else "training"
        values = grouped.setdefault((key[1], split), [])
        values.append((row, other))
    if candidates:
        raise ValueError("candidate has extra windows")
    result = []
    for (channel, split), pairs in sorted(grouped.items()):
        body = [float(b["120_500_dbfs"]) - float(a["120_500_dbfs"]) for a, b in pairs]
        action = [float(b["mean_reduction_db"]) for _, b in pairs]
        hits = [(a, b) for a, b in pairs if a["onset"] == "true"]
        result.append(
            {
                "channel": channel,
                "split": split,
                "fixed_active_windows": len(pairs),
                "body_change_median_db": statistics.median(body),
                "compressor_mean_reduction_median_db": statistics.median(action),
                "neutral_event_windows": len(hits),
                "event_window_gr_median_db": statistics.median(
                    float(b["mean_reduction_db"]) for _, b in hits
                )
                if hits
                else None,
                "note": "Fixed neutral activity/event selection; role-band energy is a proxy, not listener preference.",
            }
        )
    return result


def excerpts(source, ours, reference, output, starts):
    """Exact PCM slices; reference has its original gain, format and independent provenance."""
    with wave.open(str(source), "rb") as a, wave.open(str(ours), "rb") as b:
        if a.getparams()[:4] != b.getparams()[:4]:
            raise ValueError("SOURCE and OUR MIX formats/timelines differ")
    output = Path(output)
    output.mkdir()
    reference_dir = output / "our-to-produced-reference"
    if reference:
        reference_dir.mkdir()
    manifest = []
    for n, start in enumerate(starts, 1):
        jobs = [("1-SOURCE", source, output), ("2-OUR-MIX", ours, output)]
        if reference:
            jobs += [
                ("1-OUR-MIX", ours, reference_dir),
                ("2-PRODUCED-REFERENCE", reference, reference_dir),
            ]
        for label, path, directory in jobs:
            if path is None:
                continue
            path = Path(path)
            with wave.open(str(path), "rb") as reader:
                rate = reader.getframerate()
                frame, count = round(start * rate), 12 * rate
                if frame < 0 or frame + count > reader.getnframes():
                    raise ValueError("excerpt exceeds source")
                reader.setpos(frame)
                pcm = reader.readframes(count)
                params = reader.getparams()
            dest = directory / f"{n:02}-{label}-{start:07.3f}s.wav"
            with wave.open(str(dest), "wb") as writer:
                writer.setparams(params)
                writer.writeframes(pcm)
            with wave.open(str(dest), "rb") as reader:
                assert reader.getnframes() == count and reader.readframes(count) == pcm
            manifest.append(
                {
                    "label": label,
                    "source": str(path.resolve()),
                    "file": str(dest.resolve()),
                    "start_frame": frame,
                    "frames": count,
                    "sample_rate": rate,
                    "pcm_sha256": hashlib.sha256(pcm).hexdigest(),
                    "gain_change_db": 0,
                }
            )
    write(
        output / "manifest.json",
        {
            "order": "SOURCE → OUR MIX; OUR MIX → PRODUCED REFERENCE separately",
            "reference_note": "Same supplied timeline timestamps; production edits/latency are not corrected or assumed sample-aligned.",
            "excerpts": manifest,
        },
    )


def main():
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="command", required=True)
    m = sub.add_parser("map")
    for key in ("session", "inventory", "assignments", "output"):
        m.add_argument(key)
    a = sub.add_parser("adapt")
    for key in ("session", "windows", "output"):
        a.add_argument(key)
    r = sub.add_parser("review")
    for key in ("neutral_windows", "candidate_windows", "output"):
        r.add_argument(key)
    r.add_argument("--rate", type=int, required=True)
    e = sub.add_parser("excerpts")
    for key in ("source", "ours", "output"):
        e.add_argument(key)
    e.add_argument("starts", type=float, nargs="+")
    e.add_argument("--reference")
    args = p.parse_args()
    if args.command == "review":
        with open(args.neutral_windows) as a, open(args.candidate_windows) as b:
            result = review(csv.DictReader(a), csv.DictReader(b), args.rate)
        write(args.output, result)
        return
    if args.command == "excerpts":
        excerpts(args.source, args.ours, args.reference, args.output, args.starts)
        return
    if args.command == "map":
        settings, decisions = derive(
            read(args.session), read(args.inventory), read(args.assignments)
        )
    else:
        with open(args.windows) as f:
            settings, decisions = adapt(read(args.session), csv.DictReader(f))
    out = Path(args.output)
    out.mkdir()
    write(out / "settings.json", settings)
    write(out / "decisions.json", decisions)


if __name__ == "__main__":
    main()
