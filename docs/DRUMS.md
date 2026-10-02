# Kick, snare and rhythmic emphasis

**Current contract:** [offline source preservation](SOURCE_PRESERVATION.md) permits
explicit channel/master HPF choices including bypass. Drum analysis now measures
the configured master filter. Correction guards and explicit fader intent are retained.

**Current intent:** the earlier +2 dB rhythmic preference is withdrawn. The
[independent reassessment](COMPLAINIACS_WORKFLOW_REVIEW.md) starts after its earlier
reversal and tests inherited makeup separately. The historical experiment below
remains evidence, not the current musical requirement.

`drum-verify` and `drum-correct` now keep musical intent separate from technical
processing acceptance: `current_emphasis_db: null` withholds automatic fader
movement but permits fixed-fader processing to pass its unchanged protections.
Verification still rejects any fader movement without an explicit supported delta.
`balance-intent.json` reports that abstention separately from `outcome.json`.

Implemented and offline-validated, 2026-10-02. The new experiment starts from the
latest bass-corrected configuration, whose listener acceptance is still unresolved.
It preserves the listener-accepted guitar correction and verified bass DI correction;
the bass amp remains excluded. No playback, hardware change, commit or push occurred.

## Follow-up: confirmed snare bleed

The listener subsequently confirmed bleed and requested deeper programmatic analysis.
[The snare-bleed investigation](SNARE_BLEED.md) found and fixed a weak-precursor event
segmentation failure, added conditional waveform/prediction checks and rejected three
static cleanup candidates. The quiet-event/sustain statistics below describe the
initial detector and must be read with that correction. The +2 dB rhythmic candidate
passes revalidation; no gate, expander or cancellation was added.

## Result

The selected change is **+2 dB at each kick/snare fader**, relative to the saved
processed baseline. Kick moves **+1.5 → +3.5 dB**; snare moves **+1.0 → +3.0 dB**.
Their makeup remains **+5.5 / +3.5 dB**. EQ and compression remain unchanged.
Static EQ relief candidates were measured at fixed faders and retained as evidence,
but neither established an improvement sufficient to select it. This is a measured
rhythmic-balance adjustment, not a claim that the existing drum tone is ideal.

The full-song peak rises **1.682 dB** before export. Independent sample-peak
finalization therefore lowers the new export gain by that amount:

| Full-song measurement | Baseline | Selected |
|---|---:|---:|
| Kick/snare level relative to unchanged channels | Reference | +2.000 dB |
| Peak before export | +6.345 dBFS | +8.027 dBFS |
| Export gain | −6.355 dB | −8.037 dB |
| Final PCM sample peak | −0.010 dBFS | −0.010 dBFS |
| Final PCM RMS | −19.143 dBFS | −20.301 dBFS |
| Final PCM integrated loudness, FFmpeg measurement | −15.8 LUFS | −17.0 LUFS |
| Frames, 44.1 kHz stereo | 5,038,080 | 5,038,080 |

Consequently the drums' absolute contributions in the final export rise only
**0.318 dB**, while bass, guitars, vocals and other unchanged contributions fall
**1.682 dB**. Their relative balance with each other and their processing remain
unchanged. The new full-mix peak is in the 91.15 s drum event near the ending;
the baseline maximum is at 77.92 s. The later hit now determines export gain for
the whole song. No limiter, extra makeup or master change counteracts this tradeoff.
Internal floating-point samples above unity are not PCM clipping.

**Meter-stage clarification:** the renderer's existing `processed_lufs` field
measures the bus **before export gain**. Here it reads −9.413 → −8.946 LUFS.
Those numbers must not be described as finished-file loudness. Earlier bass/tone
reports using that field require the same interpretation. The table above measures
the actual finalized PCM files. Loudness is reported, never targeted; there are
no normalized sources, equal-LUFS targets, matched listening copies or true-peak claims.

## What the baseline balance represents

The manufacturer-study fader optimizer produced the current kick +1.5 and snare
+1.0 dB positions. Its accepted deltas and settings were checked against saved
provenance. That search used broad engineering relationship ranges and a movement
penalty, not the musician's forward-rhythm preference. No additional +2 dB offset
was present. The subsequent guitar and bass corrections changed processing, so
that earlier result cannot certify today's perceptually neutral balance.

