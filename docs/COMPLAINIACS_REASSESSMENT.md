# Complainiacs reassessment: balance before more correction

**Historical pass.** The listener subsequently heard this revised FINAL and still
found the snare prominent, then withdrew earlier numerical mixing preferences.
The [current workflow reassessment](COMPLAINIACS_WORKFLOW_REVIEW.md) starts from
this exact revised state, selects a separate makeup-only change and supplies the
complete SOURCE → selected FINAL checkpoint. Earlier decisions below retain their
provenance; they are not current requirements. Superseded processed audio may be
retired under the user's later cleanup instruction; settings and evidence remain.

Prepared locally, 2026-10-02. **One new fader-only FINAL is selected for listening.**
It is implemented and offline-validated, with listener acceptance pending. No playback,
host-audio changes, commit or push occurred. The five other examples retain their
settings and exports. Private audio, complete evidence and exact settings are indexed
in `artifacts/automix/complainiacs-reassessment-v1/LISTEN.md`.

## What most likely caused the concerns

1. **Foreground placement changed substantially (high confidence).** SOURCE uses
   the original zero faders and established pans. The manufacturer-stage fader search
   moved the primary guitar to −6 dB and lead vocal to +6 dB, a 12 dB change between
   those two paths before other processing. The second guitar path went to −1 dB;
   vocal room went to −6 dB. These were optimizer choices against provisional ranges,
   not independently justified musician preferences for each channel.
2. **The source correction changed the ensemble relationship (high confidence for
   measurements, moderate for perception).** The stronger guitar correction restored
   body but reduced presence. On fixed SOURCE-selected singing/overlap windows, the
   training vocal-sum/guitar presence median rose from −1.86 to +0.43 dB even though
   neither fader moved. A better isolated body ratio did not guarantee guitar prominence.
3. **Extra drums imposed an export cost (high confidence).** The +2 dB kick/snare
   step increased the full-song peak by 1.682 dB. Independent finalization lowered
   every unchanged contribution by 1.682 dB. Bass and guitars became quieter in the
   exported comparison; vocal/guitar balance itself did not change at this step.
   The fader increase also raised whatever bleed those drum microphones contain.
4. **The vocal chain is a plausible harshness contributor (moderate confidence).**
   Its +2 dB/2 kHz/Q 0.56 and +3.5 dB/6.7 kHz/Q 0.11 bells overlap. The complete EQ
   response is about +4.86 dB at 2 kHz and +4.78 dB at 3.2 kHz, before compression.
   On raw-active windows, EQ plus HPF adds a median 2.82 dB broadband. Compression
   including the separate +2 dB makeup changes that by a median −0.47 dB; the artistic
   fader then adds +6 dB. This is stronger evidence for processing/placement than
   for a defective original voice. Sibilance, microphone cause and audible harshness
   have not been automatically identified.
5. **Bass definition was not the same objective as body (high confidence for the
   tradeoff).** The +4 dB/700 Hz, −2 dB/180 Hz correction improved its numerical
   definition/body ratio partly by removing body. Held-out bass/guitar body changed
   +0.82 → −0.03 dB. The original −7.5 dB/35.5 Hz shelf and mandatory 40 Hz master
   HPF also attenuate the lowest notes unequally. This does not establish that the
   DI needs a sub boost. Its light compressor action does not support a compression
   repair. The later drum increase further reduced bass/kick prominence.

The finished spectrum **does not support one uniform treble fault**. At 24–36 s,
SOURCE/CURRENT 100–400-to-800–3200 Hz ratios are 5.02/4.45 dB; at 48–60 s they are
6.61/9.04 dB. CURRENT is body-heavier by this measure in several instrumental
sections while still placing the singing voice farther forward. Whole-mix occupancy
cannot identify guitar or bass tone. No master shelf was selected from it.

SOURCE can plausibly work better where the unboosted vocal sits in the guitar
texture, the original guitar levels matter, or the later peak-determined attenuation
reduces sustained support. At 96–108 s CURRENT is 4.86 dB lower in whole-mix RMS
than SOURCE; spectral decay and coherent sums complicate attributing all of that to
one instrument. These are explanations to test by listening, not a claim that the
agent heard or preferred either file.

## Decision history and provenance

