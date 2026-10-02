import copy
import unittest
import json
import tempfile
from pathlib import Path

from snare_temporal_evidence import context, evaluate, fit, read_review, support, vector


def fixture():
    features, events = [], []
    for i in range(18):
        t = 24.5 + i * .6
        features.append({'seconds': t, 'held_out': False, 'ambiguous_onset': False,
                         'raw_attack_db': -15., 'kick_distance_seconds': .5,
                         'phase_bands': [[[(-20 - b * 3 - p * 4) for b in range(5)]
                                          for _ in range(5)] for p in range(4)]})
        events.append({'seconds': t, 'rise_candidates_seconds': [t]})
    return {'direct_reference_level': -15., 'events': features,
            'listener_confirmed_bleed': True}, events


class TemporalEvidenceTests(unittest.TestCase):
    def test_silence_and_dense_training_abstain(self):
        d, ev = fixture()
        empty = copy.deepcopy(d)
        empty['events'] = []
        self.assertIsNone(fit(empty, [])['models']['temporal']['template'])
        for e in ev:
            e['rise_candidates_seconds'].append(e['seconds'] + .4)
        m = fit(d, ev)
        self.assertIsNone(m['models']['temporal']['template'])
        self.assertIsNotNone(m['models']['attack']['template'])

    def test_heldout_cannot_change_fitted_models(self):
        d, ev = fixture()
        before = fit(d, ev)
        e = copy.deepcopy(d['events'][0])
        e.update(seconds=40., held_out=True, raw_attack_db=20.)
        d['events'].append(e)
        ev.append({'seconds': 40., 'rise_candidates_seconds': [40.]})
        self.assertEqual(before, fit(d, ev))

    def test_quiet_same_shape_remains_unresolved(self):
        d, ev = fixture()
        m = fit(d, ev)
        quiet = copy.deepcopy(d['events'][0])
        quiet.update(seconds=40., held_out=True, raw_attack_db=-40., kick_distance_seconds=0.)
        quiet['phase_bands'] = [[[x - 25 for x in band] for band in phase]
                                for phase in quiet['phase_bands']]
        for a, b in zip(vector(d['events'][0], 4), vector(quiet, 4)):
            self.assertAlmostEqual(a, b)
        d['events'].append(quiet)
        ev.append({'seconds': 40., 'rise_candidates_seconds': [40.]})
        r = evaluate(d, ev, m)
        self.assertEqual(r['events'][-1]['source_identity'], 'unresolved_protected')
        self.assertIsNone(r['processing_candidate'])

    def test_precursor_rapid_repeat_and_split_are_protected(self):
        e = {'seconds': 1., 'ambiguous_onset': False}
        ev = [{'seconds': 1., 'rise_candidates_seconds': [1.]},
              {'seconds': 1.26, 'rise_candidates_seconds': [1.20, 1.26]}]
        self.assertIn('next_rise_within_support', support(e, ev, .24))
        e['ambiguous_onset'] = True
        self.assertIn('compound_onset', support(e, ev, .24))
        e.update(seconds=35.9, ambiguous_onset=False)
        self.assertIn('split_boundary', support(e, ev, .24))

    def test_equal_observations_can_hide_wanted_quiet_snare(self):
        # Two physically different source decompositions yield identical observations.
        # Model A: all target energy is spill. Model B: half is wanted unison playing.
        reference = [.0, .4, -.3, .2, -.1, .0]
        observed = [.2 * x for x in reference]
        wanted_b = [.1 * x for x in reference]
        spill_b = [.1 * x for x in reference]
        self.assertEqual(observed, [a + b for a, b in zip(wanted_b, spill_b)])
        # A perfectly predictive subtractor destroys the wanted component in B.
        residual = [y - .2 * x for x, y in zip(reference, observed)]
        self.assertTrue(all(x == 0 for x in residual))
        self.assertGreater(sum(x*x for x in wanted_b), 0)

    def test_invalid_evidence_cannot_earn_isolation(self):
        d, ev = fixture()
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp)
            (p / 'diagnosis.json').write_text(json.dumps(d))
            (p / 'events.json').write_text(json.dumps({'events': [[], ev]}))
            self.assertEqual(read_review(p), (d, ev))
            ev[0]['rise_candidates_seconds'] = []
            (p / 'events.json').write_text(json.dumps({'events': [[], ev]}))
            with self.assertRaises(ValueError):
                read_review(p)

    def test_gap_coverage_protects_tail_and_retained_precursor(self):
        m = {'stages': ['kick_raw'] + ['unused'] * 4 + ['snare_raw'], 'frames': []}
        for t in [1.15, 1.20, 1.40]:
            m['frames'].append({'seconds': t, 'duration': .01, 'rms': [-40.] * 15,
                                'max_gr': [0., 0.]})
        ev = [{'seconds': 1., 'rise_candidates_seconds': [1.]},
              {'seconds': 1.46, 'rise_candidates_seconds': [1.40, 1.46]}]
        r = context(m, ev, -15.)['sections'][0]
        self.assertAlmostEqual(r['residual_120ms_seconds'], .02)
        self.assertEqual(r['residual_240ms_seconds'], 0)
        self.assertAlmostEqual(r['kick_bass_overlap_seconds'], .03)

    def test_unknown_model_cannot_authorize_processing(self):
        d, ev = fixture()
        m = fit(d, ev)
        m['processing_permission'] = True
        with self.assertRaises(ValueError):
            evaluate(d, ev, m)


if __name__ == '__main__':
    unittest.main()
