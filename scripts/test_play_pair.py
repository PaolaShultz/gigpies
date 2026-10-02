import tempfile
import unittest
import wave
from pathlib import Path
from play_pair import run_pair


class PairTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.a, self.b = [Path(self.tmp.name) / n for n in ('a.wav', 'b.wav')]
        for p, value in [(self.a, 100), (self.b, 200)]:
            with wave.open(str(p), 'wb') as w:
                w.setparams((2, 2, 8000, 0, 'NONE', 'not compressed'))
                w.writeframes(value.to_bytes(2, 'little') * 16000)

    def test_preflight_then_one_pair_with_fixed_gap_and_no_mutation(self):
        originals = [x.read_bytes() for x in (self.a, self.b)]
        calls = []
        result = run_pair([self.a, self.b], 'test-sink',
                          sink_check=lambda t: calls.append(('sink', t)),
                          player=lambda c, t: calls.append(('play', Path(c['path']).name)),
                          pause=lambda t: calls.append(('gap', t)))
        self.assertEqual(calls, [('sink', 'test-sink'), ('play', 'a.wav'), ('gap', 1.), ('play', 'b.wav')])
        self.assertEqual(result['plays_per_clip'], 1)
        self.assertEqual([x.read_bytes() for x in (self.a, self.b)], originals)

    def test_bad_second_clip_or_missing_sink_starts_nothing(self):
        calls = []
        def player(*args):
            calls.append(args)
        original = self.b.read_bytes()
        self.b.write_bytes(b'bad')
        with self.assertRaises((EOFError, wave.Error)):
            run_pair([self.a, self.b], 'test', sink_check=lambda _: None, player=player)
        self.assertEqual(calls, [])
        self.b.write_bytes(original)
        def missing(_):
            raise RuntimeError('missing sink')
        with self.assertRaises(RuntimeError):
            run_pair([self.a, self.b], 'test', sink_check=missing, player=player)
        self.assertEqual(calls, [])

    def test_duplicate_and_bad_gap_rejected_before_playback(self):
        with self.assertRaises(ValueError):
            run_pair([self.a, self.a], 'unused')
        with self.assertRaises(ValueError):
            run_pair([self.a, self.b], 'unused', float('nan'))

    def test_failed_first_play_does_not_start_second_or_retry(self):
        calls = []
        def fail(c, _):
            calls.append(c['path'])
            raise RuntimeError('playback failed')
        with self.assertRaises(RuntimeError):
            run_pair([self.a, self.b], 'test', sink_check=lambda _: None, player=fail)
        self.assertEqual(len(calls), 1)