| Stage | Consequential change and basis | What validation established | Acceptance / remaining uncertainty |
|---|---|---|---|
| SOURCE | Original selected sources, DI-only bass, initial faders and pan; independent peak finalization | File identity, native alignment, routing and output level | Listening reference; no later faders silently substituted |
| Unity-source revision | Unity trims, required 90/40 Hz HPFs, separate makeup and independent export gain followed earlier instructions | DSP/routing and sample-peak contracts | These foundational contracts remain; SOURCE bypasses HPFs |
| Manufacturer-derived | Mapped Yamaha EQ, compressor values and output gain; explicit RBJ shelf, knee/release adaptations; no FX | Applied parameters, synthetic DSP tests and fixed-fader renders | Starting points, not console emulation or recording-specific diagnoses |
| Threshold adaptation | Kick threshold raised about 0.57 dB from measured action | Limited actual relief; old reduction budget still slightly unmet | No general kick compression failure proved |
| Manufacturer fader pass | Kick +1.5, snare +1, OH −5, room 0, toms +4.5/+1/+1, DI −1, guitars −6/−1, voice +6, vocal room −6 dB | Search improved its provisional objective; some targets remained unmet | “Accepted” in optimization is algorithmic. Later feedback reported improved overall balance, not approval of every parameter |
| Guitar tone / decay | Earlier +1/−1 correction replaced by +5 dB/300 Hz and −3 dB/2400 Hz, Q 0.7, on primary only | Steady-tone, coherent-sum and dynamics checks; fade excluded only from steady-tone objective | Body improvement was later recorded as listener-accepted. Original TONE_DECAY said pending; later DRUMS/inventory record acceptance. Exact breadth of that verdict is not recoverable from these files |
| Reference review | Separate supplied-reference alignment and native-gain comparison | Eight aligned main sections; ending abstained | Playback completed; supplied mix was disliked. Reference never selected settings |
| Bass correction | +4 dB/700 Hz and −2 dB/180 Hz, Q 0.7, from a provisional definition intent | Repeated-note, crest, reduction and headroom checks; partial definition improvement | Bass preference remained unresolved. Isolated PCM plateau remains unrepaired |
| Rhythmic candidate | +2 dB kick/snare relative to a **provisional** neutral baseline | Intended relative offset and technical protections; 1.682 dB export penalty | Relative neutrality was never established by the numerical fit; latest feedback reopens the offset |
| Bleed work | Detector/recovery repairs; static and predictive cleanup withheld | Useful identifiability limits and protection failures | Listener-confirmed snare bleed remains. No cleanup applied or experiment repeated here |
| Six-example checkpoint | Preserved the rhythmic candidate as Complainiacs FINAL | Complete files, correct initial SOURCE faders, exact excerpts | Playback log records all 12 clips completed. This task's new feedback is the first explicit verdict on that checkpoint and rejects aspects of FINAL |

The older source passes replaced appended guitar corrections rather than repeatedly
stacking them. Makeup is applied once inside each compressor; no new compensation
was added by fader or export stages. The guitar microphones are summed coherently.
Held-out broadband coherent-minus-independent guitar power is about +0.24 dB in
SOURCE and +0.44 dB in CURRENT: there is no evidence here for a blanket polarity
repair. Band-specific interactions can still vary. Both paths retain their relative
fader difference and pan in the new candidate.

There is no master limiter/FX action in these mixes. The processed master retains
its 40 Hz HPF. Export gain is a separate constant, not compressor makeup or an
artistic fader. Float bus samples above unity are headroom, not clipped output PCM.
Earlier guitar/bass documents quoted renderer `processed_lufs` as though it described
the finished file. It is pre-export: the guitar and bass finished files are both
about −15.8 LUFS, not −9.34/−9.41 LUFS. Historical numbers remain preserved with
this stage correction.

## Bounded pass and rejected proposals

`plan.json` was written before new candidate evaluation. Budget: **three processing
proposals, one fader proposal, one combined proposal; no held-out retries**. Earlier
baselines were genuine alternatives. Even origin-aligned 12-second sections trained;
odd sections checked the frozen choice. Boundary-crossing windows do not vote.
The whole song was previously studied, so this is not a blind generalization test.
Exploratory bass note-support results were inspected across both splits before the
selection; the selected fader candidate’s held-out relationships were inspected only
after selection. All five settings were frozen before measurement, with no retries.

- **Vocal EQ reversal:** set only the two positive bells to 0 dB, fixed faders and
  makeup. Training active 20 ms windows show up to 2.887 dB crest loss, with 37
  windows over the frozen 1.5 dB budget; held-out has 40 such windows, maximum
  2.761 dB. The proposal and both bundles containing it were rejected. This conservative
  per-window check is stricter than the guitar rule's split-summary crest check;
  it does not prove audible damage. Its suitability for vocal timbre changes remains
  a research/listening question. No threshold was relaxed and no smaller retry run.
