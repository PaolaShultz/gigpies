#!/usr/bin/env python3
"""Explicit finite synthetic GigPies -> Lux -> Lightdesk scene episode, null output."""
import argparse
import copy
import hashlib
import importlib.util
import json
import socket
import subprocess
import tempfile
import time
from pathlib import Path


def run(args):
    spec = importlib.util.spec_from_file_location("lux_check", args.lux_check_helper)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    children = []
    client = None
    started = time.monotonic()
    events = []
    observations = []
    with tempfile.TemporaryDirectory(prefix="gp-scenes-") as temporary:
        root = Path(temporary).resolve()
        source, owner = root / "source", root / "owner"
        source.mkdir(mode=0o700)
        owner.mkdir(mode=0o700)
        source_log, owner_log = tempfile.TemporaryFile(), tempfile.TemporaryFile()

        def launch(command, log):
            child = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                     stdout=subprocess.DEVNULL, stderr=log)
            children.append(child)
            return child

        def endpoint(path, child):
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                if child.poll() is not None:
                    raise AssertionError(f"child startup exit {child.returncode}")
                if path.exists():
                    return
                time.sleep(.01)
            raise AssertionError("endpoint startup deadline")

        connected_at = renew_at = 0
        last_event = None

        def scoped_write(kind):
            for _ in range(25):
                client.snapshot()
                reply = client.exchange(client.envelope(kind, dict(scope=helper.SCOPE), True))[0]
                client.next_id += 1
                client.revision = reply["revision"]
                client.log.clear()
                if reply["kind"] == "applied":
                    if kind == "grant":
                        client.lease = reply["body"]["lease"]
                    return reply
                if reply.get("reason") != "stale_revision":
                    raise AssertionError(reply)
            raise AssertionError("bounded lease revision contention")

        def snapshot():
            nonlocal connected_at, renew_at, last_event
            now = time.monotonic()
            assert now - started < 130, "finite episode deadline"
            # Rotate only this client's transport before the owner's 30s bound;
            # preserve writer/lease/request sequence, with no mutation replay.
            if now - connected_at >= 20:
                client.stream.close()
                client.stream = socket.socket(socket.AF_UNIX)
                client.stream.settimeout(3)
                client.stream.connect(str(owner / "lux.sock"))
                connected_at = now
            inventory = client.snapshot()
            client.log.clear()
            if client.lease is not None and now >= renew_at:
                scoped_write("renew")
                renew_at = now + .5
            scene = inventory.get("scenes", {})
            identity = (inventory["analysis"]["generation"], scene.get("current"),
                        scene.get("reason"), scene.get("evidence", {}).get("section_id"))
            if identity != last_event:
                row = dict(seconds=round(now - started, 3), identity=identity,
                           evidence=scene.get("evidence"), interpretation=scene.get("interpretation"))
                assert len(events) < 1024, "bounded event evidence"
                events.append(row)
                print(json.dumps(row), flush=True)
                last_event = identity
            return inventory

        def action(command, expected=None):
            for _ in range(25):
                snapshot()
                reply = client.exchange(client.envelope("command", dict(scope=helper.SCOPE, command=command), True))[0]
                client.next_id += 1
                client.revision = reply["revision"]
                client.log.clear()
                if reply["kind"] == "applied":
                    assert expected is None, ("unexpected application", command)
                    return reply["body"]
                if reply.get("reason") == "stale_revision":
                    continue
                if expected is not None:
                    assert expected in reply["reason"], reply
                    return reply
                raise AssertionError(reply)
            raise AssertionError("bounded revision contention")

        def wait(predicate, seconds):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                inventory = snapshot()
                if predicate(inventory):
                    return inventory
                time.sleep(.04)
            raise AssertionError(("condition deadline", inventory.get("analysis"), inventory.get("scenes")))

        def desk_read():
            result = subprocess.run([str(args.lightdesk), "--provider", "lux", "--socket-path",
                str(owner / "lux.sock"), "--show-id", helper.SHOW, "--epoch", client.epoch],
                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                timeout=5, check=True)
            assert len(result.stdout) <= 262144
            return result.stdout.decode()

        def render(name):
            script = root / "view.txt"
            script.write_text("@page scenes\n")
            destination = args.preview / name
            subprocess.run([str(args.lightdesk), "offscreen", "--socket-path", str(owner / "lux.sock"),
                "--show-id", helper.SHOW, "--epoch", client.epoch, "--script", str(script),
                "--output", str(destination)], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, timeout=5, check=True)
            snapshot()
            return str(destination)

        try:
            producer = launch([str(args.source), str(source), "120", "scenes"], source_log)
            endpoint(source / "analysis.sock", producer)
            command = [str(args.lux), "--synthetic-private-dir", str(owner), "--durable",
                       "--analysis", str(source / "analysis.sock"), "--musical-analysis", "--scene-director"]
            lux = launch(command, owner_log)
            endpoint(owner / "lux.sock", lux)
            client = helper.Client(owner)
            connected_at = time.monotonic()
            client.write("grant", dict(scope=helper.SCOPE))
            initial = snapshot()
            assert initial["wire_schema"] == "lx06-scenes-v1"
            baseline_map = copy.deepcopy(initial["scenes"]["map"])
            malformed = copy.deepcopy(baseline_map)
            malformed["revision"] = str(int(baseline_map["revision"]) + 1)
            malformed["scenes"][0]["fade_ms"] = 0
            action(dict(action="scene_map_replace", map=malformed), expected="scene")
            assert snapshot()["scenes"]["map"] == baseline_map
            preview = action(dict(action="scene_preview", scene_id="driving"))
            assert preview["physical"] == "unknown" and preview["values"]
            assert snapshot()["scenes"]["map"] == baseline_map
            wait(lambda i: i["analysis"]["source_identity"] is not None, 3)
            action(dict(action="analysis_calibrate", phase="start"))
            wait(lambda i: i["analysis"]["calibration_windows"] >= 60, 3)
            action(dict(action="analysis_calibrate", phase="finish"))
            wait(lambda i: i["analysis"]["state"] == "ready", 3)
            action(dict(action="mode", mode="auto"))
            next_grant = 0
            next_sample = 0
            render_paths = []
            while True:
                now = time.monotonic()
                inv = snapshot()
                assert inv["analysis"]["state"] == "ready", ("unexpected source loss", inv["analysis"])
                source_frame = int(inv["analysis"]["source_window_range"]["end_frame_exclusive"])
                if source_frame >= 100 * 48000:
                    break
                if now >= next_grant:
                    # Explicit repeated grants in this finite test, never production auto-renewal.
                    action(dict(action="scene_grant", fixtures=["fixture-11"],
                                intensity_cap=800, color_cap=900, ttl_ms=2000))
                    next_grant = now + 1
                if now >= next_sample:
                    scene = inv["scenes"]
                    fixture = inv["snapshot"]["fixtures"][0]
                    observations.append(dict(seconds=round(now-started, 3), scene=scene["current"],
                        section=scene["evidence"]["section_id"], reason=scene["reason"],
                        transition=scene["transition"], intended=scene["intended_values"],
                        attributes=fixture["attributes"]))
                    next_sample = now + .5
                if not render_paths and now-started > 40:
                    assert "scene" in desk_read().lower()
                    render_paths.append(render("active"))
                time.sleep(.1)
            seen = {o["scene"] for o in observations if o["scene"] is not None}
            assert len(seen) >= 3, ("multiple actual scene choices", seen)
            # A stable recurring analyzer ID must regain the same scene identity.
            stable = [o for o in observations if o["section"] is not None and o["transition"] is None
                      and o["reason"] in ("current_scene_fits", "stable_evidence_hold")]
            returns = []
            for row in stable:
                old = next((x for x in stable if x["section"] == row["section"]
                            and row["seconds"]-x["seconds"] > 20
                            and any(x["seconds"] < y["seconds"] < row["seconds"]
                                    and y["section"] != row["section"] for y in stable)), None)
                if old is not None and old["scene"] == row["scene"]:
                    returns.append(dict(section=row["section"], scene=row["scene"],
                                        first=old["seconds"], returned=row["seconds"]))
                    break
            assert returns, "no supported recurring section recovered its visual identity"
            resolved = {(a["attribute"], a["resolved"]) for o in observations for a in o["attributes"]
                        if isinstance(a["source"], dict) and "auto" in a["source"]}
            assert len({v for a,v in resolved if a == "intensity"}) > 3
            assert any(a in ("red", "green", "blue") and v > 0 for a,v in resolved), "scene RGB must reach real authority"
            action(dict(action="touch", values=[dict(fixture="fixture-11", attribute="intensity", value=333),
                                                 dict(fixture="fixture-11", attribute="red", value=111),
                                                 dict(fixture="fixture-11", attribute="green", value=222),
                                                 dict(fixture="fixture-11", attribute="blue", value=333)]))
            action(dict(action="clear_to_hold"))
            held = snapshot()
            by_attr = {a["attribute"]: a for a in held["snapshot"]["fixtures"][0]["attributes"]}
            assert by_attr["intensity"]["resolved"] == 333 and by_attr["red"]["resolved"] == 111
            assert by_attr["intensity"]["source"] == by_attr["red"]["source"] == "hold"
            producer.terminate()
            producer.wait(timeout=3)
            lost = wait(lambda i: i["analysis"]["state"] == "absent", 3)
            assert lost["scenes"]["grant"] is None
            assert not lost["scenes"]["evidence"]["available"]
            action(dict(action="checkpoint"))
            render_paths.append(render("held-source-loss"))
            # Hand writer authority to the actual Lightdesk editor, with output held.
            scoped_write("retire")
            client.lease = None
            editor_script = root / "authoring.txt"
            editor_script.write_text("grant\n@page scenes\n"
                "scene edit driving mood dark\n@review-all\n@confirm\n@up\n"
                "scene edit driving lift 850\n@review-all\n@confirm\n@up\n"
                "scene edit driving rgb stage 120 40 280\n@review-all\n@confirm\n@up\n"
                "scene preview driving\n@page scenes\n")
            authored_preview = args.preview / "authored-dark-lift"
            role_dir = root / "roles"
            role_dir.mkdir(mode=0o700)
            role_request = root / "role-request.json"
            role_request.write_text(json.dumps(dict(format="gigpies-role-acquire", version=1,
                expected_generation="0", binding=dict(role="lighting-desk", display_connector="synthetic-display",
                    display_edid="synthetic-edid", controller="synthetic-controller", profile="test-profile",
                    operator_label=None), inventory=dict(displays=[dict(connector="synthetic-display",
                    edid="synthetic-edid",operator_label=None)], controllers=[dict(identity="synthetic-controller",
                    serial="synthetic-serial", profile="test-profile",operator_label=None)]))))
            subprocess.run([str(args.lightdesk), "offscreen", "--socket-path", str(owner / "lux.sock"),
                "--show-id", helper.SHOW, "--epoch", client.epoch, "--script", str(editor_script),
                "--output", str(authored_preview), "--role-provider", str(args.role_provider),
                "--role-dir", str(role_dir), "--role-request", str(role_request)], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, timeout=10, check=True)
            render_paths.append(str(authored_preview))
            captured = json.loads((authored_preview / "lux.json").read_text())
            assert captured["scene_preview"]["scene_id"] == "driving"
            preview_values = {v["attribute"]: v["value"] for v in captured["scene_preview"]["values"]}
            assert preview_values["intensity"] == 850 and preview_values["red"] == 120
            authored = snapshot()["scenes"]["map"]
            driving = next(s for s in authored["scenes"] if s["id"] == "driving")
            assert driving["mood"] == "dark" and driving["lift"] == 850
            assert driving["roles"][0]["rgb"] == [120,40,280]
            assert authored["revision"] == str(int(baseline_map["revision"])+3)
            before_epoch = int(client.epoch)
            client.close()
            client = None
            lux.terminate()
            lux.wait(timeout=3)
            (owner / "lux.sock").unlink(missing_ok=True)
            lux = launch(command, owner_log)
            endpoint(owner / "lux.sock", lux)
            client = helper.Client(owner)
            connected_at = time.monotonic()
            recovered = snapshot()
            assert int(client.epoch) > before_epoch
            assert recovered["scenes"]["map"] == authored
            assert recovered["scenes"]["grant"] is None
            assert recovered["scenes"]["current"] is None
            recovered_attrs = {a["attribute"]: a for a in recovered["snapshot"]["fixtures"][0]["attributes"]}
            assert recovered_attrs["intensity"]["resolved"] == 333
            assert recovered_attrs["red"]["resolved"] == 111
            source_log.seek(0)
            diagnostics = source_log.read(32769)
            assert len(diagnostics) <= 32768
            return dict(result="passed", elapsed_seconds=round(time.monotonic()-started, 3),
                sha256={str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in [args.source,args.lux,args.lightdesk,args.lux_check_helper,args.role_provider]},
                events=events, observations=observations, recurring=returns, scenes=sorted(seen),
                preview=preview, render_paths=render_paths, source_diagnostics=diagnostics.decode(),
                recovery=dict(epoch=client.epoch, map_revision=recovered["scenes"]["map"]["revision"]),
                limits=["synthetic PCM24 only", "same-host private IPC", "null/disarmed",
                        "no rehearsal accuracy, sustained-load, display or physical fixture qualification"])
        except Exception as error:
            source_log.seek(0)
            owner_log.seek(0)
            args.evidence.write_text(json.dumps(dict(result="failed", error=repr(error),
                events=events, observations=observations,
                child_stderr=getattr(error, "stderr", b"").decode(errors="replace") if isinstance(getattr(error,"stderr",None),bytes) else None,
                source_diagnostics=source_log.read(32768).decode(errors="replace"),
                owner_diagnostics=owner_log.read(32768).decode(errors="replace")), indent=2) + "\n")
            raise
        finally:
            if client is not None:
                client.close()
            for child in reversed(children):
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait(timeout=3)
            source_log.close()
            owner_log.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "lux", "lightdesk", "lux-check-helper", "role-provider", "evidence", "preview"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    for name in ("source", "lux", "lightdesk", "lux_check_helper", "role_provider"):
        path = getattr(args, name)
        if not path.is_absolute() or not path.is_file():
            parser.error(f"{name} needs an explicit absolute file")
    if not args.evidence.is_absolute() or not args.preview.is_absolute():
        parser.error("absolute ignored evidence and preview destinations required")
    args.evidence.parent.mkdir(parents=True, exist_ok=True)
    args.preview.mkdir(parents=True, exist_ok=True)
    result = run(args)
    args.evidence.write_text(json.dumps(result, indent=2) + "\n")
    print("PASS synthetic scenes, actual logical RGB/intensity, recurrence, Hold, source loss and recovery")


if __name__ == "__main__":
    main()
