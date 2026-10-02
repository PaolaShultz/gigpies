#!/usr/bin/env python3
"""Summarize/plot Rust bass evidence; never proposes or changes processing settings."""
import argparse
import json
from pathlib import Path
import numpy as np


def summarize(before, after, proposal):
    w = before['windows']
    mask = np.array(proposal['diagnosis']['eligible'])
    def med(values):
        return float(np.median(values)) if len(values) else None
    def dbsum(values):
        return 10 * np.log10(np.maximum(np.sum(10 ** (np.array(values) / 10), axis=-1), 1e-24))
    results = {}
    for label, choose in [('training', [not x['held_out'] for x in w]), ('held_out', [x['held_out'] for x in w])]:
        ids = np.flatnonzero(mask & choose)
        results[label] = {'windows': len(ids), 'notes': {}}
        for midi in sorted({w[i]['note']['midi'] for i in ids}):
            ii = [i for i in ids if w[i]['note']['midi'] == midi]
            row = {'count': len(ii)}
            for name, data in [('before', before), ('after', after)]:
                ww = [data['windows'][i] for i in ii]
                row[name] = {
                    'bass_rms_dbfs': med([x['stages'][4]['rms_dbfs'] for x in ww]),
                    'fundamental_dbfs': med([x['stages'][4]['fundamental_dbfs'] for x in ww if x['stages'][4]['fundamental_dbfs'] is not None]),
                    'definition_body_db': med([dbsum(x['stages'][4]['bands_dbfs'][5:7]) - dbsum(x['stages'][4]['bands_dbfs'][3:5]) for x in ww]),
                    'bass_minus_kick_40_100_db': med([dbsum(x['stages'][4]['bands_dbfs'][1:3])-dbsum(x['stages'][6]['bands_dbfs'][1:3]) for x in ww]),
                    'bass_minus_guitar_400_1600_db': med([dbsum(x['stages'][4]['bands_dbfs'][5:7])-dbsum(x['stages'][7]['bands_dbfs'][5:7]) for x in ww]),
                    'mean_gr_db': med([x['mean_reduction_db'] for x in ww]),
                }
            results[label]['notes'][midi] = row
        for name, data in [('before', before), ('after', after)]:
            results[label][name] = {'median_stage_bands_dbfs': {stage: np.median([data['windows'][i]['stages'][j]['bands_dbfs'] for i in ids], axis=0).tolist() for j, stage in enumerate(data['stages'])}}
    e = before['envelopes']; raw = np.array([x['raw_dbfs'] for x in e]); gr = np.array([x['mean_reduction_db'] for x in e])
    active = raw > proposal['diagnosis']['activity_floor_dbfs']
    smooth = 10*np.log10(np.maximum(np.convolve(10**(raw/10), np.ones(4)/4, mode='full')[:len(raw)], 1e-24))
    rising = np.flatnonzero((smooth-np.concatenate([smooth[:4], smooth[:-4]]) > 6) & active)
    attacks = []
    for i in rising:
        if not attacks or i-attacks[-1] > 10:
            attacks.append(i)
    isolated = [i for j, i in enumerate(attacks) if i+30 < len(e) and (j+1 == len(attacks) or attacks[j+1]-i > 30)]
    results['envelopes'] = {'detected_attack_count': len(attacks), 'isolated_300ms_event_count': len(isolated), 'attack_definition': '>6 dB rise of causal 40 ms raw power relative to 40 ms earlier, active input, 100 ms refractory; an energy event, not verified articulation', 'attack_gr_first_30ms_median_db': med([float(np.mean(gr[i:i+3])) for i in attacks]), 'isolated_attack_gr_100_300ms_median_db': med([float(np.mean(gr[i+10:i+30])) for i in isolated]), 'active_mean_gr_median_db': med(gr[active]), 'max_gr_db': max(x['max_reduction_db'] for x in e)}
    for label, eligible in [('quiet', active & (raw <= np.quantile(raw[active], .25))), ('strong', active & (raw >= np.quantile(raw[active], .75))), ('kick_during_bass_pause', (~active) & (np.array([x['kick_raw_dbfs'] for x in e]) > -35))]:
        results['envelopes'][label] = {'seconds': float(np.sum(eligible)*.01), 'raw_rms_median_dbfs': med(raw[eligible]), 'mean_gr_median_db': med(gr[eligible])}
    return results


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('review', type=Path)
    p.add_argument('bass_source', type=Path)
    a = p.parse_args()
    before, after, proposal = [json.loads((a.review / n).read_text()) for n in ['baseline.json', 'candidate.json', 'proposal.json']]
    report = summarize(before, after, proposal)
    (a.review/'summary.json').write_text(json.dumps(report, indent=2))
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from scipy.io import wavfile
    from scipy import signal
    rate, x = wavfile.read(a.bass_source)
    if x.dtype.kind == 'i':
        x = x.astype(float) / 2 ** (x.dtype.itemsize*8-1)
    if x.ndim > 1:
        x = x[:, 0]
    freq, time, power = signal.spectrogram(x, rate, nperseg=16384, noverlap=8192)
    band = (freq >= 30) & (freq <= 2000)
    fig, ax = plt.subplots(3, 1, figsize=(13, 10), constrained_layout=True)
    plot = ax[0].pcolormesh(time, freq[band], 10*np.log10(np.maximum(power[band], 1e-20)), vmin=-100, vmax=-35, shading='auto')
    ax[0].set(yscale='log', ylim=(30,2000), ylabel='DI frequency (Hz)', title='Raw DI: musical energy, not automatic notch targets')
    fig.colorbar(plot, ax=ax[0], label='dBFS/Hz')
    e = before['envelopes']
    ax[1].plot([x['seconds'] for x in e], [x['mean_reduction_db'] for x in e], lw=.6)
    ax[1].set(ylabel='Bass compressor GR (dB)', title='Actual 10 ms mean gain reduction, excluding makeup')
    for name, data in [('Current', before), ('Corrected', after)]:
        w = data['windows']
        b = np.array([x['stages'][4]['bands_dbfs'] for x in w])
        ratio = 10*np.log10(np.maximum(np.sum(10**(b[:,5:7]/10),axis=1),1e-24))-10*np.log10(np.maximum(np.sum(10**(b[:,3:5]/10),axis=1),1e-24))
        ratio[np.array([x['stages'][0]['rms_dbfs'] <= -65 for x in w])] = np.nan
        ax[2].plot([x['seconds'] for x in w], ratio, label=name, lw=1)
    ax[2].set(ylabel='400–1600 / 100–400 Hz (dB)', xlabel='Seconds', ylim=(-60,0), title='Bass after routing/master HPF; low-confidence windows shown, not used to vote')
    ax[2].legend()
    fig.savefig(a.review/'evidence.png', dpi=140)


if __name__ == '__main__':
    main()
