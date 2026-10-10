# Complainiacs workflow reassessment

**Subsequent contract revision:** [source preservation](../../guides/SOURCE_PRESERVATION.md)
makes SOURCE eligible as FINAL without mandatory offline HPFs. This does not change
the selected Complainiacs mix, its failed alternatives or the historical findings below.

Prepared locally, 2026-10-02. **Selected for listening: reduce snare compressor
makeup from +3.5 to +1.906326 dB; retain the revised FINAL's faders and all other
processing.** This is an offline-validated listening hypothesis. Listener acceptance
is pending. No playback, host-audio change, commit or push occurred.

The complete six-example checkpoint, full exports, exact settings and verification
are indexed in `artifacts/automix/complainiacs-reassessment-v2/LISTEN.md`.
It contains SOURCE → selected FINAL only. The earlier
[reassessment](COMPLAINIACS_REASSESSMENT.md) remains historical evidence.

## Starting state and evidence ownership

The supplied current-state correction is confirmed by the saved settings and render:
kick **+1.5 dB**, snare **+1 dB**, DI **0 dB**, guitars **−4/+1 dB**, direct vocal
**+3 dB**. The earlier extra +2 dB drum step was already removed. Snare makeup was
still **+3.5 dB**. This pass does not repeat that fader reversal.

| Evidence or decision | Provenance | What it establishes now |
|---|---|---|
| Earlier +2 dB rhythmic emphasis and numerical mixing preferences | User-prescribed; explicitly withdrawn | Historical intent only; no current requirement or preferred neutral reference |
| Snare remains prominent in revised FINAL | Latest provided listening observation | A concern to investigate; no prescribed subtraction or diagnosis |
| Forward/harsh voice, reduced guitar prominence, insufficient bass/body, useful SOURCE passages | Earlier provided listening observations | Context for assessing alternatives; no mandatory EQ, fader or target ratio |
| Yamaha EQ, compressor timings and output gains | Manufacturer starting points plus documented DSP adaptations | Parameter provenance; no evidence these settings suit this recording |
| Manufacturer fader search | Engineering relationship ranges and movement penalty | Improved its own objective; did not establish preferred instrument placement |
| Guitar and bass source corrections | Actual DSP and provisional tone policies; limited historical guitar feedback | Measured tone changes and guards; neither establishes whole-ensemble preference |
| Source integrity, current chain, event contributions, export meters | Observed in this pass | Reproducible technical evidence, with stages and contexts identified |
| New selection | Engineering judgment informed by the latest observation and training evidence | A bounded candidate to hear; no claimed listening verdict |

SOURCE remains the established selected originals through their **initial faders
and pan**, with no channel/master DSP. Later FINAL faders do not redefine it.
Its independent export gain is permitted; source channels were not normalized.

## Reconstructing the chain

The snare path is native input → unity trim → 90 Hz HPF → existing EQ → linked
compressor → explicit makeup → +1 dB fader/centre mono pan → coherent band sum →
40 Hz master HPF → independent constant export gain. There is no FX, master
limiter, dynamic gain automation, gate, polarity change or timing correction.
Centre mono routing contributes −3.0103 dB per-side power; that convention also
applies to SOURCE. Stereo overheads and room retain their native sides.

Existing snare EQ remains −0.5 dB at 132 Hz/Q 1.2, a neutral 1 kHz band,
+3 dB at 3150 Hz/Q 0.11, and a +4.5 dB shelf at 5 kHz. Earlier production
measurements found roughly 5.6 dB additional 3.2–8 kHz energy before compression.
The broad bell and shelf therefore remain plausible contributors to perceived
edge, but that does not establish excessive brightness as the current fault.

Compression remains −17 dBFS threshold, 2.5:1, hard knee, 8 ms attack and 12 ms
release. On fixed 0–80 ms source-event windows:

| Measured stage change | Quiet-event quartile, held-out | Strong-event quartile, held-out |
|---|---:|---:|
| HPF/EQ broadband change | +1.283 dB | +0.474 dB |
| Compressor energy loss, excluding makeup | 0.006 dB | 1.991 dB |
| Inherited makeup | +3.500 dB | +3.500 dB |
| Artistic fader | +1.000 dB | +1.000 dB |