- **Bass body restoration:** remove only the appended 180 Hz cut. Repeated-note
  fundamental changes are approximately −0.24…+0.95 dB; maximum output rise 1.50 dB,
  crest loss 0.72 dB, quiet-window rise 0.82 dB and added reduction 0.59 dB. Those
  physical budgets pass. The old definition objective regresses about 1.3 dB and
  its per-window tone guard fails in 75 training/53 held-out eligible windows.
  Those remain failures, not an automatic-policy acceptance. Restored body is a
  plausible artistic alternative, but it alone misses the dominant ensemble issue;
  it was not selected or combined in another search.
- **Earlier baselines:** removing the last drum offset recovers export headroom,
  but leaves the forward vocal/guitar relationship. Earlier manufacturer processing
  loses the subsequently supported guitar-body improvement. SOURCE preserves useful
  relationships but is not the required processed-HPF baseline. None better addresses
  this task's combined direction on available evidence.
- **Selected fader-only proposal:** directly addresses instrument relationships
  while every channel's EQ, compression, makeup and source samples remain identical.
  Production channel meters and reduction maxima are identical to CURRENT. The
  full mix peak falls rather than consumes additional headroom.

### Exact selected settings

| Channel | CURRENT fader | REVISED fader | Change |
|---|---:|---:|---:|
| Kick | +3.5 | +1.5 | −2 dB |
| Snare | +3.0 | +1.0 | −2 dB |
| Bass DI | −1.0 | 0.0 | +1 dB |
| Guitar primary | −6.0 | −4.0 | +2 dB |
| Guitar secondary | −1.0 | +1.0 | +2 dB |
| Lead vocal | +6.0 | +3.0 | −3 dB |

All other faders remain. All makeup remains separately editable and unchanged.
DI-only routing, native timing, stereo/pan relationships, unity trims and required
HPFs remain. No added gate, EQ, master processing, normalization or automation.

### Ensemble checks and policy limitations

Fixed SOURCE-selected simultaneous windows retain the same members across variants.
These are overlapping filter-band energy ratios before the master, not audibility
scores or universal target values.

| Held-out relationship | SOURCE | CURRENT | REVISED |
|---|---:|---:|---:|
| Vocal sum / guitars, presence | −7.02 | −1.74 | −6.02 dB |
| Vocal sum / guitars, broadband | −3.54 | −0.53 | −4.71 dB |
| Guitars / kit, body | −6.43 | −4.79 | −2.43 dB |
| Bass / guitars, body | −2.58 | −0.03 | −1.03 dB |
| Bass / kit, low band | −8.36 | −4.33 | −2.72 dB |

The presence relationship also falls in both quiet and strong raw-activity strata.
Quiet activity can contain bleed; it is not automatically quiet singing. At the
ending there is no eligible vocal relationship, so the report abstains. Guitar/kit
body rises about 2.2–2.5 dB across main sections and about 2.15 dB during the fade.
The ending is consequently a useful second listening passage.

**Technical validation is distinct from the old artistic policy.** The default
optimizer would not approve this change: held-out vocal/guitar violation becomes
3.94 dB, room/close-hit violation 3.02 dB and vocal-room/direct violation 7.89 dB;
two tom/overhead failures also remain. They are recorded unchanged as failures.
The 0…+12 dB vocal target conflicts directly with the new instruction to place the
voice within guitars and the SOURCE relationship the listener preferred. It is not
used as a preference objective in this pass. The old optimizer and protections
were not edited to admit this candidate. This is an explicitly selected artistic
fader revision, not a successful automatic-policy result. Room balance remains a
tradeoff for listening, especially as lowering direct vocal raises room/direct ratio.

## Reusable method changes

`balance-source-analyze` measures actual bypass audio with the supplied initial
routing, preserving production source reading and pan behavior. It cannot infer
which settings are historically SOURCE; the caller must supply that provenance.
`groups.csv` now includes the coherent whole kit and direct-plus-room vocal sum,
plus independent channel power for comparison with coherent power.

