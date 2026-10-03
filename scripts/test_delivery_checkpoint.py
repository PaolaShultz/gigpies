import json
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import wave

from listening_checkpoint import digest, prepare
from verify_delivery import verify


class DeliveryTests(unittest.TestCase):
    def fixture(self, root):
        directory = root / "delivery"
        directory.mkdir()
        sample = 838861
        with wave.open(str(directory / "processed.wav"), "wb") as wav:
            wav.setparams((2, 3, 100, 0, "NONE", "not compressed"))
            wav.writeframes(sample.to_bytes(3, "little", signed=True) * 4000)
        (directory / "processed-unity-float.wav").write_bytes(b"synthetic bus identity")
        (directory / "report.txt").write_text("synthetic delivery summary")
        meter = dict(frames=2000, sample_rate=100,
                     sample_peak_dbfs=20 * math.log10(sample / 8388608), true_peak_dbtp=-20.)
        policy = dict(version=1, peak_basis="true", ceiling_db=-1., comparison={"mode": "none"},
                      sample_format="pcm24", dither="none")
        primary = dict(input=meter, output=meter, ceiling_db=-1., gain_db=-1.,
                       pcm_sha256=digest(directory / "processed.wav"),
                       bus_sha256=digest(directory / "processed-unity-float.wav"))
        report = dict(contract="gigpies-delivery-v1", primary=primary, comparison_copies={})
        for name, value in [("prepared.json", {}), ("measurements.json", {}), ("sources.json", []),
                            ("delivery-policy.json", policy), ("delivery.json", report)]:
            (directory / name).write_text(json.dumps(value))
        self.repin(directory)
        return directory, dict(contract="gigpies-delivery-v1", expected_ids=["example"],
                               examples=[dict(id="example", delivery=str(directory), starts=[1])])

    def repin(self, directory):
        names = ["prepared.json", "measurements.json", "sources.json", "delivery-policy.json",
                 "delivery.json", "report.txt", "processed.wav", "processed-unity-float.wav"]
        (directory / "delivery-ready.json").write_text(json.dumps(dict(
            version=1, contract="gigpies-delivery-v1", files={n: digest(directory / n) for n in names})))

    def measure(self, directory, peak=-20.):
        with patch("verify_delivery.subprocess.run") as run:
            run.return_value.stdout = "fake ffmpeg for unit test\n"
            return verify(directory, lambda path, rate: (peak, ["injected-test-meter"]))

    def test_exact_policy_clip_and_required_independent_evidence(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            directory, spec = self.fixture(root)
            with self.assertRaises(FileNotFoundError):
                prepare(spec, root / "missing")
            self.measure(directory)
            result = prepare(spec, root / "valid")
            self.assertTrue(result["complete_verified"])
            self.assertTrue(result["true_peak_compliance"])
            self.assertEqual(result["examples"][0]["excerpts"][0]["gain_change_db"], 0)
            (directory / "report.txt").write_text("stale historical export gain")
            with self.assertRaises(ValueError):
                prepare(spec, root / "stale-summary")
            (directory / "report.txt").write_text("synthetic delivery summary")
            (directory / "processed.wav").write_bytes(b"changed")
            with self.assertRaises(ValueError):
                prepare(spec, root / "stale")
            self.assertFalse((root / "stale").exists())

    def test_over_ceiling_and_contradictory_or_legacy_reports_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            directory, spec = self.fixture(root)
            with self.assertRaises(ValueError):
                self.measure(directory, -0.99)
            with self.assertRaisesRegex(ValueError, "disagreement"):
                self.measure(directory, -1.5)
            self.assertFalse((directory / "independent-verification.json").exists())
            self.measure(directory)
            policy = json.loads((directory / "delivery-policy.json").read_text())
            policy["ceiling_db"] = -3.
            (directory / "delivery-policy.json").write_text(json.dumps(policy))
            self.repin(directory)
            with self.assertRaises(ValueError):
                prepare(spec, root / "contradictory")
            (directory / "delivery-ready.json").write_text('{"true_peak":false}')
            with self.assertRaises(ValueError):
                prepare(spec, root / "legacy")

    def test_unknown_contract_cannot_fall_back_to_legacy(self):
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaises(ValueError):
                prepare({"contract": "unrecognized"}, Path(temp) / "out")


if __name__ == "__main__":
    unittest.main()