These quartiles use training-derived thresholds and contain 51/53 held-out
events. Quiet events can contain kick bleed or quiet unison playing. They are
not isolated ghost notes. The gain raises low-action events almost in full;
strong events receive less net gain because the compressor works harder.
There is no new evidence of a compressor recovery fault requiring different
attack, release, ratio or threshold.

The [Yamaha 01V96 manual](https://data.yamaha.com/files/download/other_assets/5/334235/01v96v2_om_en_f0.pdf),
printed page 277, describes output gain as an output-level control. It does not
make a preset's gain measured loss compensation for this source or this DSP.
The retained local manual/extract was consulted; a fresh primary-site fetch returned
403. No console equivalence or new manufacturer recommendation is inferred.

## Ensemble diagnosis

The added production measurements retain coherent sums for the close snare,
remaining kit, drum ambience and remaining ensemble. Omission below is a
**diagnostic calculation**, not rendered removal or source separation. Correlated
microphones can reinforce or cancel; independent channel powers cannot predict
their combined contribution.

| Fixed held-out event context | SOURCE: close snare / remaining kit | Revised FINAL | Selected FINAL |
|---|---:|---:|---:|
| Strong events, 53 | −9.50 | −4.24 | −5.83 dB |
| Quiet hypotheses, 51 | −20.92 | −13.29 | −14.88 dB |
| Raw-vocal activity, 92 | −10.54 | −3.95 | −5.54 dB |
| Low raw-vocal activity, 123 | −11.34 | −5.86 | −7.45 dB |
| Compound events, 39 | −10.85 | −5.14 | −6.73 dB |
| Late section 84–96 s, 32 | −10.68 | −4.99 | −6.59 dB |

These are broadband relationships in the same events, before master/export.
**None is a preferred balance target.** Raw-vocal activity is a proxy for singing;
compound/rapid events expose fill-related uncertainty rather than transcribing
fills. The clear instrumental 48–60 s section, vocal passages and late section
are separately retained. The 96–114 s decay is checked in time-domain/export
measurements despite having no qualifying snare events.

The close-snare contribution has moved forward relative to the rest of the kit
in several contexts, not just in a whole-song average. In training strong events,
its level changes from about −19.16 dBFS in SOURCE to −16.13 in revised FINAL,
while overheads change −12.41 → −17.76 and room −15.43 → −16.89 dBFS.
The −5 dB overhead fader is one inherited contributor to this relationship.
That evidence does not authorize raising overheads: doing so would also change
cymbals, stereo kit and bleed, without a new listening basis.

Rooms and overheads remain substantial. Omitting the close snare would lower
held-out strong-event kit level by a median 1.13 dB in revised FINAL, versus
0.80 dB in the selection. It would **raise** quiet-event kit level by about
0.32/0.29 dB because of destructive interaction. Thus lowering this fader/makeup
cannot remove all perceived snare, and quieter close-mic samples need not lower
every instant of the coherent mix. No phase repair is inferred.

Likely causes and confidence:

- **High confidence in gain attribution:** inherited makeup, +1 dB close fader,
  reduced overhead level and retained processing place the close path farther
  forward than SOURCE. The previously reversed +2 dB step is not the current cause.
- **Moderate confidence in perceptual relevance:** reducing that contribution
  addresses a controllable part of the latest concern. No agent listening verdict
  supports calling the selected level preferable.
- **Moderate plausibility, unresolved preference:** broad snare boosts can contribute
  edge. The tested EQ alternative fails protection and remains unapplied.
- **High confidence in source ambiguity:** room/OH and close tracks contain mixtures.
  Earlier snare-bleed identifiability failures remain valid. No experiment was
  repeated, and no cleanup is enabled.
- **High confidence in export attribution:** a late peak constrained revised FINAL;
  the selected mix's maximum instead occurs around 77.93 s. Neither master dynamics
  nor export gain selectively amplifies snare.

## Bounded pass and selection

`plan.json` preceded new candidate evaluation. Budget: **two processing proposals,
one artistic control, at most one combined proposal; zero held-out retries**.
Even origin-aligned 12-second sections trained; odd sections validated. Boundary
crossing events/windows cannot vote. These are checks within an already studied
song, not a blind generalization experiment.

1. **P1, makeup only:** 46 unambiguous strong training events, without another
   event inside 80 ms, supply an energy-weighted compressor loss of **1.906326 dB**.
   Replace the inherited +3.5 dB with this value. EQ, dynamics and faders remain.
   This is an explicit electrical reference for one listening hypothesis, not a
   general makeup rule or definition of musical neutrality.
2. **P2, broad bell removal:** set only the existing 3150 Hz bell to 0 dB at fixed
   faders and makeup. It fails the unchanged 1.5 dB crest guard in **18 training
   frames**, with maximum loss **1.921 dB**. Reject it; no smaller retry or threshold
   revision. This does not prove audible damage or validate that guard for every
   kind of EQ; it remains a protection failure under the current method.
3. **A1, artistic control:** put P1's −1.593674 dB gain change at the snare fader
   instead, leaving makeup +3.5 dB. Actual production routed measurements agree
   with P1 within 4 × 10⁻¹² dB in training. There is no acoustic evidence that one
   control location is inherently better in this FX-free chain. P1 was selected
   to make the makeup reference explicit and retain artistic faders. **Do not
   combine P1 and A1:** that would apply the same gain intent twice.

The selection was frozen before inspecting new held-out results. Complete
`drum-verify` then passes the unchanged output, quiet-level, crest/attack-body,
compression, source-contact and mix-peak protections. Attack/body shape and
compressor action are unchanged to numerical precision. Combined validation is
P1 at the retained artistic balance; no additional fader/EQ correction was stacked.

Unchanged revised FINAL was the fallback. Earlier rhythmic, bass, guitar and
manufacturer baselines were legitimate alternatives. The rhythmic version restores
withdrawn emphasis; bass/guitar versions restore the earlier vocal/guitar placement
implicated by listening; manufacturer processing also loses the later supported
guitar-body change. None supplied a demonstrated advantage for this concern. SOURCE
remains a valid listening alternative but bypasses required processed HPFs, so it
cannot silently become the processed replacement.

The old balance policy still fails. Held-out tom/OH deviations remain 0.95/0.82 dB,
vocal/guitar 3.94 dB and vocal-room/direct 7.89 dB. Room/close-hit deviation increases
**3.02 → 3.26 dB** because the close contribution is lower. These are recorded policy
failures; no range was widened. The explicit policy's musical objective is not the
withdrawn user intent, and passing technical protection is not policy compliance.

## Other accumulated decisions

The prior vocal EQ reversal's crest failure and bass-body restoration's tone-policy
failure remain recorded and unselected. Their guards were not replaced here.
The guitar correction can restore body while reducing presence, and the DI's
180 Hz cut can improve a definition ratio while reducing body. Those tradeoffs are
retained, not relabelled as universal repairs. The revised vocal/guitar presence
relationship and bass/guitar body relationship are unchanged by the selection;
guitar/kit and bass/kit move only slightly as close snare falls.

DI-only routing, original samples/timing, stereo relationships, pan, unity trims,
required channel/master HPFs and independent makeup/faders remain verified. No
foundational-contract revision is proposed. The bass's known isolated PCM plateau
is still unrepaired. Absolute faders, matched spectra and in-range medians remain
insufficient evidence for preferred balance.

## Reusable workflow repairs

- `drum-verify` previously rejected fixed-fader processing solely because musical
  neutral balance was unknown. Unknown intent now withholds automatic fader changes
  while independent processing retains every technical guard. `drum-correct` uses
  the same separation. A new production-DSP regression verifies acceptance at fixed
  faders, refusal of unrequested movement and continued technical rejection.
- Balance CSVs now expose coherent close-snare, remaining-kit, ambience, tom, kick
  and remaining-ensemble groups. Synthetic correlated/anticorrelated signals check
  the sums, including FX in the ensemble. Existing fields retain their values.
- `scripts/snare_context.py` reports fixed-event, section, quiet/strong and vocal
  activity contexts, including signed interaction and insufficient evidence. It
  never selects settings or labels a clean source. Regressions cover cancellation,
  silence, changed input, SOURCE provenance and held-out independence.
- Listening preparation now requires **pinned initial SOURCE settings and WAV
  hashes**. Matching routing alone previously allowed later faders to masquerade
  as SOURCE. All parents and metadata are checked again before publishing the
  completion manifest. Regression cases cover wrong SOURCE and metadata replacement.
- Renderer reports now label legacy `bypass_lufs`/`processed_lufs` explicitly as
  **before export gain**. Finished PCM is measured separately, avoiding the earlier
  misleading −9 LUFS descriptions of roughly −16 LUFS finished files.

No numerical protection or preference target changed. No per-song special case or
automatic loudness compensation was added. The workflow preserves a clear path:
record intent/provenance → diagnose source and ensemble → freeze a small budget →
test processing at fixed faders → assess separate faders → freeze selection →
validate complete output → prepare one verified listening set → await acceptance.
Interrupted/failed preparation has no ready manifest; recovery uses a new output
directory and the same frozen inputs. Previous settings and evidence remain available.

## Finished exports and listening

Complainiacs exports contain **5,038,080 frames**, 44.1 kHz, stereo 24-bit PCM.
All six examples' full SOURCE and FINAL files are independently finalized to
**−0.01 dBFS sample peak**. No source normalization, equal-LUFS targets,
loudness-matched copies or true-peak compliance claim.

| Complainiacs | Export gain | Finished integrated loudness |
|---|---:|---:|
| SOURCE | −7.0568 dB | −15.6 LUFS |
| Revised FINAL heard before this task | −6.2460 dB | approximately −15.4 LUFS |
| Selected FINAL | −6.2063 dB | −15.5 LUFS |

The selection gets **+0.039612 dB** more export gain. Thus the close-snare contribution
falls **1.554062 dB** in the finished file; other unchanged contributions rise
0.039612 dB. Main-section RMS falls only 0.08–0.12 dB. This is a local relationship
change with very little global headroom recovery. The peak moves from 91.1598 s/right
to 77.9298 s/left. Late decay RMS is effectively unchanged.

The checkpoint uses the established passages: Complainiacs **24–36 s**,
Dark Ride **48–60 s**, Rainfall **240–252 s**, Wild & Co **168–180 s**,
Catbite **156–168 s**, Phoenix **36–48 s**. Complainiacs also has **90–102 s**
for the late drum event and decay. All 14 clips retain their parents' exact PCM
samples, format, position and gain. There are no intermediate mixes in the set.

## Validation, retention and limits

**105 normal Rust tests and 29 Python tests pass**, with formatting, Clippy with
warnings denied and the locked release build using Rust 1.97.1 and
`CARGO_INCREMENTAL=0`. Focused drum, balance, contribution and checkpoint tests ran
during implementation. The complete normal suite ran for the shared measurement,
verification and render-report changes.

Other examples received focused checks, without retuning:

- Rainfall, Wild & Co and Catbite: fresh complete production balance measurements
  reproduce prior channel windows/master CSVs and every prior group field exactly.
- Phoenix: complete selected-mix analysis; its saved full-song guitar settings are
  unchanged. No numerical snare/band model was invented for flute/violin/guitar.
- Dark Ride: a full production rerender is byte-identical for both PCM exports and
  both float buses. Both complementary bass parts remain; undocumented capture
  identity does not become a DI claim. Duplicate rerender audio was removed.
- All five: original-media hashes, retained settings and full exports/excerpts
  verified. Prior rejected balance proposals remain rejected.

Three historical/private-media tests remain intentionally ignored. Old exhaustive
searches, audition matrices and snare-identifiability experiments were skipped.
This pass's bounded production measurements, fixed full verification, selected
render and complete listening preparation ran explicitly. No hardware test or
listener acceptance is claimed.

At the user's subsequent request, superseded generated processed exports and clips
are retired after the replacement set passes verification. Raw/SOURCE mixes and
their paths are preserved; identical generated raw files can share storage. Exact
settings, measurements, rejection reasons, hashes, source notices, originals and
the initial uncommitted-work snapshot remain. `cleanup-result.json` records each
retirement and actual space recovery. Together with disposable task evidence,
cleanup recovered **4.31 GiB**. About **293 MiB** of current checkpoint
audio and evidence remains; the larger retained artifact store mainly includes
raw/SOURCE mixes and earlier unique diagnostic evidence. Historical audio links may be retired; the
new listening index identifies the retained set and reproducible settings.

Remaining limits: preferred snare tone/level, ambience and room balance, bass body,
vocal timbre and stronger guitar sustain still need listening. The static source
corrections and their profiles remain provisional. Short-window crest protections
can reject EQ changes whose audibility is unresolved; this pass does not replace
them with permissive thresholds. Event identity, musical phrasing and perceptual
masking are not established by these measurements. The selected output is ready
for that listening decision, not declared accepted.