`scripts/ensemble_review.py` compares variants using training-derived SOURCE activity
thresholds. It refuses processed evidence masquerading as SOURCE, changed raw input,
alignment, channel identity or pan. It reports paired changes by 12-second section,
training/held-out split, and quiet/strong raw-activity strata. Channel processing,
makeup, faders and optional export gain are labelled separately. Insufficient counts
and absent relationships remain visible; preference is never filled automatically.
No song name changes behavior and no new preferred ratio is installed for other songs.

This closes a measurement gap, not the artistic decision problem. The prior optimizer
already had section guards, but its objective could still reward a foreground vocal
and source rules could improve their own ratios while changing ensemble prominence.
The new report makes that interaction reviewable without another large search.

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
target/release/gigpies balance-source-analyze INITIAL_SOURCE.json SOURCES NEW_SOURCE_REVIEW
target/release/gigpies balance-analyze CANDIDATE.json SOURCES NEW_CANDIDATE_REVIEW
python3 scripts/ensemble_review.py SPEC.json NEW_REPORT.json
```

SPEC examples and complete commands are retained in the private checkpoint. This
observer uses the existing DI-only balance contract. Dark Ride's undocumented bass
capture does not become eligible for a DI analyzer by implication.

## Targeted research

Question: could the apparently “high” 6.7 kHz vocal bell substantially boost the
2–3 kHz region in this implementation? The
[W3C Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/) defines peaking
coefficients and Q/bandwidth relationships. Those equations match this engine's
coefficient law; Q 0.11 gives a broad response. Calculating the cascade confirms
roughly +4.8 dB around 2–3.2 kHz from the whole EQ. This supports testing overlap
between existing boosts. It does not establish Yamaha transfer-function equivalence,
a harshness threshold, or a preferred vocal EQ. Actual production measurements and
the rejected probe are separate evidence. No new masking model, true-peak limiter,
source-separation experiment or speculative “mud” correction was needed to make
this decision.

## Finished export and listening

All complete files are 44.1 kHz stereo 24-bit PCM, 5,038,080 frames (114.242 s),
independently finalized to **−0.01 dBFS sample peak**. No true-peak claim.

| File | Export gain | Finished RMS | Finished integrated loudness |
|---|---:|---:|---:|
| SOURCE | −7.057 dB | −18.912 dBFS | −15.6 LUFS |
| CURRENT FINAL | −8.037 dB | −20.301 dBFS | −17.0 LUFS |
| REVISED FINAL | −6.246 dB | −18.733 dBFS | −15.4 LUFS |

REVISED gets 1.791 dB more export gain than CURRENT. Therefore finished contributions
change by **+3.791 dB guitars, +2.791 dB bass, −1.209 dB lead vocal, −0.209 dB
kick/snare**; other unchanged channels rise 1.791 dB. Within-mix relationships still
follow the fader changes. The louder new overall export can influence preference;
no equal-LUFS copy or per-clip normalization was made.

Listen in SOURCE → CURRENT FINAL → REVISED FINAL order at **24–36 s**, then optionally
**90–102 s** for the final drum event and guitar decay. Every clip is verified against
its parent's exact PCM bytes and gain. The private index links complete files,
settings, manifests, candidate reasons and finished measurements.

## Validation and unresolved work

103 normal Rust tests and 24 Python tests pass. Focused balance tests also pass after
the final metadata/error-path edits; formatting, Clippy with warnings denied and the
locked release build pass. New regressions check SOURCE against actual renderer
samples, coherent cancellation, fixed activity, section conflicts, export-stage
labels and rejection of altered provenance/alignment. Complete read-only Rainfall
and Catbite remeasurements reproduce all prior channel windows byte-for-byte and
all prior group fields exactly; Complainiacs does too. No other example was retuned.

Three historical/private audition tests remain ignored. Exhaustive searches, old
render matrices, historical auditions and snare-identifiability experiments were
intentionally skipped. Current bounded measurements, one selected full render,
original hashes and exact clip verification ran explicitly. No test claims musical
preference, safe source separation or hardware verification.

Unresolved: vocal timbre at the lower fader, room/direct preference, stronger guitar
sustain, whether bass needs restored body beyond a level move, and the validity of
very short crest guards for broad vocal EQ. Source tone policies remain provisional;
no protection threshold or general song preference was silently changed. The new
complete FINAL is ready for listening despite those limits.

Retention: about 107 MiB of redundant task-created exports/measurements were removed
or linked to identical retained evidence. The selected PCM/float mix, clips, settings
and diagnostic evidence remain local; roughly 314 MiB is retained. Original media,
prior exports and unrelated uncommitted work were preserved.