We remeasured the current processed mix. The existing ranges still contain these
training and held-out medians. The candidate also stays within them:

| Held-out simultaneous role-band relationship | Baseline | Selected |
|---|---:|---:|
| Kick / overheads, low band | +9.242 dB | +11.242 dB |
| Kick / bass and guitars, low band | +8.463 dB | +10.463 dB |
| Snare / overheads, body band | +3.642 dB | +5.642 dB |
| Snare / bass and guitars, body band | +4.559 dB | +6.559 dB |
| Primary guitar / kit, body band | −6.229 dB | −6.563 dB |
| Bass / guitars, low band | +7.064 dB | +7.064 dB |

Those relationships come from the existing covariance-aware balance analyzer;
[its filters, ranges and event definitions](BALANCE_PASS.md) remain provisional.
P10/p90 relationships vary substantially. In-range medians neither prove neutral
perception nor rule out local competition. Existing tom and room deviations remain;
no new band relationship was silently optimized to compensate for the drum change.

The experiment explicitly uses the **current processed fit as a provisional neutral
reference**, with zero previously applied rhythmic offset. It meets +2 dB relative
to that reference. Whether this is also +2 dB relative to the listener's idea of
neutral remains a listening question. Absolute fader positions and makeup alone
cannot answer it. An unknown reference makes the reusable balance rule abstain.

## Source and processing diagnosis

### Kick

The original peak is −4.553 dBFS with zero PCM full-scale contacts. In active
371.5 ms spectral windows the raw kick is concentrated in roughly 40–200 Hz.
The existing +3.5 dB bell at 100 Hz/Q 1.2 adds about 2.7–2.8 dB to 65–100 Hz;
its influence also reaches 100–200 Hz. That is a processing contribution to shared
bass/kick energy, not evidence of faulty recording or a reason by itself to cut it.
The narrow −3.5 dB bell at 265 Hz and +4 dB high shelf at 5.3 kHz remain recorded.
No resonance or capture-position diagnosis is made from them.

Compression remains threshold −23.433885 dBFS, 3:1, hard knee, 9 ms attack,
58 ms release and +5.5 dB makeup. Actual full-song maximum reduction is **8.123 dB**.
Across 412 detected energy events, median attack/body/isolated late-recovery action
is **5.17 / 3.76 / 0.45 dB**. Strong-event quartile attack reduction is 5.98 dB;
quiet-event quartile is 0.024 dB. Held-out median attack/body contrast changes only
−0.096 dB through compression. This evidence does not establish persistent
transient collapse or inadequate recovery requiring an automatic compressor change.
Rapid events can arrive before complete recovery; their pre-hit action and spacing
are logged, and they cannot vote as isolated decays.

A fixed-fader probe reduces the existing 100 Hz bell from +3.5 to +2 dB.
Training/held-out attack-frame output falls approximately **0.74 / 0.76 dB**,
while mean reduction there falls only **0.27 / 0.28 dB**. The 65–100 Hz output
band falls about 0.86 dB, and the pilot mix peak falls 0.15 dB. This trades kick
support for modest compressor relief without an established compression fault.
It is retained as a sensitivity experiment, not selected as a correction.

### Snare

The original peak is −0.949 dBFS with zero PCM contacts. Active coarse spectral
windows have substantial 100–400 Hz body: median raw 100–200 / 200–400 / 3.2–8 kHz
band powers are approximately **−24.04 / −27.32 / −39.31 dBFS**. These are unequal
bandwidth powers, not perceptual timbre scores. The required 90 Hz channel HPF
attenuates the lowest bands. The very broad +3 dB bell at 3.15 kHz/Q 0.11 and
+4.5 dB shelf at 5 kHz together raise 3.2–8 kHz energy roughly **5.6 dB** before
compression. This is a substantial brightness contribution, but raw body remains
strong; neither “thin snare” nor “excessive brightness” is established as a fault.

Compression remains −17 dBFS, 2.5:1, hard knee, 8 ms attack, 12 ms release and
+3.5 dB makeup. Full-song maximum reduction is **4.872 dB**. Across 431 detected
energy events, median attack/body/late-recovery action is **1.18 / 0.39 / effectively
0 dB**; the strong-event attack median is 2.10 dB. Held-out attack/body contrast
changes −0.52 dB through compression. The isolated-event tail/attack median is
−13.9 dB. These results do not show a consistent excessive-sustain or recovery fault.

