# Expert selection of artistic effects

The listener clarified that the requested deliverables are finished musical mixes,
including GigPies-generated effects selected by the expert system. The previous
six-example checkpoint had no GigPies FX rack. Calling it complete described its
duration and export verification; it did not meet this artistic requirement.
The [FX provenance audit](FX_PASS.md#where-the-audible-ambience-comes-from) records
which older versions did contain generated effects.

Source preservation and spatial production are separate decisions. A good recorded
tone can remain intact while an explicit artistic request justifies added space.
An effect needs an intended musical benefit, not a fabricated technical defect.
This pass adds parallel effects to the selected channel tone and balance. It does
not restore the rejected Dark Ride presence EQ/compression/makeup chain.

## Implemented workflow

```sh
CARGO_INCREMENTAL=0 cargo +1.97.1 build --release --locked
target/release/gigpies ambience-plan BASELINE.json SOURCES NEW_PLAN POLICY.json
target/release/gigpies render NEW_PLAN/settings.json SOURCES NEW_RENDER
```

`ambience-plan` is an explicit artistic step, separate from corrective `source-pass`
and the historical fixed `fx-preset`. It writes no audio. The production renderer
uses its frozen settings to make the complete mix. It creates `settings.json` only
after training and held-out checks pass; failed plans retain evidence and the
baseline, without publishing a ready selection. Use fresh output directories.

Policy supplies:

- A musical style and its basis; optional **verified** BPM. No genre/tempo detector
  runs. Without BPM, synchronized echoes are withheld.
- Named source groups with exact channel/file assignments and identity provenance.
  Every retained channel needs a disposition; grouping compatible FX sends does
  not assert one performance, source separation or microphone identity.
- Explicit reports of existing space, when available. This flag can temper added
  space; a waveform or filename never sets it automatically.
- An FX amount from zero to one, and separate frozen training/held-out passages.

The program owns the effect choices and parameters after those inputs are supplied:

1. Measure the accepted direct signal using production channel DSP, timing, faders,
   pan and master HPF. Retain 20 ms energy windows for the training passages only.
   Activity and energy-rise observations are density proxies, not note transcription
   or an “already reverberant” diagnosis.
2. Select a spatial recipe from the instrument family and requested style. Busy
   signals get shorter, quieter space. Reported existing ambience also reduces the
   proposed decay and amount. Insufficient activity withholds a bus.
3. Convert the requested decay duration to **our engine's actual control**. The
   engine uses a unitless feedback parameter. Six bounded bisection steps compare
   filtered production-rack impulse decays; the report retains desired and measured
   duration. The measurement is remaining-energy decay to −60 dB after predelay
   within a six-second impulse capture, not a measured physical room RT60.
4. Run the proposed rack continuously from sample zero. Measure each wet return
   against its coherent feeding group during active training windows. Choose a
   bounded return gain for the recipe's explicit artistic amount. This calibrates
   the engine transfer level; it changes no input trim, compressor makeup or fader.
5. Freeze the resulting settings, then check the combined ensemble in training and
   held-out passages. Held-out failure vetoes the proposal and cannot trigger a
   parameter retry. Passing checks makes an audition eligible; it does not prove
   musical improvement.

The first catalogue has **ten style choices**: metal, rock, punk, ska, pop, acoustic,
folk, jazz, electronic and ambient. These are independently chosen musical starting
hypotheses, not a validated library covering every subgenre or performer.

| Source family | Initial spatial direction |
|---|---|
| Snare | Plate for amplified styles; chamber for acoustic/folk/jazz |
| Toms/percussion | Filtered chamber, restrained tail |
| Lead vocal | Plate for tight/amplified styles; hall for acoustic or spacious treatments; separate vocal predelay |
| Backing vocal | Chamber for shared depth |
| Rhythm guitar | Short room; plate for the ska profile |
| Lead guitar | Plate; restrained tempo echo when style and verified BPM support it |
| Acoustic guitar | Chamber |
| Keys | Chamber or ambient hall; subtle chorus for pop/electronic profiles |
| Winds/strings | Chamber for natural ensemble styles; hall otherwise |
| Kick/bass | Retain direct low-end support; no default spatial bus |
| Recorded ambience/unknown | Preserve the supplied path; no speculative additional send |
| Supplied FX return | Refuse it as a mix input under this workflow |

The exact current recipe values and their adaptations live in
`src/automix/ambience.rs`, with every selected value saved in the plan. The catalogue
can grow from instrument/genre listening evidence. Automatic compression, excitation,
master EQ or loudness maximization are not bundled into a spatial request.

The broad effect vocabulary follows established practice: the engine developer's
[reverb-type discussion](https://valhalladsp.com/2018/05/14/effect-o-pedia-reverb-types/)
describes differing spaces and warns that long tails blur fast notes. Our recipes
are hypotheses for our own engines; no third-party preset collection was imported
and no emulation equivalence is claimed.

## Preservation, protection and provenance

- Preserve files, source offsets, sample rates, stereo routing, artistic faders,
  channel EQ/compression/makeup and the accepted master HPF. Zero amount preserves
  the complete baseline settings; unknown identity can also yield no new bus.
- Only GigPies' reverb/chorus/delay engines generate new returns. Do not substitute
  supplied FX tracks, processed stems or finished reference mixes. Existing acoustic
  room captures are source recordings; printed effects inseparable from an instrument
  remain a stated limitation. The program cannot certify dry capture or remove such
  processing by inference. Supplied FX can be a later comparison, outside rendering.
- Keep the established vocal reverb minimum predelay of 30 ms. Return gain correction
  is bounded to ±18 dB before the requested amount, within the existing FX schema.
  No channel protection threshold is widened.
- The combined proposal must keep wet ensemble RMS at most −12 dB relative to the
  direct ensemble, RMS and peak rises at most 2 dB, and crest loss at most 3 dB in
  each evaluated passage. These are explicit experiment guards, not preferred mix
  targets or evidence of acoustic safety. Silent references cannot set a wet ratio.
- FX rack master drive is zero, master EQ/excitation are absent, and its existing
  high-headroom limiter threshold is 24 dBFS on the floating bus. Require zero
  reduction in complete-render verification; final sample-peak export is separate.
- Render six seconds of effect tail and inspect the ending. Each full export is
  independently finalized to −0.01 dBFS sample peak. No source normalization,
  equal-LUFS target or true-peak compliance claim applies.

The checkpoint writer now accepts a longer FINAL only with explicit `allow_fx_tail`,
matching source routing/offsets/lengths, consistent per-channel padding and production
FX metadata matching the added frames. The established SOURCE file/hash stays pinned;
no silent padding or gain change is applied to that reference. Every excerpt still
matches its parent's exact samples, format, position and gain.

## Current six-example experiment

The frozen plan is `artifacts/automix/expert-fx-v1/plan.json`. One rule-generated
proposal and one training-only return calibration per song are allowed, followed
by combined validation; no held-out retries. It starts from the source-preservation
checkpoint's selected direct mixes: unchanged SOURCE for Dark Ride and the retained
five channel/balance selections. Styles and source assignments are explicit in each
song's `policy.json`. No imported effect returns are added. Dark Ride's two bass
parts retain unknown capture identity and their `other` channel roles.

**All six complete renders passed technical verification.** The normal suite has
117 passing Rust tests and 30 passing Python tests, with three historical media
checks intentionally ignored. Clippy with warnings denied, formatting and the locked
Rust 1.97.1 release build pass. Unrelated exhaustive auditions were skipped.

Regressions cover actual rack/renderer agreement, preserved channel settings, exact
zero amount, unknown-source abstention, style/tempo choices, held-out isolation,
invalid identity/FX-return refusal, ensemble overload rejection and checkpoint tail
contracts. Full media checks verify original recording hashes, unchanged direct
settings, exact bypass PCM/float with a zero-only extension, independent export
peaks, zero master reduction and endings below −90 dBFS over the final 250 ms.
The catalogue's musical acceptance remains pending.

| Example | FX buses | FINAL export gain | Gain change from previous FINAL | Finished LUFS |
|---|---:|---:|---:|---:|

| complainiacs | 4 | -6.305838 dB | -0.099495 dB | -15.6 |
| dark-ride | 8 | -2.980157 dB | -0.122513 dB | -14.8 |
| rainfall | 4 | +5.377307 dB | -0.027205 dB | -16.9 |
| wild-co | 7 | -5.312877 dB | +0.022441 dB | -16.0 |
| catbite | 6 | -2.888254 dB | -0.010514 dB | -17.0 |
| phoenix | 3 | +8.987501 dB | -0.066600 dB | -16.1 |

These gain changes follow the finished mix's sample peak. They are not source
normalization or compensation for a taste target. Dark Ride preserves the source
channel power/body and adds eight FX buses; the new export requires about 0.12 dB
more attenuation than SOURCE. It retains roughly 3.30 dB more export gain than the
older rejected presence-heavy FINAL.

No proposal required a held-out retry or widened guard. All six new settings contain
our generated returns. Supplied room/capture paths remain part of the direct routing;
printed effects are not removed or newly certified. Existing source contacts and
unknown Dark Ride bass capture identity remain limitations. Listen for clarity,
distance, drum tail length, guitar articulation, low-end support and whether the
combined supplied and generated ambience is appropriate. Technical checks do not
resolve those preferences.

The complete verified index is `artifacts/automix/expert-fx-v1/LISTEN.md`; it contains
only the established SOURCE → selected finished FINAL pairs, with the existing
second Complainiacs passage. Full FINALs include six seconds of generated tail;
SOURCE files and their clip samples remain unchanged. Original recordings and
previous direct selections are retained for provenance and recovery.

After dependency/open-file checks, cleanup removed only the new duplicate bypass
exports (verified identical to retained prior files plus zero tails) and compressed
measurement histories losslessly. It recovered **1.104 GiB**. Current deliverables
and evidence retain about **1.1 GiB**; the normal build output is **761 MiB**, with
about **42 GiB** free. Existing uncommitted work was snapshotted and preserved.
No commit, push or host-audio configuration change was made. Dark Ride playback
was explicitly requested separately; it does not establish listener acceptance.
