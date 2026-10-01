import tempfile
import unittest
import wave
from pathlib import Path

from preset_experiment import adapt, derive, excerpts, review


class ExperimentTests(unittest.TestCase):
    def session(self):
        return {
            "sample_rate": 1000,
            "channels": [
                {
                    "file": "synthetic.wav",
                    "eq": [],
                    "compressor": {"threshold_db": -20, "ratio": 4, "makeup_db": 2},
                }
            ],
        }

    def test_confidence_quiet_silence_and_heldout_cannot_change_threshold(self):
        s = self.session()
        for active, confidence, start in [
            ("false", 1, 0),
            ("true", 0.35, 0),
            ("true", 1, 12000),
        ]:
            rows = [
                {
                    "start_frame": start,
                    "channel": 0,
                    "active": active,
                    "confidence": confidence,
                    "max_reduction_db": 20,
                }
            ] * 20
            out, _ = adapt(s, rows)
            self.assertEqual(s, out)
        rows = [
            {
                "start_frame": 0,
                "channel": 0,
                "active": "true",
                "confidence": 1,
                "max_reduction_db": 20,
            }
        ] * 7
        self.assertEqual(adapt(s, rows)[0], s)
        rows *= 2
        out, _ = adapt(s, rows)
        self.assertEqual(out["channels"][0]["compressor"]["threshold_db"], -14)
        self.assertEqual(out["channels"][0]["compressor"]["makeup_db"], 2)
        self.assertEqual(s, self.session())

    def test_unknown_parameters_retained_and_source_index_checked(self):
        original = {"bands": [{"type": "UNSUPPORTED", "hz": 123, "q": None, "db": 2}]}
        inv = {
            "test": {
                "manufacturer": "Synthetic",
                "product": "test",
                "preset": "test",
                "source": "test://fixture",
                "page": 1,
                "original": original,
            }
        }
        assignment = [{"channel": 0, "file": "synthetic.wav", "eq": "test"}]
        neutral = self.session()
        neutral["channels"][0]["compressor"].update(ratio=1, makeup_db=0)
        out, decisions = derive(neutral, inv, assignment)
        self.assertEqual(out["channels"][0]["eq"], [])
        self.assertEqual(decisions[0]["source"]["original"], original)
        with self.assertRaises(ValueError):
            derive(neutral, inv, assignment * 2)
        with self.assertRaises(ValueError):
            derive(self.session(), inv, assignment)
        assignment[0]["file"] = "wrong.wav"
        with self.assertRaises(ValueError):
            derive(self.session(), inv, assignment)

    def test_review_uses_neutral_selection_and_rejects_different_timelines(self):
        a = dict(
            start_frame=12000,
            channel=0,
            active="true",
            confidence=0.85,
            onset="true",
            mean_reduction_db=0,
            **{"120_500_dbfs": -30},
        )
        b = dict(
            a,
            active="false",
            confidence=0,
            mean_reduction_db=3,
            **{"120_500_dbfs": -27},
        )
        result = review([a], [b], 1000)
        self.assertEqual(result[0]["split"], "held_out")
        self.assertEqual(result[0]["body_change_median_db"], 3)
        self.assertEqual(result[0]["event_window_gr_median_db"], 3)
        with self.assertRaises(ValueError):
            review([a], [], 1000)

    def test_excerpts_preserve_samples_and_refuse_overwrite(self):
        with tempfile.TemporaryDirectory() as d:
            source = Path(d) / "s.wav"
            with wave.open(str(source), "wb") as w:
                w.setparams((2, 2, 1000, 0, "NONE", "not compressed"))
                w.writeframes(b"\x12\x01\x34\x02" * 24000)
            out = Path(d) / "out"
            excerpts(source, source, source, out, [1])
            with wave.open(str(next(out.glob("*SOURCE*"))), "rb") as r:
                self.assertEqual(r.readframes(12000), b"\x12\x01\x34\x02" * 12000)
            with self.assertRaises(FileExistsError):
                excerpts(source, source, None, out, [1])


if __name__ == "__main__":
    unittest.main()
