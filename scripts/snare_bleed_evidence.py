#!/usr/bin/env python3
"""Summarize fixed snare-spill masks and candidate tradeoffs; never change audio/settings."""
import json
import statistics
import sys
from pathlib import Path


def median(values):
    values = list(values)
    return statistics.median(values) if values else None


def summarize(pilot):
    pilot = Path(pilot)
    baseline = json.loads((pilot / 'baseline.json').read_text())
    diagnosis = json.loads((pilot / 'diagnosis.json').read_text())
    events = json.loads((pilot / 'events.json').read_text())['events'][1]
    # Include weak competing rises as well as the chosen cluster anchor.
    rises = [t for e in events for t in e['rise_candidates_seconds']]
    level = diagnosis['direct_reference_level']
    residual = [i for i, f in enumerate(baseline['frames'])
                if level is not None and f['broad'][1] < level - 18
                and all(not t - .02 <= f['seconds'] <= t + .12 for t in rises)]
    frames = [baseline['frames'][i] for i in residual]
    result = {
        'scope': 'Inter-event residual includes bleed, possible quiet playing and snare tails; not isolated bleed labels.',
        'residual_frames': len(frames), 'nominal_residual_seconds': len(frames) * .01,
        'raw_snare_rms_median': median(f['broad'][1] for f in frames),
        'high_band_eq_rise_db': median(f['bands'][3][4] - f['bands'][1][4] for f in frames),
        'high_band_eq_plus_makeup_rise_db': median(f['bands'][4][4] - f['bands'][1][4] for f in frames),
        'class_counts': diagnosis['counts'], 'static_candidates': [],
    }
    for name in ['high-relief', 'body-relief', 'combined-relief']:
        path = pilot / name / 'measurement.json'
        if not path.exists():
            continue
        candidate = json.loads(path.read_text())
        if len(candidate['frames']) != len(baseline['frames']):
            raise ValueError('Candidate frame count differs')
        if any(a['seconds'] != b['seconds'] for a, b in zip(candidate['frames'], baseline['frames'])):
            raise ValueError('Candidate timeline differs')
        result['static_candidates'].append({
            'name': name,
            'median_residual_broadband_reduction_db': median(
                baseline['frames'][i]['broad'][4] - candidate['frames'][i]['broad'][4] for i in residual),
            'median_residual_high_band_reduction_db': median(
                baseline['frames'][i]['bands'][4][4] - candidate['frames'][i]['bands'][4][4] for i in residual),
            'outcome': json.loads((pilot / name / 'outcome.json').read_text()),
        })
    return result


if __name__ == '__main__':
    print(json.dumps(summarize(sys.argv[1]), indent=2))