Quiet-event interpretation is much less secure: **91 of 108** lower-quartile snare
rises coincide within 30 ms of a kick rise, and their median tail/attack ratio is
+1.03 dB. This could include bleed and overlapping playing rather than real isolated
ghost notes with excessive sustain. A compressor or gate must not be chosen from
that ratio alone. In long spans remote from detected snare events, raw median level
is −55.6 dBFS; EQ plus channel HPF changes broadband level by −2.83 dB, before
makeup. This does not support claiming that the existing shelf raises every pause.
The +2 dB artistic fader change does also raise any bleed on that channel by 2 dB
relative to unchanged channels. It does not separate or clean it.

The fixed-fader shelf probe reduces +4.5 to +2.5 dB at 5 kHz. It lowers 3.2–8 kHz
output by about **1.19 dB**, while changing attack-frame broadband output by only
−0.02 to −0.03 dB and body-band power by about +0.01 dB. The pilot mix peak falls
0.12 dB. This can soften snare detail and high-frequency bleed together. Without
isolated bleed evidence or a specified timbre correction, that is an unresolved
preference tradeoff; it was not selected automatically.

### Bass interaction, dense passages and ending

Bass settings and its after-master-HPF samples are unchanged before export.
The kick boost overlaps different fundamentals/harmonics differently. With the
**same baseline-selected** confidently pitched overlap windows in both versions:

| Estimated bass pitch | Windows | Bass / kick, 40–100 Hz: baseline → selected |
|---|---:|---:|
| E1-like, ~41 Hz | 27 | +2.95 → +0.95 dB |
| G1-like, ~49 Hz | 18 | +1.23 → −0.77 dB |
| C2-like, ~65 Hz | 21 | −4.28 → −6.28 dB |
| E2-like, ~82 Hz | 19 | +6.25 → +4.25 dB |
| F♯2-like, ~92 Hz | 20 | −1.91 → −3.91 dB |

These use the existing confidence-limited bass estimator, not verified note labels.
The analysis windows can contain several drum hits. Bass/kick 100–200 Hz ratios
also vary by pitch; C2-like windows remain about +7.68 dB there after emphasis.
Thus one low-band average does not describe all note support. A 2 dB shift is a
measured competition change, not proof of audible masking. The potential cost is
less relative bass prominence on some notes, especially the already kick-dominant
C2-like windows. There is no new evidence requiring a bass EQ or fader revision.
Its prior definition-target deviation and pending listener acceptance remain.

All ten 12-second source sections, including dense guitar/vocal passages around
24–36 and 72–84 s, rapid event sequences, pauses and the ending, were reanalyzed.
No channel transient or compressor guard worsens because the selected processing
is identical. Full-export section RMS falls about **1.09–1.19 dB** across 0–96 s,
and **1.56 dB** in the 96–108 s fade. Guitars and vocals are unchanged before
export, but quieter in the final file and less prominent relative to the drums.
The late tail is not used to declare a steady-tone defect.

## Reusable implementation

`automix/drums.rs` owns source measurement, drum evidence, bounded probes and joint
validation. `balance::rhythmic_emphasis_delta` owns the independently editable
artistic offset. They reuse the production `Source`, BWF alignment, `Strip`, routing,
biquads and FFT. The only bass-module change exposes its existing FFT helpers within
the parent module; bass decisions and DSP are unchanged. No renderer, session schema
or audio-callback processing was changed.

1. Validate explicit kick/snare/DI indices **and filenames**, native session rate,
   unity trims, pan/routing, required 90/40 Hz HPFs, unmatched export and no FX.
   Measurement streams from the common origin, preserving compressor/filter history
   before a pilot. The requested span is bounded to ten minutes.
2. Record 15 synchronized stages. Ten-ms frames retain RMS, sample peak, linked mean
   and maximum reduction, plus raw role-band power. Rational boundaries retain every
   sample, including the final partial frame. Independent 16384-sample stereo-power
   Hann FFTs cover nine bands from 25 Hz to 8 kHz. Incomplete final FFT blocks are
   omitted; time-domain guards and the renderer still cover the complete timeline.
