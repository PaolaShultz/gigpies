#!/usr/bin/env python3
"""Opt-in synthetic Tap -> Lux -> Lightdesk check. No audio, display or DMX devices."""
import argparse
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import time
from pathlib import Path


def run(args):
    spec = importlib.util.spec_from_file_location("lux_check", args.lux_check_helper)
    cli = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cli)
    started = time.monotonic()
    children = []
    client = None
    with tempfile.TemporaryDirectory(prefix="gp-music-") as name:
        root = Path(name).resolve()
        source, owner = root / "source", root / "lux"
        source.mkdir(mode=0o700)
        owner.mkdir(mode=0o700)

        source_log = tempfile.TemporaryFile()
        timeline = []

        def launch(command, stderr=subprocess.DEVNULL):
            child = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.DEVNULL, stderr=stderr)
            children.append(child)
            return child

        def endpoint(path, child):
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                if child.poll() is not None:
                    raise RuntimeError(f"child startup failed: exit {child.returncode}")
                if path.exists():
                    return
                time.sleep(.01)
            raise RuntimeError("bounded endpoint startup deadline")

        try:
            producer = launch([str(args.source), str(source), "30"], source_log)
            endpoint(source / "analysis.sock", producer)
            lux = launch([str(args.lux), "--synthetic-private-dir", str(owner),
                          "--timed", "--analysis", str(source / "analysis.sock"), "--musical-analysis"])
            endpoint(owner / "lux.sock", lux)
            client = cli.Client(owner)
            client.write("grant", dict(scope=cli.SCOPE))
            renew_at = time.monotonic() + .5
            last_state = None

            def snapshot():
                nonlocal renew_at, last_state
                if time.monotonic() - started > 50:
                    raise RuntimeError("bounded integration deadline")
                inv = client.snapshot()
                state = (inv["analysis"]["state"], inv["analysis"]["reason"], inv["analysis"]["generation"])
                if state != last_state:
                    event = dict(elapsed_seconds=round(time.monotonic()-started, 3), state=state,
                                 source_window=inv["analysis"]["source_window_range"], losses=inv["analysis"]["losses"])
                    if len(timeline) < 128:
                        timeline.append(event)
                    print("analysis", event, flush=True)
                    last_state = state
                if time.monotonic() >= renew_at:
                    client.write("renew", dict(scope=cli.SCOPE))
                    renew_at = time.monotonic() + .5
                return inv

            def wait(predicate, seconds):
                deadline = time.monotonic() + seconds
                while time.monotonic() < deadline:
                    inv = snapshot()
                    if predicate(inv):
                        return inv
                    time.sleep(.02)
                source_log.seek(0)
                raise AssertionError(f"condition timeout: timeline={timeline}; source={source_log.read(16384).decode()}; analysis={inv['analysis']}")

            def action(command):
                for _ in range(20):
                    snapshot()
                    reply = client.exchange(client.envelope("command", dict(scope=cli.SCOPE, command=command), True))[0]
                    client.next_id += 1
                    client.revision = reply["revision"]
                    if reply["kind"] == "applied":
                        return
                    assert reply["reason"] == "stale_revision", reply
                raise AssertionError("bounded command contention")

            def lightdesk():
                result = subprocess.run([str(args.lightdesk), "--provider", "lux", "--socket-path",
                    str(owner / "lux.sock"), "--show-id", cli.SHOW, "--epoch", client.epoch],
                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=4, check=True)
                assert len(result.stdout) <= 262144
                return result.stdout.decode()

            wait(lambda i: i["analysis"]["source_identity"] is not None, 3)
            action(dict(action="analysis_calibrate", phase="start"))
            wait(lambda i: i["analysis"]["calibration_windows"] >= 60, 3)
            action(dict(action="analysis_calibrate", phase="finish"))
            action(dict(action="touch", values=[dict(fixture="fixture-11", attribute="intensity", value=333)]))
            action(dict(action="clear_to_hold"))
            tracked = wait(lambda i: i["analysis"]["musical"]["state"] == "tracking"
                           and i["analysis"]["musical"]["chord"] is not None
                           and i["analysis"]["musical"]["key"] is not None, 22)
            visible = lightdesk()
            assert "Chord estimate" in visible and "Key estimate" in visible and "Camelot" in visible
            if args.offscreen_evidence is not None:
                script = root / "health.txt"
                script.write_text("@page health\n")
                subprocess.run([str(args.lightdesk), "offscreen", "--socket-path", str(owner / "lux.sock"),
                    "--show-id", cli.SHOW, "--epoch", client.epoch, "--script", str(script),
                    "--output", str(args.offscreen_evidence)], stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5, check=True)
            producer.terminate()
            producer.wait(timeout=3)
            lost = wait(lambda i: i["analysis"]["state"] == "absent", 3)
            music = lost["analysis"]["musical"]
            assert music["state"] == "unknown" and music["source_frame"] is None
            assert music["chroma"] == [0] * 12
            assert all(music[k] is None for k in ("chord", "key", "movement", "section"))
            attr = lost["snapshot"]["fixtures"][0]["attributes"][0]
            assert attr["hold"] == 333 and attr["resolved"] == 333 and attr["source"] == "hold"
            assert lost["output"] == "null_disarmed" and lost["physical"] == "unknown"
            assert "Music unknown" in lightdesk()
            source_log.seek(0)
            source_diagnostics = source_log.read(16385)
            assert len(source_diagnostics) <= 16384
            paths = [args.source, args.lux, args.lightdesk, args.lux_check_helper]
            return dict(state="passed", sha256={str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
                        state_transitions=timeline, source_diagnostics=source_diagnostics.decode(),
                        musical=tracked["analysis"]["musical"], source_loss=music,
                        manual_hold=attr["hold"], elapsed_seconds=round(time.monotonic()-started, 3),
                        limits=["synthetic PCM24", "private same-host UDS", "null/disarmed", "no hardware or load qualification"])
        finally:
            if client is not None:
                client.close()
            for child in reversed(children):
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait(timeout=2)
            source_log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "lux", "lightdesk", "lux-check-helper"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--offscreen-evidence", type=Path, help="optional ignored directory for actual provider Health SVG/PPM/JSON")
    args = parser.parse_args()
    for name in ("source", "lux", "lightdesk", "lux_check_helper"):
        path = getattr(args, name)
        if not path.is_absolute() or not path.is_file():
            parser.error(f"{name} requires an explicit absolute file")
    result = run(args)
    args.evidence.write_text(json.dumps(result, indent=2) + "\n")
    print("PASS real synthetic music source, Lux estimates, Lightdesk presentation and source-loss Hold")


if __name__ == "__main__":
    main()
