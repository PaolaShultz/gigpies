#!/usr/bin/env python3
"""Frozen temporal representation audit; no source labels, DSP, settings or playback.

fit REVIEW NEW_MODEL: only 24--36 s rows can train either predeclared representation.
evaluate REVIEW MODEL NEW_REPORT: frozen full-song audit, with explicit abstentions.
"""
import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path


REPRESENTATIONS = {'attack': (1, .03), 'temporal': (4, .24)}


def distance(a, b):
    return statistics.median(abs(x - y) for x, y in zip(a, b))


def vector(event, phases):
    bands = event['phase_bands']
    attack = bands[0][1]
    total = 10 * math.log10(sum(10 ** (x / 10) for x in attack))
    # One attack reference preserves decay relative to attack; no audio gain change.
    return [x - total for phase in bands[:phases] for x in phase[1]]


def support(event, events, radius):
    t = event['seconds']
    reasons = []
    if event['ambiguous_onset']:
        reasons.append('compound_onset')
    if int((t - radius) // 12) != int((t + radius - 1e-8) // 12):
        reasons.append('split_boundary')
    other = [r for e in events if abs(e['seconds'] - t) > 1e-8
             for r in e['rise_candidates_seconds']]
    if any(t - radius + 1e-8 < r < t for r in other):
        reasons.append('previous_rise_within_support')
    if any(t < r < t + radius - 1e-8 for r in other):
        reasons.append('next_rise_within_support')
    return reasons


def fit(diagnosis, events):
    level = diagnosis['direct_reference_level']
    result = {'schema': 1, 'training_seconds': [24, 36], 'models': {},
              'reference_level': level, 'processing_permission': False}
    for name, (phases, radius) in REPRESENTATIONS.items():
        rows = [e for e in diagnosis['events'] if level is not None
                and 24 <= e['seconds'] - radius
                and e['seconds'] + radius <= 36
                and not e['held_out'] and e['raw_attack_db'] >= level - 3
                and not support(e, events, radius)]
        model = {'phases': phases, 'support_seconds': radius,
                 'training_events': [e['seconds'] for e in rows],
                 'template': None, 'training_envelope_db': None,
                 'status': 'insufficient_isolated_training_events'}
        if len(rows) >= 8:
            xs = [vector(e, phases) for e in rows]
            model['template'] = [statistics.median(v) for v in zip(*xs)]
            # Leave-one-out avoids comparing a training row only to its own template.
            errors = sorted(distance(x, [statistics.median(v) for v in zip(
                *(xs[:i] + xs[i+1:]))]) for i, x in enumerate(xs))
            model['training_envelope_db'] = errors[math.ceil(.95 * len(errors)) - 1]
            model['status'] = 'diagnostic_only_no_source_labels'
        result['models'][name] = model
    return result


def evaluate(diagnosis, events, model):
    if model['schema'] != 1 or model['processing_permission'] is not False:
        raise ValueError('unsupported or processing-enabled model')
    level = model['reference_level']
    rows = []
    for e in diagnosis['events']:
        row = {'seconds': e['seconds'], 'held_out': e['held_out'],
               'quiet_kick_coincident_hypothesis': level is not None
               and e['raw_attack_db'] < level - 8 and e['kick_distance_seconds'] <= .04,
               'source_identity': 'unresolved_protected', 'representations': {}}
        for name, (phases, radius) in REPRESENTATIONS.items():
            m = model['models'][name]
            if m['phases'] != phases or m['support_seconds'] != radius:
                raise ValueError('representation differs from frozen protocol')
            reasons = support(e, events, radius)
            value = None if m['template'] is None else distance(vector(e, phases), m['template'])
            row['representations'][name] = {
                'support_failures': reasons, 'distance_db': value,
                'inside_training_envelope': None if value is None else value <= m['training_envelope_db']}
        rows.append(row)
    groups = []
    for held in [False, True]:
        for quiet in [False, True]:
            rs = [r for r in rows if r['held_out'] == held
                  and r['quiet_kick_coincident_hypothesis'] == quiet]
            groups.append({'held_out': held, 'quiet_kick_coincident_hypothesis': quiet,
                           'events': len(rs), 'representations': {
                name: {'supported_events': sum(not r['representations'][name]['support_failures'] for r in rs),
                       'inside_envelope': sum(r['representations'][name]['inside_training_envelope'] is True for r in rs)}
                for name in REPRESENTATIONS}})
    return {'listener_confirmed_bleed': diagnosis['listener_confirmed_bleed'],
            'decision': 'abstain_insufficient_source_identity',
            'processing_candidate': None, 'production_dsp_candidate_validation': None,
            'listener_acceptance': None, 'groups': groups, 'events': rows,
            'limitations': ['Outside the training envelope does not identify spill.',
                           'Inside the envelope does not prove wanted snare.',
                           'No detected neighboring rise does not establish isolation or silence.',
                           'All non-quiet groups include weak noncoincident and compound events.',
                           'Held-out sections are within-song diagnostics, not independent recordings.']}


def context(measurement, events, level):
    """Raw, fixed-threshold context counts; no interval is labelled spill-only."""
    if measurement['stages'][0] != 'kick_raw' or measurement['stages'][5] != 'snare_raw':
        raise ValueError('requires production drum measurement stages')
    rises = [t for e in events for t in e['rise_candidates_seconds']]
    result = []
    for section in sorted({int(f['seconds'] // 12) for f in measurement['frames']}):
        fs = [f for f in measurement['frames'] if int(f['seconds'] // 12) == section]
        residual = lambda f, hold: (level is not None and f['rms'][5] < level - 18
            and all(not t - .02 <= f['seconds'] <= t + hold for t in rises))
        result.append({'start_seconds': section * 12, 'held_out': bool(section % 2),
            'duration_seconds': sum(f['duration'] for f in fs),
            'snare_below_minus90_seconds': sum(f['duration'] for f in fs if f['rms'][5] < -90),
            'residual_120ms_seconds': sum(f['duration'] for f in fs if residual(f, .12)),
            'residual_240ms_seconds': sum(f['duration'] for f in fs if residual(f, .24)),
            'kick_bass_overlap_seconds': sum(f['duration'] for f in fs
                if f['rms'][0] > -45 and f['rms'][10] > -65),
            'snare_peak_reduction_db': max(f['max_gr'][1] for f in fs),
            'kick_peak_reduction_db': max(f['max_gr'][0] for f in fs)})
    return {'sections': result,
            'scope': '120/240 ms are coverage audits, not proposed gate holds; residual can contain wanted sustain or missed quiet hits.',
            'bass_scope': 'Existing bass contribution after its processing; thresholds describe coactivity, not source identity or masking.'}


def read_review(path):
    path = Path(path)
    diagnosis = json.loads((path / 'diagnosis.json').read_text())
    events = json.loads((path / 'events.json').read_text())['events'][1]
    # Missing, duplicated or nonfinite event data must not silently earn isolation.
    times = [e['seconds'] for e in events]
    if times != sorted(set(times)) or not all(math.isfinite(t) and 0 <= t <= 600 for t in times):
        raise ValueError('invalid event timeline')
    for e in events:
        rises = e['rise_candidates_seconds']
        if (not rises or rises != sorted(set(rises)) or e['seconds'] not in rises
                or not all(math.isfinite(t) and 0 <= t <= 600 for t in rises)):
            raise ValueError('missing retained rises')
    if any(e['seconds'] not in times for e in diagnosis['events']):
        raise ValueError('feature/event timeline mismatch')
    for e in diagnosis['events']:
        bands = e['phase_bands']
        if (len(bands) != 4 or any(len(p) != 5 or any(len(b) != 5 for b in p) for p in bands)
                or any(not math.isfinite(x) for p in bands for b in p for x in b)
                or not math.isfinite(e['raw_attack_db'])
                or not math.isfinite(e['kick_distance_seconds'])):
            raise ValueError('invalid feature shape or values')
    level = diagnosis['direct_reference_level']
    if level is not None and not math.isfinite(level):
        raise ValueError('invalid reference level')
    return diagnosis, events


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['fit', 'evaluate'])
    parser.add_argument('review', type=Path)
    parser.add_argument('paths', nargs='+', type=Path)
    parser.add_argument('--drum-measurement', type=Path)
    args = parser.parse_args()
    if len(args.paths) != (1 if args.mode == 'fit' else 2):
        parser.error('fit needs NEW_MODEL; evaluate needs MODEL NEW_REPORT')
    output = args.paths[-1]
    if output.exists():
        raise FileExistsError(output)
    d, events = read_review(args.review)
    if args.mode == 'fit':
        result = fit(d, events)
    else:
        result = evaluate(d, events, json.loads(args.paths[0].read_text()))
        if args.drum_measurement:
            result['context'] = context(json.loads(args.drum_measurement.read_text()), events,
                                        json.loads(args.paths[0].read_text())['reference_level'])
            result['drum_measurement_sha256'] = hashlib.sha256(args.drum_measurement.read_bytes()).hexdigest()
        result['frozen_model_sha256'] = hashlib.sha256(args.paths[0].read_bytes()).hexdigest()
    result['input_hashes'] = {name: hashlib.sha256((args.review / name).read_bytes()).hexdigest()
                              for name in ['diagnosis.json', 'events.json']}
    with output.open('x') as f:
        json.dump(result, f, indent=2, allow_nan=False)
        f.write('\n')


if __name__ == '__main__':
    main()
