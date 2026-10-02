import json
import math
import shutil
from pathlib import Path
import tempfile
import unittest
import wave

from listening_checkpoint import digest, prepare, verify_excerpt


class CheckpointTests(unittest.TestCase):
    def test_explicit_fx_tail_preserves_source_identity_and_clip_gain(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = self.fixture(root)
            final = root / "fx-final"
            shutil.copytree(root / "render", final)
            spec["examples"][0]["final_render"] = str(final)
            path = final / "processed.wav"
            with wave.open(str(path), "rb") as reader:
                params = reader.getparams()
                original = reader.readframes(params.nframes)
            with wave.open(str(path), "wb") as writer:
                writer.setparams(params)
                writer.writeframes(original + bytes(600 * 6))
            settings = json.loads((final / "prepared.json").read_text())
            settings["effects"] = {"tail_seconds": 6}
            (final / "prepared.json").write_text(json.dumps(settings))
            report = json.loads((final / "measurements.json").read_text())
            report["frames"] = 2600
            report["channels"][0]["tail_padding_frames"] = 600
            report["effects"] = {"source_frames": 2000, "tail_frames": 600}
            (final / "measurements.json").write_text(json.dumps(report))
            with self.assertRaises(ValueError):
                prepare(spec, root / "not-authorized")
            spec["examples"][0]["allow_fx_tail"] = True
            result = prepare(spec, root / "valid")
            exports = result["examples"][0]["exports"]
            self.assertEqual([x["frames"] for x in exports], [2000, 2600])
            self.assertEqual(exports[0]["sha256"], spec["examples"][0]["source_wav_sha256"])
            for bad in ("offset", "tail", "duration"):
                broken = json.loads(json.dumps(report))
                if bad == "offset":
                    broken["channels"][0]["offset_frames"] = 1
                elif bad == "tail":
                    broken["effects"]["tail_frames"] = 599
                else:
                    broken["frames"] = 2700
                    broken["channels"][0]["tail_padding_frames"] = 700
                    broken["effects"]["tail_frames"] = 700
                (final / "measurements.json").write_text(json.dumps(broken))
                with self.assertRaises(ValueError):
                    prepare(spec, root / bad)
                self.assertFalse((root / bad).exists())

    def fixture(self, root):
        render = root / "render"
        render.mkdir()
        peak = round(8388608 * 10 ** (-0.01 / 20))
        # Nonperiodic samples expose a one-frame shift; the isolated peak checks
        # that excerpts retain the parent gain instead of normalizing themselves.
        data = b"".join(x.to_bytes(3, "little", signed=True) * 2
                        for x in [peak] + list(range(1, 2000)))
        for stem in ["bypass", "processed"]:
            with wave.open(str(render / f"{stem}.wav"), "wb") as w:
                w.setparams((2, 3, 100, 0, "NONE", "not compressed"))
                w.writeframes(data)
        settings = dict(output_mode="unmatched", ceiling_db=-0.01, master_db=0,
                        calibration={"initial_trim_db": 0}, groups=[{"trim_db": 0}],
                        channels=[{"file": "original.wav", "pan": 0, "fader_db": 0}])
        report = dict(output_mode="unmatched", frames=2000, sample_rate=100,
                      channels=[dict(file="original.wav", bwf_time_reference=100,
                                     offset_frames=0, source_frames=2000, tail_padding_frames=0)],
                      bypass_export_gain_db=-2, processed_export_gain_db=4,
                      bypass_lufs=-20, processed_lufs=-22)
        for name, value in [("prepared.json", settings), ("measurements.json", report)]:
            (render / name).write_text(json.dumps(value))
        return dict(expected_ids=["example"], examples=[dict(id="example",
                    source_render=str(render), final_render=str(render), starts=[1],
                    source_settings_sha256=digest(render / "prepared.json"),
                    source_wav_sha256=digest(render / "bypass.wav"))])

    def test_later_faders_cannot_silently_redefine_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = self.fixture(root)
            path = root / "render/prepared.json"
            settings = json.loads(path.read_text())
            settings["channels"][0]["fader_db"] = -4
            path.write_text(json.dumps(settings))
            with self.assertRaisesRegex(ValueError, "pinned initial"):
                prepare(spec, root / "set")
            self.assertFalse((root / "set").exists())
            del spec["examples"][0]["source_settings_sha256"]
            with self.assertRaisesRegex(ValueError, "pin established"):
                prepare(spec, root / "set")

    def test_metadata_change_during_preparation_cannot_publish_ready(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = self.fixture(root)
            original_verify = verify_excerpt
            def change_metadata(*args):
                result = original_verify(*args)
                path = root / "render/measurements.json"
                report = json.loads(path.read_text())
                report["processed_export_gain_db"] += 1
                path.write_text(json.dumps(report))
                return result
            with patch("listening_checkpoint.verify_excerpt", side_effect=change_metadata):
                with self.assertRaisesRegex(ValueError, "metadata changed"):
                    prepare(spec, root / "set")
            self.assertFalse((root / "set/manifest.json").exists())

    def test_complete_inventory_exact_gain_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = self.fixture(root)
            result = prepare(spec, root / "set")
            self.assertTrue(result["complete_verified"])
            entry = result["examples"][0]
            self.assertEqual([e["export_gain_db"] for e in entry["exports"]], [-2, 4])
            self.assertEqual([e["bus_lufs_before_export"] for e in entry["exports"]], [-20, -22])
            for clip in entry["excerpts"]:
                self.assertEqual(clip["gain_change_db"], 0)
                verify_excerpt(clip["parent"], clip["file"], 100, 1200)
                with self.assertRaises(ValueError):
                    verify_excerpt(clip["parent"], clip["file"], 101, 1200)
            with self.assertRaises(FileExistsError):
                prepare(spec, root / "set")

    def test_missing_example_bad_span_and_peak_cannot_publish_ready(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = self.fixture(root)
            spec["expected_ids"].append("missing")
            with self.assertRaises(ValueError):
                prepare(spec, root / "set")
            spec["expected_ids"].pop()
            for start in [math.nan, -1, 10]:
                spec["examples"][0]["starts"] = [start]
                with self.assertRaises(ValueError):
                    prepare(spec, root / "set")
            spec["examples"][0]["starts"] = [1]
            path = root / "render/processed.wav"
            with wave.open(str(path), "wb") as w:
                w.setparams((2, 3, 100, 0, "NONE", "not compressed"))
                w.writeframes(b"\0" * 12000)
            with self.assertRaises(ValueError):
                prepare(spec, root / "set")
            self.assertFalse((root / "set").exists())

    def test_changed_pan_and_timeline_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = self.fixture(root)
            import shutil
            shutil.copytree(root / "render", root / "other")
            spec["examples"][0]["final_render"] = str(root / "other")
            path = root / "other/measurements.json"
            report = json.loads(path.read_text())
            report["channels"][0]["offset_frames"] = 1
            path.write_text(json.dumps(report))
            with self.assertRaises(ValueError):
                prepare(spec, root / "set")
            report["channels"][0]["offset_frames"] = 0
            path.write_text(json.dumps(report))
            path = root / "other/prepared.json"
            settings = json.loads(path.read_text())
            settings["channels"][0]["pan"] = 0.5
            path.write_text(json.dumps(settings))
            with self.assertRaises(ValueError):
                prepare(spec, root / "set")

    def test_corrupt_truncated_or_changed_gain_clip_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = prepare(self.fixture(root), root / "set")
            clip = result["examples"][0]["excerpts"][0]
            path = Path(clip["file"])
            original = path.read_bytes()
            for data in [original[:-6], original[:-1] + bytes([original[-1] ^ 1])]:
                path.write_bytes(data)
                with self.assertRaises(ValueError):
                    verify_excerpt(clip["parent"], path, 100, 1200)


if __name__ == "__main__":
    unittest.main()
