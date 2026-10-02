#!/usr/bin/env python3
"""Summarize saved production-DSP measurements; never select processing or play audio."""
import json
import math
import statistics
import sys
from pathlib import Path


def median(xs):
    xs = list(xs)
    return statistics.median(xs) if xs else None


def band_sum(x, indices):
    return 10 * math.log10(max(1e-24, sum(10 ** (x[i] / 10) for i in indices)))


def summarize(root, baseline=False):
    root = Path(root)
    m = json.loads((root / ('baseline.json' if baseline else 'measurement.json')).read_text())
    d = json.loads((root / ('baseline-diagnosis.json' if baseline else 'diagnosis.json')).read_text())
    result = {'pcm_contacts': m['pcm_contacts'], 'drums': [], 'sections': [], 'bass_pitch_overlap': {}}
    for drum in range(2):
        raw = drum * 5
        events = d['events'][drum]
        levels = sorted(e['raw_attack_db'] for e in events)
        lo = levels[len(levels)//4] if levels else -65
        hi = levels[3*len(levels)//4] if levels else -65
        groups = {'all': events, 'quiet': [e for e in events if e['raw_attack_db'] <= lo],
                  'strong': [e for e in events if e['raw_attack_db'] >= hi],
                  'rapid': [e for e in events if e['next_seconds'] is not None and e['next_seconds'] < .24]}
        stats = {name: {'count': len(es), **{key: median(e[key] for e in es if e[key] is not None)
                 for key in ['attack_gr_db', 'body_gr_db', 'recovery_gr_db', 'pre_hit_gr_db', 'tail_attack_db']}}
                 for name, es in groups.items()}
        # Remote from either edge of a detected event: possible bleed, never an isolation label.
        remote = [f for f in m['frames'] if all(abs(f['seconds']-e['seconds']) > .3 for e in events)]
        stats['remote_from_events'] = {'frames': len(remote), 'raw_rms': median(f['rms'][raw] for f in remote),
          'eq_change': median(f['rms'][raw+1]-f['rms'][raw] for f in remote if f['rms'][raw] > -90),
          'post_change': median(f['rms'][raw+2]-f['rms'][raw] for f in remote if f['rms'][raw] > -90)}
        stats['quiet_coincident_other_drum_events'] = sum(any(abs(e['seconds']-other['seconds']) <= .03 for other in d['events'][1-drum]) for e in groups['quiet'])
        stats['max_gr'] = max((f['max_gr'][drum] for f in m['frames']), default=0)
        stats['event_stage_rms'] = [median(m['frames'][e['frame']]['rms'][raw+k] for e in events) for k in range(5)]
        result['drums'].append(stats)
    for section in sorted({int(f['seconds']//12) for f in m['frames']}):
        fs = [f for f in m['frames'] if int(f['seconds']//12)==section]
        ss = [s for s in m['spectra'] if int(s['seconds']//12)==section]
        result['sections'].append({'start': section*12,'held_out': bool(section%2),
            'stage_rms_median': [median(f['rms'][i] for f in fs) for i in range(15)],
            'stage_bands_median': [[median(s['bands'][i][b] for s in ss) for b in range(9)] for i in range(15)],
            'max_mix_peak': max(f['peak'][14] for f in fs)})
    mask = json.loads((root / 'baseline.json').read_text()) if (root / 'baseline.json').exists() else m
    for note in sorted({s['bass_note']['midi'] for s in mask['spectra'] if s['bass_note']}):
        ss = [s for s, ref in zip(m['spectra'], mask['spectra']) if ref['bass_note'] and ref['bass_note']['midi']==note
              and band_sum(ref['bands'][4], [1,2]) > -45]
        result['bass_pitch_overlap'][note] = {'windows': len(ss),
          'bass_minus_kick_40_100': median(band_sum(s['bands'][10],[1,2])-band_sum(s['bands'][4],[1,2]) for s in ss),
          'bass_minus_kick_100_200': median(s['bands'][10][3]-s['bands'][4][3] for s in ss)}
    return result


if __name__ == '__main__':
    print(json.dumps(summarize(sys.argv[1], "--baseline" in sys.argv[2:]), indent=2))
