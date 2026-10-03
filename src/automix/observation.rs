//! Descriptive taps in the production render loop. No settings or taste decisions.
use super::{
    config::{Role, Session},
    dsp::{Biquad, Meter, db, gain},
    render::write_json,
};
use crate::inventory::Result;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

pub(super) struct Observer {
    rate: u32,
    source: Vec<Meter>,
    eq: Vec<Meter>,
    compressed: Vec<Meter>,
    routed: Vec<Meter>,
    stages: [Meter; 4],
    stage_windows: [Meter; 4],
    band_filters: Vec<Vec<[Biquad; 2]>>,
    band_energy: Vec<[f64; 4]>,
    contributions: Vec<[f64; 2]>,
    filters: Vec<Vec<[Biquad; 2]>>,
    drive: f64,
    group_ids: Vec<usize>,
    groups: Vec<String>,
    group_energy: Vec<f64>,
    individual_energy: Vec<f64>,
    block_reduction: Vec<[f64; 2]>,
    block_frames: u64,
    windows: BufWriter<File>,
    peak: f64,
    peak_record: serde_json::Value,
    frames: u64,
    mono_energy: f64,
    stereo_energy: f64,
    neutral_terms: Vec<[f64; 2]>,
    neutral_pan_gain: Vec<(f64, f64)>,
    neutral_master: f64,
    neutral_max_residual: f64,
    neutral_max_bound_fraction: f64,
}
impl Observer {
    pub fn new(s: &Session, out: &Path) -> Result<Self> {
        let n = s.channels.len();
        let returns = s.effects.as_ref().map_or(0, |f| f.buses.len());
        let mut filters = Vec::new();
        if s.master_hpf_hz > 0. {
            filters.push(
                [Biquad::highpass(
                    s.master_hpf_hz,
                    std::f64::consts::FRAC_1_SQRT_2,
                    s.sample_rate,
                ); 2],
            );
        }
        if let Some(fx) = &s.effects {
            filters.extend(
                fx.master_eq
                    .iter()
                    .map(|e| [Biquad::equalizer(e, s.sample_rate); 2]),
            );
        }
        let groups = vec!["drums", "bass", "guitars", "vocals", "other", "returns"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let group_ids = s
            .channels
            .iter()
            .map(|c| match c.role {
                Role::Kick | Role::Snare | Role::Tom | Role::Overheads | Role::DrumRoom => 0,
                Role::BassDi | Role::BassAmp => 1,
                Role::RhythmGuitar | Role::LeadGuitar | Role::AcousticGuitar => 2,
                Role::LeadVocal | Role::BackingVocal | Role::VocalRoom => 3,
                _ => 4,
            })
            .chain(std::iter::repeat_n(5, returns))
            .collect();
        let mut windows = BufWriter::new(File::create(out.join("stage-windows.csv"))?);
        writeln!(
            windows,
            "end_frame,kind,index,mean_reduction_db,max_reduction_db,coherent_power,incoherent_power,signed_cross_power"
        )?;
        Ok(Self {
            rate: s.sample_rate,
            source: vec![Meter::default(); n],
            eq: vec![Meter::default(); n],
            compressed: vec![Meter::default(); n],
            routed: vec![Meter::default(); n],
            stages: std::array::from_fn(|_| Meter::default()),
            stage_windows: std::array::from_fn(|_| Meter::default()),
            band_filters: vec![
                [40_f64, 120., 500., 1500., 5000.]
                    .iter()
                    .map(|hz| [Biquad::highpass(
                        hz.min(s.sample_rate as f64 * 0.45),
                        std::f64::consts::FRAC_1_SQRT_2,
                        s.sample_rate
                    ); 2])
                    .collect();
                groups.len()
            ],
            band_energy: vec![[0.; 4]; groups.len()],
            contributions: vec![[0.; 2]; n + returns],
            filters: vec![filters; n + returns],
            drive: s
                .effects
                .as_ref()
                .map_or(1., |f| gain(f.maximizer_drive_db)),
            group_ids,
            group_energy: vec![0.; groups.len()],
            individual_energy: vec![0.; groups.len()],
            groups,
            block_reduction: vec![[0.; 2]; n + 2],
            block_frames: 0,
            windows,
            peak: 0.,
            peak_record: serde_json::Value::Null,
            frames: 0,
            mono_energy: 0.,
            stereo_energy: 0.,
            neutral_terms: vec![[0.; 2]; n],
            neutral_pan_gain: s
                .channels
                .iter()
                .map(|c| {
                    (
                        c.pan,
                        10_f64.powf(c.fader_db / 20.)
                            * 10_f64.powf(s.calibration.initial_trim_db / 20.),
                    )
                })
                .collect(),
            neutral_master: 10_f64.powf(s.master_db / 20.),
            neutral_max_residual: 0.,
            neutral_max_bound_fraction: 0.,
        })
    }
    pub fn channel(&mut self, i: usize, taps: [[f64; 2]; 4], channels: usize, reduction: f64) {
        let [source, eq, compressed, routed] = taps;
        // Independent documented equations; neither route() nor its output is used.
        let (pan, level) = self.neutral_pan_gain[i];
        let weights = if channels == 1 {
            let angle = std::f64::consts::PI * (pan + 1.) / 4.;
            [angle.cos(), angle.sin()]
        } else {
            [(1. - pan.max(0.)).sqrt(), (1. + pan.min(0.)).sqrt()]
        };
        self.neutral_terms[i] =
            std::array::from_fn(|c| source[c] * weights[c] * level * self.neutral_master);
        for c in 0..channels {
            self.source[i].add(source[c]);
            self.eq[i].add(eq[c]);
            self.compressed[i].add(compressed[c]);
        }
        for v in routed {
            self.routed[i].add(v);
        }
        self.contributions[i] = routed;
        self.block_reduction[i][0] += reduction;
        self.block_reduction[i][1] = self.block_reduction[i][1].max(reduction);
    }
    pub fn neutral(&mut self, production: [f64; 2]) -> Result<()> {
        for (c, actual) in production.into_iter().enumerate() {
            let (mut sum, mut correction, mut absolute) = (0_f64, 0_f64, 0_f64);
            for x in self.neutral_terms.iter().map(|x| x[c]) {
                let t = sum + x;
                correction += if sum.abs() >= x.abs() {
                    (sum - t) + x
                } else {
                    (x - t) + sum
                };
                sum = t;
                absolute += x.abs();
            }
            let error = (actual - (sum + correction)).abs();
            let k = (8 * self.neutral_terms.len() + 16) as f64;
            let bound = k * f64::EPSILON / (1. - k * f64::EPSILON) * absolute;
            if error > bound {
                return Err("independent neutral sum exceeded forward-error bound".into());
            }
            self.neutral_max_residual = self.neutral_max_residual.max(error);
            if bound > 0. {
                self.neutral_max_bound_fraction =
                    self.neutral_max_bound_fraction.max(error / bound);
            }
        }
        Ok(())
    }
    pub fn frame(
        &mut self,
        t: u64,
        returns: &[[f64; 2]],
        master: f64,
        stages: [[f64; 2]; 4],
        fx_reduction: f64,
        final_reduction: f64,
    ) -> Result<()> {
        let n = self.source.len();
        for (i, x) in returns.iter().enumerate() {
            self.contributions[n + i] = x.map(|v| v * master);
        }
        let mut grouped = vec![[0.; 2]; self.groups.len()];
        for (i, x) in self.contributions.iter().enumerate() {
            let g = self.group_ids[i];
            for c in 0..2 {
                grouped[g][c] += x[c];
                self.individual_energy[g] += x[c] * x[c];
            }
        }
        for (g, x) in grouped.iter().enumerate() {
            self.group_energy[g] += x[0] * x[0] + x[1] * x[1];
            let mut filtered = [[0.; 2]; 5];
            for (i, f) in self.band_filters[g].iter_mut().enumerate() {
                for c in 0..2 {
                    filtered[i][c] = f[c].tick(x[c]);
                }
            }
            for b in 0..4 {
                for (low, high) in filtered[b].iter().zip(filtered[b + 1]) {
                    self.band_energy[g][b] += (low - high).powi(2);
                }
            }
        }
        let mut filtered = self.contributions.clone();
        for (i, x) in filtered.iter_mut().enumerate() {
            for f in &mut self.filters[i] {
                for c in 0..2 {
                    x[c] = f[c].tick(x[c]);
                }
            }
            for v in x {
                *v *= self.drive;
            }
        }
        let reduction_gain = gain(-fx_reduction - final_reduction);
        let output = stages[3];
        for (i, x) in stages.into_iter().enumerate() {
            for v in x {
                self.stages[i].add(v);
                self.stage_windows[i].add(v);
            }
        }
        for (i, r) in [fx_reduction, final_reduction].into_iter().enumerate() {
            self.block_reduction[n + i][0] += r;
            self.block_reduction[n + i][1] = self.block_reduction[n + i][1].max(r);
        }
        for (c, v) in output.into_iter().enumerate() {
            if v.abs() > self.peak {
                self.peak = v.abs();
                let signed: Vec<_> = filtered.iter().map(|x| x[c] * reduction_gain).collect();
                let reconstructed: f64 = signed.iter().sum();
                self.peak_record = serde_json::json!({"frame":t,"seconds":t as f64/self.rate as f64,"channel":c,"output_sample":v,"source_then_return_contributions":signed,"reconstructed_sample":reconstructed,"residual":v-reconstructed,"stage":"after common master linear filters and observed linked gains, before float32/export","fx_reduction_db":fx_reduction,"final_reduction_db":final_reduction});
            }
        }
        self.mono_energy += ((output[0] + output[1]) * 0.5).powi(2);
        self.stereo_energy += (output[0] * output[0] + output[1] * output[1]) * 0.5;
        self.frames += 1;
        self.block_frames += 1;
        if self.block_frames >= u64::from(self.rate / 10) {
            self.flush(t + 1)?;
        }
        Ok(())
    }
    fn flush(&mut self, end: u64) -> Result<()> {
        if self.block_frames == 0 {
            return Ok(());
        }
        for (i, r) in self.block_reduction.iter_mut().enumerate() {
            writeln!(
                self.windows,
                "{end},reduction,{i},{:.9},{:.9},,,",
                r[0] / self.block_frames as f64,
                r[1]
            )?;
            *r = [0.; 2];
        }
        for i in 0..self.groups.len() {
            let scale = 2. * self.block_frames as f64;
            let a = self.group_energy[i] / scale;
            let b = self.individual_energy[i] / scale;
            writeln!(
                self.windows,
                "{end},group,{i},,,{a:.12e},{b:.12e},{:.12e}",
                a - b
            )?;
            self.group_energy[i] = 0.;
            self.individual_energy[i] = 0.;
            for (b, name) in ["40_120", "120_500", "500_1500", "1500_5000"]
                .iter()
                .enumerate()
            {
                writeln!(
                    self.windows,
                    "{end},band_{name},{i},,,{:.12e},,",
                    self.band_energy[i][b] / scale
                )?;
            }
            self.band_energy[i] = [0.; 4];
        }
        for (i, m) in self.stage_windows.iter_mut().enumerate() {
            writeln!(
                self.windows,
                "{end},stage_rms_peak,{i},{:.9},{:.9},,,",
                db(m.rms()),
                db(m.peak)
            )?;
            *m = Meter::default();
        }
        self.block_frames = 0;
        Ok(())
    }
    pub fn finish(mut self, out: &Path, s: &Session) -> Result<()> {
        self.flush(self.frames)?;
        self.windows.flush()?;
        write_json(
            &out.join("neutral-reference.json"),
            &serde_json::json!({"method":"independent documented routing + Neumaier compensated sum; shared decoded samples","frames":self.frames,"maximum_f64_absolute_residual":self.neutral_max_residual,"maximum_fraction_of_declared_bound":self.neutral_max_bound_fraction,"bound":"gamma(8N+16) * sum(abs(terms)), EPS=f64::EPSILON","settings_sha256":super::identity::identity(s)?}),
        )?;
        write_json(
            &out.join("stages.json"),
            &serde_json::json!({"settings_sha256":super::identity::identity(s)?,"sample_rate":self.rate,"frames":self.frames,"channels":(0..self.source.len()).map(|i|serde_json::json!({"file":s.channels[i].file,"source":self.source[i].report(),"post_filter":self.eq[i].report(),"post_compressor_before_makeup":self.compressed[i].report(),"post_makeup_exciter_fader_pan_master_gain":self.routed[i].report()})).collect::<Vec<_>>(),"stage_order":["direct_ensemble","combined_with_returns","after_master_hpf","after_fx_master_and_final_limiter"],"stages":self.stages.iter().map(Meter::report).collect::<Vec<_>>(),"peak_contributions":self.peak_record,"groups":self.groups,"group_definition":"role-based coherent routed sums; grouping does not infer capture identity","reduction_indices":"channel order then FX master then final limiter; 100ms windows include silence/recovery","mono_fold":"(L+R)/2, compared with mean L/R power","mono_minus_stereo_db":db((self.mono_energy/self.stereo_energy.max(1e-30)).sqrt()),"true_peak_stage":"meter retained float and final PCM separately; these taps report sample peaks"}),
        )
    }
}