3. Detect a 5 dB raw role-band rise over the prior 40 ms, above −65 dBFS and training
   p95 minus 35 dB. Anchor to the strongest following 30 ms frame, with an 80 ms
   refractory interval. Kick uses approximate 40–120 Hz power, snare 120–500 Hz.
   These bands subtract second-order HPF outputs and are not rectangular FFT bands.
4. Compare 0–30 ms attack, 30–80 ms body and 150–240 ms recovery; log 80–240 ms tails.
   Recovery/tail statistics require no following detected event within 240 ms.
   Exclude events crossing 12-second split boundaries. Even sections train; odd
   sections validate. Require eight events in each split for automatic acceptance.
   Sub-80-ms rolls and overlapping bleed can defeat the detector; no beat, fill,
   microphone-placement or source-separation classifier is claimed.
5. Test at most two static EQ probes, plus unchanged processing: reduce the first
   existing positive kick bell in 70–130 Hz by 1.5 dB, and the first existing positive
   snare shelf in 4–8 kHz by 2 dB. Keep faders fixed and add no EQ bands. These are
   our bounded sensitivity probes, not manufacturer settings. With no established
   tone target or isolated bleed evidence, the coordinator retains EQ and logs
   the reason. General automatic drum tone-target selection remains unimplemented.
6. Compressor relief requires at least eight isolated training events, attack/body
   contrast loss over 1.5 dB in at least 70% of training events, **and** median late
   reduction over 3 dB. Maxima alone cannot trigger it. Propose +2 dB threshold once;
   keep attack/release/ratio unchanged. Measure the change in 0–80 ms event output
   and adjust **existing** makeup by its negative training median, capped at ±2 dB.
   Both splits must show at least 0.25 dB median reduction relief and at most 0.5 dB
   residual event-level change. This path is an initial engineering rule, not a
   universal correct-compression definition. It did not trigger on this recording.
7. Compute artistic fader delta as desired emphasis minus explicitly recorded current
   emphasis, never as desired minus absolute fader. Unknown current emphasis abstains;
   offsets and changes are bounded to 3 dB. Measure processing-only, fader-only and
   combined results separately. Identical processing is reused without redundant DSP.
8. Use baseline event masks and all time-domain frames for technical validation.
   Processing-only active output change stays within ±3 dB; quiet output rise stays
   within 1 dB; added peak reduction stays within 1 dB; every event's attack/body loss
   stays within 1.5 dB. Subtract the explicit artistic fader delta when assessing
   processing output, but retain the full actual mix-peak budget of +3 dB.
   Repeated PCM contact uses the existing source-rule fraction/count thresholds;
   isolated contacts remain reported. Float headroom never becomes that PCM diagnosis.
9. Freeze the pilot candidate, then run `drum-verify` on the complete source timeline.
   It cannot select a replacement or change protected settings. Rejected proposals
   retain the baseline. A successful technical check does not certify preference.

Reports separate baseline evidence/diagnosis, intent and basis, static proposals,
processing plan, makeup change, fader proposal, actual outcomes, applied settings
and null listener preference. No produced-reference features enter these commands.
The independently supplied mix and its prior review remain comparison evidence;
we did not pursue its spectrum, loudness or instrument balance.

