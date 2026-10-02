use super::*;
use std::io::Write;
pub fn write(path: &Path, state: &State) -> Result<()> {
    let hz = (0..161)
        .map(|i| 31.25 * 512_f64.powf(i as f64 / 160.))
        .filter(|f| *f < state.baseline.sample_rate as f64 * 0.45)
        .collect::<Vec<_>>();
    let curves = state
        .choices
        .iter()
        .map(|c| {
            (0..=100)
                .map(|a| {
                    hz.iter()
                        .map(|&hz| {
                            response(&c.proposal.bands, state.baseline.sample_rate, hz, a as f64)
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let baseline=state.request.group.inputs.iter().map(|i|{
        let c=&state.baseline.channels[i.channel];
        serde_json::json!({"file":c.file,"db":hz.iter().map(|&hz|response(&c.eq,state.baseline.sample_rate,hz,100.)+10.*Biquad::highpass(c.hpf_hz,std::f64::consts::FRAC_1_SQRT_2,state.baseline.sample_rate).power_response(hz,state.baseline.sample_rate).max(1e-24).log10()).collect::<Vec<_>>()})
    }).collect::<Vec<_>>();
    let data = serde_json::to_string(
        &serde_json::json!({"state":state,"hz":hz,"curves":curves,"baseline":baseline}),
    )?
    .replace('&', "\\u0026")
    .replace('<', "\\u003c")
    .replace('>', "\\u003e");
    let html = include_str!("review.html").replace("__MATCH_DATA__", &data);
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    f.write_all(html.as_bytes())?;
    Ok(())
}
