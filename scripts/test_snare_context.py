import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from snare_context import contribution, report


class SnareContextTests(unittest.TestCase):
    def test_coherent_omission_can_increase_level_and_silence_abstains(self):
        result = contribution(1, .81, .01)
        self.assertGreater(result['omission_level_change_db'], 19)
        self.assertAlmostEqual(result['signed_cross_power'], -1.8)
        result = contribution(0, 0, 0)
        self.assertIsNone(result['close_over_remainder_db'])
        self.assertIsNone(result['omission_level_change_db'])

    def fixture(self, root):
        (root / 'scope.json').write_text(json.dumps({'mode': 'source_bypass'}))
        events = [{'seconds': x, 'held_out': bool(x // 12 % 2), 'raw_attack_db': -20 + x,
                   'ambiguous_onset': False, 'next_seconds': .5} for x in (1, 2, 13, 14)]
        path = root / 'diagnosis.json'
        path.write_text(json.dumps({'events': [[], events]}))
        groups, inputs = {}, {}
        for event in events:
            for offset in (0, 2, 4, 6):
                frame = event['seconds'] * 100 + offset
                inputs[frame, 0] = {'frames': 2, 'input_dbfs': -20}
                for group in ('lead_vocal', 'snare', 'drums_without_snare', 'drums', 'ensemble',
                              'ensemble_without_snare', 'overheads', 'drum_room', 'toms', 'drum_ambience',
                              'guitars', 'bass', 'vocal_sum'):
                    groups[frame, group] = {band: -30 for band in
                        ('broadband_dbfs', '120_500_dbfs', '1500_5000_dbfs')}
        spec = {'rate': 100, 'source_analysis': str(root / 'source'), 'diagnosis': str(path),
                'variants': [{'id': 'baseline', 'analysis': str(root / 'baseline')}]}
        return spec, groups, inputs, events

    def test_held_out_changes_cannot_redefine_training_or_quiet_strata(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            spec, groups, inputs, events = self.fixture(root)
            with patch('snare_context.read_analysis', return_value=(groups, inputs)):
                before = report(spec, 'training')
                for event in events:
                    if event['held_out']:
                        event['raw_attack_db'] = 100
                Path(spec['diagnosis']).write_text(json.dumps({'events': [[], events]}))
                after = report(spec, 'training')
                self.assertEqual(before, after)
                self.assertEqual(before['variants'][0]['subsets']['all']['events'], 2)
                self.assertIsNone(before['listener_preference'])

    def test_changed_input_or_processed_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            spec, groups, inputs, _ = self.fixture(root)
            bad = {k: {**v, 'input_dbfs': -10} for k, v in inputs.items()}
            with patch('snare_context.read_analysis', side_effect=[(groups, inputs), (groups, bad)]):
                with self.assertRaisesRegex(ValueError, 'raw source differs'):
                    report(spec, 'training')
            (root / 'scope.json').write_text(json.dumps({'mode': 'processed'}))
            with self.assertRaisesRegex(ValueError, 'SOURCE bypass'):
                report(spec, 'training')
