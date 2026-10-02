#!/usr/bin/env python3
"""Compare production measurements on fixed raw-source windows, without choosing EQ.

Usage: python3 scripts/ensemble_review.py SPEC.json NEW_REPORT.json
SPEC supplies rate, source {settings, analysis}, variants [{id, settings, analysis,
export_gain_db}], and relationships [{name, numerator, denominator, band}].
Analysis paths name directories containing balance windows.csv and groups.csv.
Bands are the documented overlapping balance filters, not perceptual loudness.
"""
import argparse
import csv
import json
import math
from pathlib import Path


def quantile(values, p):
    x = sorted(values)
    return x[round((len(x) - 1) * p)] if x else None


def stats(values):
    return dict(count=len(values), p10=quantile(values, .1),
                median=quantile(values, .5), p90=quantile(values, .9))


def read_analysis(path):
    path = Path(path)
    with (path / 'groups.csv').open() as stream:
        groups = {}
        for row in csv.DictReader(stream):
            key = (int(row['start_frame']), row['group'])
            if key in groups:
                raise ValueError('duplicate group window')
            groups[key] = {k: float(v) for k, v in row.items() if k not in ('start_frame', 'group')}
    with (path / 'windows.csv').open() as stream:
        channels = {}
        for row in csv.DictReader(stream):
            key = (int(row['start_frame']), int(row['channel']))
            if key in channels:
                raise ValueError('duplicate channel window')
            channels[key] = {k: float(row[k]) for k in ('frames', 'input_dbfs', 'pre_compressor_dbfs',
                'post_compressor_dbfs', 'peak_dbfs', 'mean_reduction_db', 'max_reduction_db')}
    if not groups or not channels or any(not math.isfinite(v) for d in [groups, channels] for r in d.values() for v in r.values()):
        raise ValueError('empty or nonfinite evidence')
    return groups, channels


def review(spec):
    rate = spec['rate']
    if not isinstance(rate, int) or rate <= 0:
        raise ValueError('invalid rate')
    source = spec['source']
    scope = json.loads((Path(source['analysis']).parent / 'scope.json').read_text())
    if scope.get('mode') != 'source_bypass':
        raise ValueError('activity reference must be explicit SOURCE bypass evidence')
    settings = json.loads(Path(source['settings']).read_text())
    raw, inputs = read_analysis(source['analysis'])
    frames = sorted({f for f, _ in inputs})
    # Windows crossing section boundaries never vote in either split.
    section = lambda f: f // (12 * rate)
    contained = [f for f in frames if section(f) == section(f + int(inputs[f, 0]['frames']) - 1)]
    fit = [f for f in contained if section(f) % 2 == 0]
    names = {g for _, g in raw}
    floors = {g: max(-65., (quantile([raw[f, g]['broadband_dbfs'] for f in fit], .95) or -240.) - 30.) for g in names}
    routing = lambda s: [(c['file'], c['pan']) for c in s['channels']]
    result = {'scope': 'routed post-channel/pre-master powers; fixed SOURCE activity, training-only thresholds; export gain is separately added to absolute levels',
              'limitations': 'Energy ratios are not masking, source identity or preference. Raw activity can include bleed. Within-song splits are not blind generalization.',
              'activity_floor_dbfs': floors, 'listener_preferred': None, 'variants': []}
    ids = [v['id'] for v in spec['variants']]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate variant')
    for v in spec['variants']:
        s = json.loads(Path(v['settings']).read_text())
        groups, channels = read_analysis(v['analysis'])
        if routing(s) != routing(settings) or s['sample_rate'] != rate or settings['sample_rate'] != rate:
            raise ValueError('routing/pan/rate differs')
        if groups.keys() != raw.keys() or channels.keys() != inputs.keys() or any(
            abs(channels[k]['input_dbfs'] - inputs[k]['input_dbfs']) > .0001
            or channels[k]['frames'] != inputs[k]['frames'] for k in inputs):
            raise ValueError('timeline or raw inputs differ')
        export = v.get('export_gain_db')
        if export is not None and not math.isfinite(export):
            raise ValueError('invalid export gain')
        record = {'id': v['id'], 'export_gain_db': export, 'relationships': [], 'channels': []}
        for rel in spec['relationships']:
            n, d, band = rel['numerator'], rel['denominator'], rel['band']
            selected = [f for f in contained if raw[f, n]['broadband_dbfs'] > floors[n] and raw[f, d]['broadband_dbfs'] > floors[d]]
            levels = [raw[f, n]['broadband_dbfs'] for f in selected if section(f) % 2 == 0]
            quiet, strong = quantile(levels, .25), quantile(levels, .75)
            subsets = {'training': [f for f in selected if section(f) % 2 == 0],
                       'held_out': [f for f in selected if section(f) % 2 == 1]}
            subsets.update({f'section_{k:02}': [f for f in selected if section(f) == k] for k in sorted({section(f) for f in contained})})
            for split in ('training', 'held_out'):
                subsets[split + '_quiet'] = [f for f in subsets[split] if quiet is not None and raw[f, n]['broadband_dbfs'] <= quiet]
                subsets[split + '_strong'] = [f for f in subsets[split] if strong is not None and raw[f, n]['broadband_dbfs'] >= strong]
            r = {**rel, 'subsets': {}}
            for name, ff in subsets.items():
                before = [raw[f, n][band] - raw[f, d][band] for f in ff]
                after = [groups[f, n][band] - groups[f, d][band] for f in ff]
                r['subsets'][name] = {'sufficient': len(ff) >= 8, 'source_ratio_db': stats(before),
                    'ratio_db': stats(after), 'paired_ratio_change_db': stats([b-a for a,b in zip(before,after)]),
                    'numerator_pre_master_dbfs': stats([groups[f,n][band] for f in ff]),
                    'numerator_plus_export_dbfs': None if export is None else stats([groups[f,n][band]+export for f in ff]),
                    'coherent_minus_incoherent_db': stats([groups[f,n]['broadband_dbfs']-groups[f,n]['incoherent_broadband_dbfs'] for f in ff])}
            record['relationships'].append(r)
        for i, c in enumerate(s['channels']):
            floor = max(-65., quantile([inputs[f,i]['input_dbfs'] for f in fit], .95)-30.)
            ff = [f for f in contained if inputs[f,i]['input_dbfs'] > floor]
            record['channels'].append({'file': c['file'], 'fader_db': c['fader_db'], 'makeup_db': c['compressor']['makeup_db'],
                'raw_active_windows': len(ff), 'eq_hpf_level_change_db': stats([channels[f,i]['pre_compressor_dbfs']-inputs[f,i]['input_dbfs'] for f in ff]),
                'compression_plus_makeup_db': stats([channels[f,i]['post_compressor_dbfs']-channels[f,i]['pre_compressor_dbfs'] for f in ff]),
                'mean_reduction_db': stats([channels[f,i]['mean_reduction_db'] for f in ff]),
                'maximum_reduction_db': max((channels[f,i]['max_reduction_db'] for f in ff),default=0)})
        result['variants'].append(record)
    return result


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('spec', type=Path)
    p.add_argument('output', type=Path)
    a = p.parse_args()
    if a.output.exists():
        raise FileExistsError(a.output)
    result = review(json.loads(a.spec.read_text()))
    with a.output.open('x') as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
