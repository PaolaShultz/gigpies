#!/usr/bin/env python3
"""Describe close-snare contribution on fixed source events; never select a mix.

Usage: python3 scripts/snare_context.py SPEC.json training|held_out NEW_REPORT.json
SPEC supplies rate, source_analysis, diagnosis and variants [{id, analysis}].
All analysis folders contain production balance CSVs. The diagnosis is frozen
baseline drum evidence. Microphone groups contain mixtures, not isolated sources.
"""
import argparse
import json
import math
from pathlib import Path

from ensemble_review import read_analysis, quantile, stats


def power(db):
    return 10 ** (db / 10)


def db(power_value):
    return 10 * math.log10(max(1e-24, power_value))


def contribution(snare, remainder, coherent):
    """Signed summation evidence, with no ratio claim at numerical silence."""
    return {
        'close_over_remainder_db': db(snare) - db(remainder) if min(snare, remainder) > 1e-20 else None,
        'coherent_minus_independent_db': db(coherent) - db(snare + remainder)
            if min(coherent, snare + remainder) > 1e-20 else None,
        'omission_level_change_db': db(remainder) - db(coherent)
            if min(remainder, coherent) > 1e-20 else None,
        'signed_cross_power': coherent - snare - remainder,
    }


def report(spec, split):
    if split not in ('training', 'held_out'):
        raise ValueError('choose training or held_out explicitly')
    rate = spec['rate']
    if not isinstance(rate, int) or rate <= 0:
        raise ValueError('invalid sample rate')
    scope = json.loads((Path(spec['source_analysis']).parent / 'scope.json').read_text())
    if scope.get('mode') != 'source_bypass':
        raise ValueError('context requires explicit SOURCE bypass evidence')
    source, inputs = read_analysis(spec['source_analysis'])
    events = json.loads(Path(spec['diagnosis']).read_text())['events'][1]
    section = lambda f: f // (12 * rate)
    frames = sorted(f for f, i in inputs if i == 0)
    contained = lambda f: section(f) == section(f + int(inputs[f, 0]['frames']) - 1)
    fit = [f for f in frames if section(f) % 2 == 0 and contained(f)]
    training_events = [e for e in events if not e['held_out']]
    levels = [e['raw_attack_db'] for e in training_events]
    quiet, strong = quantile(levels, .25), quantile(levels, .75)
    voice_floor = max(-65., (quantile([source[f, 'lead_vocal']['broadband_dbfs'] for f in fit], .95)
                            if fit else -240.) - 30.)
    selected = []
    for event in events:
        if event['held_out'] != (split == 'held_out'):
            continue
        start = round(event['seconds'] * rate)
        end = start + round(.08 * rate)
        if section(start) != section(end - 1):
            continue
        ff = [f for f in frames if start <= f and f + inputs[f, 0]['frames'] <= end and contained(f)]
        if not ff:
            continue
        # A local raw-vocal activity proxy. It is neither lyric transcription
        # nor a clean-vocal/bleed label; retain that uncertainty in the report.
        vocal = sum(power(source[f, 'lead_vocal']['broadband_dbfs']) for f in ff) / len(ff)
        selected.append((event, ff, db(vocal) > voice_floor))
    output = {'split': split, 'quiet_raw_attack_dbfs': quiet, 'strong_raw_attack_dbfs': strong,
              'raw_vocal_activity_floor_dbfs': voice_floor, 'listener_preference': None,
              'scope': '0-80 ms baseline events, complete 20 ms production windows; post-channel/fader, pre-master/export',
              'limitations': 'Events can contain bleed/unison playing. Vocal activity and rapid/compound events are context proxies, not singing/fill or source-separation labels. Signed cross-power includes coherent interaction. Omission is diagnostic, never applied audio.',
              'variants': []}
    for variant in spec['variants']:
        groups, channels = read_analysis(variant['analysis'])
        if groups.keys() != source.keys() or channels.keys() != inputs.keys() or any(
                abs(channels[k]['input_dbfs'] - v['input_dbfs']) > .0001
                or channels[k]['frames'] != v['frames'] for k, v in inputs.items()):
            raise ValueError('variant timeline or raw source differs')
        rows = []
        for event, ff, vocal in selected:
            weights = [inputs[f, 0]['frames'] for f in ff]
            energy = lambda group, band: sum(power(groups[f, group][band]) * n
                for f, n in zip(ff, weights)) / sum(weights)
            row = {'seconds': event['seconds'], 'section': int(event['seconds'] // 12),
                   'quiet': quiet is not None and event['raw_attack_db'] <= quiet,
                   'strong': strong is not None and event['raw_attack_db'] >= strong,
                   'vocal_active': vocal, 'compound': event['ambiguous_onset'],
                   'rapid': event['next_seconds'] is not None and event['next_seconds'] < .24,
                   'metrics': {}}
            for band in ('broadband_dbfs', '120_500_dbfs', '1500_5000_dbfs'):
                close = energy('snare', band)
                for context, rest, whole in [('kit', 'drums_without_snare', 'drums'),
                                             ('ensemble', 'ensemble_without_snare', 'ensemble')]:
                    for key, value in contribution(close, energy(rest, band), energy(whole, band)).items():
                        row['metrics'][f'{band}/{context}/{key}'] = value
                for group in ('snare', 'overheads', 'drum_room', 'toms', 'drum_ambience', 'guitars', 'bass', 'vocal_sum'):
                    row['metrics'][f'{band}/{group}/level_dbfs'] = db(energy(group, band))
            rows.append(row)
        subsets = {'all': rows}
        for key in ('quiet', 'strong', 'vocal_active', 'compound', 'rapid'):
            subsets[key] = [r for r in rows if r[key]]
        subsets['vocal_inactive'] = [r for r in rows if not r['vocal_active']]
        subsets.update({f'section_{k:02}': [r for r in rows if r['section'] == k]
                        for k in sorted({r['section'] for r in rows})})
        summary = {name: {'events': len(rr), 'sufficient': len(rr) >= 8,
                         'metrics': {key: stats([r['metrics'][key] for r in rr if r['metrics'][key] is not None])
                                     for key in (rows[0]['metrics'] if rows else [])}}
                   for name, rr in subsets.items()}
        output['variants'].append({'id': variant['id'], 'subsets': summary, 'events': rows})
    return output


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('spec', type=Path)
    parser.add_argument('split', choices=['training', 'held_out'])
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = report(json.loads(args.spec.read_text()), args.split)
    with args.output.open('x') as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
