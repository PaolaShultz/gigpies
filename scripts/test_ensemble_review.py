import csv
import json
from pathlib import Path
import tempfile
import unittest
from ensemble_review import review


class EnsembleTests(unittest.TestCase):
    def fixture(self, root):
        s = dict(sample_rate=100, channels=[dict(file='a.wav', pan=0, fader_db=0, compressor={'makeup_db': 0}),
                                          dict(file='b.wav', pan=0, fader_db=0, compressor={'makeup_db': 0})])
        (root/'scope.json').write_text(json.dumps({'mode':'source_bypass'}))
        settings = root/'settings.json'
        settings.write_text(json.dumps(s))
        bands = ['broadband_dbfs','40_120_dbfs','120_500_dbfs','500_1500_dbfs','1500_5000_dbfs','incoherent_broadband_dbfs']
        fields = ['frames','input_dbfs','pre_compressor_dbfs','post_compressor_dbfs','peak_dbfs','mean_reduction_db','max_reduction_db']
        for variant in ['source','candidate']:
            folder = root/variant
            folder.mkdir()
            with (folder/'groups.csv').open('w') as f, (folder/'windows.csv').open('w') as c:
                g = csv.writer(f); w = csv.writer(c)
                g.writerow(['start_frame','group']+bands)
                w.writerow(['start_frame','channel']+fields)
                for section in range(4):
                    for j in range(10):
                        frame = section*1200+j*100
                        for i, name in enumerate(['voice','guitar']):
                            raw = -40 if j < 5 else -20
                            delta = (-6 if section==1 else 2) if variant=='candidate' and i==0 else 0
                            g.writerow([frame,name]+[raw+delta]*len(bands))
                            w.writerow([frame,i,100,raw,raw,raw+delta,raw+delta+8,0,0])
        return dict(rate=100,source=dict(settings=str(settings),analysis=str(root/'source')),
                    variants=[dict(id='candidate',settings=str(settings),analysis=str(root/'candidate'),export_gain_db=-3)],
                    relationships=[dict(name='voice/guitar',numerator='voice',denominator='guitar',band='broadband_dbfs')])

    def test_section_conflict_quiet_playing_and_export_stage(self):
        with tempfile.TemporaryDirectory() as d:
            spec=self.fixture(Path(d)); report=review(spec)
            r=report['variants'][0]['relationships'][0]['subsets']
            self.assertEqual(r['training']['ratio_db']['median'],2)
            self.assertEqual(r['section_01']['paired_ratio_change_db']['median'],-6)
            self.assertEqual(r['held_out_quiet']['ratio_db']['count'],10)
            self.assertEqual(r['training']['numerator_plus_export_dbfs']['median'],
                             r['training']['numerator_pre_master_dbfs']['median']-3)
            self.assertIsNone(report['listener_preferred'])

    def test_candidate_cannot_reselect_activity_or_change_training_threshold(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);spec=self.fixture(root);before=review(spec)
            p=root/'candidate/groups.csv';text=p.read_text().replace('-38','-138');p.write_text(text)
            after=review(spec)
            self.assertEqual(before['activity_floor_dbfs'],after['activity_floor_dbfs'])
            a=before['variants'][0]['relationships'][0]['subsets']['training']['ratio_db']['count']
            b=after['variants'][0]['relationships'][0]['subsets']['training']['ratio_db']['count']
            self.assertEqual(a,b)

    def test_changed_timeline_raw_input_and_routing_fail(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);spec=self.fixture(root)
            p=root/'candidate/windows.csv';p.write_text(p.read_text().replace('1200,0,100','1201,0,100'))
            with self.assertRaises(ValueError):review(spec)
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);spec=self.fixture(root)
            p=root/'candidate/windows.csv';p.write_text(p.read_text().replace('100,-40','100,-41'))
            with self.assertRaises(ValueError):review(spec)
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);spec=self.fixture(root)
            p=root/'other.json';s=json.loads((root/'settings.json').read_text());s['channels'][0]['pan']=.3;p.write_text(json.dumps(s))
            spec['variants'][0]['settings']=str(p)
            with self.assertRaises(ValueError):review(spec)

    def test_processed_reference_cannot_masquerade_as_source(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);spec=self.fixture(root)
            (root/'scope.json').write_text(json.dumps({'mode':'processed'}))
            with self.assertRaises(ValueError):review(spec)