## Commands and local evidence

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
mkdir -p artifacts/automix/my-drum-review
# Inspect a pilot without proposing or rendering.
target/release/gigpies drum-analyze BASELINE.json SOURCES NEW-ANALYSIS POLICY.json 24 48
# Bounded processing probes, processing/fader proposals and held-out checks.
target/release/gigpies drum-correct BASELINE.json SOURCES NEW-PILOT POLICY.json 24 48
# Reanalyze a frozen candidate; no search against full-song/held-out results.
target/release/gigpies drum-verify BASELINE.json NEW-PILOT/settings.json SOURCES NEW-FULL POLICY.json 0 120
# Render only the accepted settings after reading the outcomes.
target/release/gigpies render NEW-FULL/settings.json SOURCES NEW-RENDER
python3 scripts/drum_evidence.py NEW-FULL --baseline
python3 scripts/drum_evidence.py NEW-FULL
```

Every command output directory must be new. Example policy for this session:

```json
{
  "kick": 0, "kick_file": "01_Kick.wav",
  "snare": 1, "snare_file": "02_Snare.wav",
  "bass": 7, "bass_file": "08_BassDI.wav",
  "neutral_basis": "Current processed fit, provisional; no previously added rhythmic offset",
  "current_emphasis_db": [0, 0],
  "desired_emphasis_db": [2, 2]
}
```

This does not turn a text assertion into perceptual evidence. Record the actual
basis and use `null` for unknown current emphasis. Always start from a saved baseline;
rerunning from corrected settings with current emphasis reset to zero would stack
an intentional gain twice.

The local report root is `artifacts/automix/drums-v1/`. `experiment-plan.json` records
the budget before the probes. `pilot/`, `kick-pilot/` and `snare-pilot/` preserve the
initial measurements. `selection-final/` runs the final reusable implementation and
reproduces the unchanged frozen settings; it is not another candidate search.
`full-review/` contains fixed-candidate full-source validation, synchronized windows,
per-note comparisons and section summaries. `baseline-balance/` and `candidate-balance/`
contain complete existing-policy measurements. `final/` retains the production
render, float buses and renderer reports. `verification.json` independently verifies
final PCM peaks, settings, timing, source hashes and exact excerpt bytes.

Only this identical 12-second pair is prepared, starting at 24 s:

1. `listen/01-CURRENT-BASELINE-024s.wav`
2. `listen/02-NEW-CORRECTED-MIX-024s.wav`

It keeps each full export's gain. Additional EQ candidates remain settings and
measurements; they have no listening queue. Nothing has been played. On a later
playback request, use `scripts/play_pair.py` with the explicit sink and `--gap 1`:
finish both files' and sink preflight before playback, play each once, and do no
preparation between clips. See [the established playback contract](REFERENCE_REVIEW.md).

## Confidence, remaining limitations and soundcheck

- **Achieved:** +2 dB drum prominence relative to the explicitly documented processed
  baseline, with preserved guitar/bass processing, independent makeup/faders, source
  timing, stereo relationships, pan, input trims and HPFs. Full technical guards pass.
- **Measured cost:** greater low-band kick competition on every overlapping bass
  note, louder relative drum bleed, and 1.682 dB attenuation of all unchanged
  contributions after peak finalization. The full export is about 1.2 LU quieter.
- **Unresolved:** preferred neutral balance, snare brightness, ghost-note/bleed
  separation, local perceptual masking and listener acceptance. No new source-tone
  defect was established or repaired. Existing bass definition and kit/room policy
  deviations remain; this pass does not claim all musical targets are met.
- **Confidence:** high for verified configuration, sample peaks, measured gain changes
  and unchanged processing; limited for note labels and event identification;
  insufficient for physical capture causes, preferred tone and audible masking.
  All split tests are within this previously studied song, not independent blind
  validation or proof of generalization.

For a future soundcheck, first identify whether the recording tap is pre/post
console EQ, dynamics and any gate, and whether each control affects the recorded
path. Record isolated quiet/strong kick and snare hits, rapid repeats, sustained
snare tails, bass-only notes, kick-only passages and controlled overlaps at unchanged
input gains. If an isolated measurement then confirms unwanted snare tail or bleed,
compare one physical damping/placement or signal-path-relevant processing change
and **repeat the same measurement**. If low-band support remains problematic,
repeat several bass notes with and without kick. Do not prescribe an amp control
for the verified DI or a console EQ/gate for a pre-processing recording tap.
These are conditional experiments, not inferred capture faults. No repeat recording
or hardware action was started during this historical-source review.

## Validation and retention

The complete normal suite passes: **86 Rust tests and eight Python tests**;
formatting, Clippy with warnings as errors and the release build pass. Seven new
normal drum regressions cover silence, steady bleed, quiet hits, rapid repeats,
recovery exclusions, insufficient evidence, explicit/unknown rhythmic intent,
excessive changes, transient loss, held-out independence, fixed masks, bounded EQ
probes, measured-relief requirements, real production compression, kick/bass overlap,
history continuity, routing/identity and evidence non-overwrite. They use synthetic
signals and open no hardware. Three historical private-media/exhaustive tests remain
intentionally ignored; this task's pilots, frozen full-source checks, final render
and exact PCM verification ran explicitly. No old audition matrix was regenerated.

All 13 original recordings, both previous full exports and the baseline configuration
retain their hashes. Existing unrelated uncommitted source, media and reference work
is preserved. Task artifacts are local and ignored; no generated audio is published.
