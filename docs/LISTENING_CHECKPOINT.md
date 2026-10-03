# Complete local listening checkpoint

**Current listening set, 2026-10-03:**
`artifacts/automix/summing-study/2026-10-03-engine/LISTEN.md` contains six new complete
mixes with the selected GigPies effects, independent true-peak verification and
seven exact excerpts. See [summing and delivery](SUMMING_DELIVERY.md).
Original recordings and prior retained evidence remain unchanged. Earlier generated
audio was retired; the following indexes and results are historical.

**Earlier FX set:** `artifacts/automix/expert-fx-v1/LISTEN.md` added expert-selected
GigPies effects to all six accepted direct mixes. See [artistic FX](ARTISTIC_FX.md).
Full exports, source identity, generated tails and exact clips are verified;
listener acceptance remains pending.

**Previous direct-sound set:** `artifacts/automix/source-preservation-v1/LISTEN.md` selects the
unchanged SOURCE for Dark Ride and retains the other five FINALs. See the
[source-preservation review](SOURCE_PRESERVATION.md). The checkpoint below is
historical; its source provenance and verification methods remain in use.

**Earlier reassessment:** [independent workflow reassessment](COMPLAINIACS_WORKFLOW_REVIEW.md).
Its historical private index is
`artifacts/automix/complainiacs-reassessment-v2/LISTEN.md`. It contains all six
SOURCE → selected FINAL pairs, with no intermediates. The original checkpoint and
subsequent history below are retained for provenance; earlier rhythmic preferences
are withdrawn and superseded processed audio is retired after replacement.

Prepared 2026-10-02. All six established examples have full SOURCE and FINAL MIX
exports and one synchronized 12-second pair. Nothing was played or published.
The private index is `artifacts/automix/listening-checkpoint-v1/LISTEN.md`.
It links the files, settings, routing, exact clip verification and export meters.

## Bounded selection

The local catalogue and reviewed experiment inventory establish six songs, with
122 supplied WAVs including alternates and produced references. Loose library WAVs
are not additional multitrack examples. Original media and notices stay in
`../waves/recordings/`.

| Example | Selected / supplied | Final selection | Remaining limitation |
|---|---:|---|---|
| The Complainiacs — Etc | 12 / 13 | Preserve guitar correction, DI bass correction and the +2 dB kick/snare rhythmic candidate | Confirmed bleed remains; rhythmic and bass acceptance unknown |
| Dark Ride — Hammer Down | 39 / 40 | First complete render: prior mapped processing plus both complementary bass parts with neutral bass strips | Bass capture type unknown; guitar capture/overdub relationships and preferred balance unresolved |
| Liz Nelson — Rainfall | 5 RAW / 11 | Retain manufacturer-study baseline | Guitar tone eligibility insufficient; applicable vocal/guitar target passed |
| Wild & Co — Run Crawl Walk | 16 / 25 | Retain baseline and prior backing-vocal threshold adaptations | Static balance proposal remains rejected; guitar/kit, vocal/guitar and kick/overhead conflicts persist |
| Catbite — Bad Influence | 13 / 23 | Retain baseline; guitar pilot found no supported change | Close-drum/band and bass/guitar deviations persist; static fader proposal remains rejected |
| Phoenix — Scotch Morris dataset ID | 3 RAW / 10 | Full-song guitar correction: −3 dB at 220 Hz, +2 dB at 1600 Hz, Q 0.7 | Trial target slightly unmet in training; no flute/violin ensemble policy or listener acceptance |

Phoenix's supplied metadata titles the performance *Lark On The Strand – Drummond
Castle*. The established Scotch Morris identifier remains in paths.

The priorities were completing Dark Ride's missing bass coverage, testing the
promising Phoenix pilot across the full source, and preserving earlier progress.
Wild & Co and Catbite have substantial balance deviations, but previous static
changes failed section protections. Those failures remain in force. Spectral
occupancy alone did not trigger another correction. Snare-separation research was
not repeated, and unsupported cleanup remains unapplied.

The candidate budget was one Phoenix source bundle, selected by the existing
bounded grid and checked once through production DSP, plus one conservative Dark
Ride routing baseline. Even origin-aligned 12-second sections selected Phoenix's
settings; odd sections validated them. There was no held-out retry or new artistic
fader search. The other four examples retain their latest defensible exports.

## Dark Ride routing resolution

Both bass files have BWF reference zero. Their substantial activity is mostly
complementary: Bass 2 covers passages around 27–48 s and 98–109 s where Bass 1 is
largely absent. They have only about 0.176 seconds of simultaneous samples above
−80 dBFS during their shared timeline. This supports retaining both supplied
musical parts without guessing that either is a DI or amp recording.

Both retain centre pan, unity trim/fader, zero channel HPF and bypassed EQ and
compression. Their technical engine role is `other`, with their known bass-family
identity and unknown capture recorded separately in the local inventory. The
specialized DI-only analyzers remain unchanged and were not used on these inputs.
No DI protection was weakened or unknown channel silently dropped.

All 37 previously selected inputs retain their pan and mapped processing. The
supplied SnareFX stays excluded under the established routing. All guitar files
remain present; unknown microphone/performance relationships do not authorize
deleting parts, polarity changes or new stereo assignments. Unequal tails are
zero-padded on the supplied timeline. Full-song kick maximum compressor reduction
is 10.56 dB; that remains a listening concern, not proof of an implementation defect.

## Phoenix source and ensemble result

The full-song pass starts from the manufacturer-study baseline, not the short
pilot's modified settings. Existing source protections accept the new two-band
curve at fixed faders, makeup, dynamics and routing. Held-out guitar body/presence
falls **7.78 → 3.92 dB** and median crest rises **11.78 → 12.31 dB**. Training ends
at **4.147 dB**, slightly above the experimental +4 dB upper bound. No limit changed.

Actual production float-bus review covers all 15 ensemble sections, including the
ending. Section median RMS changes −0.623…−0.284 dB; section p10 crest changes
−0.179…+0.333 dB. This descriptive review supplements the instrument guards; it
does not certify preferred flute/violin balance. The export gain increases 0.227 dB
relative to the previous final, raising unchanged flute/violin contributions by
that amount. No musical fader move is combined with the source correction.

Both unidentified Main System sides remain excluded together as in the established
baseline. Metadata verifies the three retained RAW instruments. Sample-zero timing
is preserved; this session has no BWF reference. No capture identity or acoustic
alignment is invented.

## Comparison and export contract

SOURCE uses the selected original multitracks through the **initial documented
pan and faders**, with unity trims/master and no EQ, HPF, compression or effects.
It does not inherit later Complainiacs artistic faders. Dark Ride adds both bass
parts to its existing source routing. Mono pan is constant-power; centred stereo
files retain L/R unchanged. No resampling, polarity changes or silence trimming.

FINAL preserves required channel HPFs, the processed 40 Hz master HPF, stereo
relationships and independently editable makeup/faders. Each full export is
independently finalized to −0.01 dBFS **sample peak**. No source normalization,
equal-LUFS target, matched listening copies or true-peak claim applies.

| Example | SOURCE export gain | FINAL export gain | Excerpt |
|---|---:|---:|---|
| Complainiacs | −7.057 dB | −8.037 dB | 24–36 s |
| Dark Ride | −2.858 dB | −6.279 dB | 48–60 s |
| Rainfall | +4.706 dB | +5.405 dB | 240–252 s |
| Wild & Co | −3.740 dB | −5.335 dB | 168–180 s |
| Catbite | +1.352 dB | −2.878 dB | 156–168 s |
| Phoenix | +8.641 dB | +9.054 dB | 36–48 s |

These gains multiply every contribution equally; they preserve internal balance
and crest but affect the absolute listening level. Complainiacs keeps the previously
documented rhythmic cost: compared with the bass-corrected final, unchanged parts
fall 1.682 dB while kick/snare rise only 0.318 dB after peak finalization.

The private index reports measured final PCM RMS and FFmpeg integrated loudness.
Renderer `processed_lufs` and `bypass_lufs` are pre-export bus measurements and are
labelled separately. Excerpts retain their parent's gain, format and exact sample
position. Full WAV links point to retained exports, avoiding redundant audio copies.

## Reusable preparation and validation

`scripts/listening_checkpoint.py` prepares the set from an explicit inventory and
two frozen render directories per example. It checks completeness, unmatched mode,
unity trims, preserved routing/pan/BWF timeline, full PCM duration/peak, excerpt
bounds and every copied PCM byte. Settings and parent hashes accompany the set.
A completion manifest is written only after all examples pass. It cannot select
processing or start playback. A changed parent, mismatched timeline, truncated
file or partial inventory fails preparation.

The current specification additionally requires `source_settings_sha256` and
`source_wav_sha256`, pinned from the established initial SOURCE comparison before
preparation. This catches later faders substituted into otherwise identical routing.
All parents and metadata are rechecked immediately before the completion manifest.
For the current schema, use the replacement checkpoint's
`checkpoint-specification.json`; the older specification below records its historical run.

```sh
python3 scripts/listening_checkpoint.py \
  artifacts/automix/listening-checkpoint-v1/specification.json NEW_DIRECTORY
CARGO_INCREMENTAL=0 cargo test --locked --all-targets
python3 -W error::ResourceWarning -m unittest discover -s scripts -p 'test_*.py'
```

Validation: **102 Rust and 20 Python tests pass**, plus formatting, Clippy with
warnings denied and the locked release build. Four new fast regressions exercise
inventory, exact alignment/gain, bad peaks, changed pan/timeline, truncation and
non-overwrite. Preparation exposed an unclosed hash reader in the existing excerpt
meter; it now closes explicitly. No Rust DSP or safety-policy change was needed.

Three historical private-media tests remain intentionally ignored. The current
private full renders and complete listening verification ran explicitly; old
auditions, exhaustive matrices and snare-identifiability experiments were skipped.
All original-media hashes and prior uncommitted work were preserved. The new set
retains about 677 MiB of useful renders, float buses, clips and concise evidence;
no disposable source copies or scratch environments were created.

This is implemented preparation and offline validation. Listener acceptance is
pending except for previously recorded Complainiacs guitar feedback. No playback,
host-audio change, hardware verification, commit or push occurred. A subsequent
playback request must preflight every requested clip before playing each once in
the agreed order with one-second gaps and no preparation between clips.

## Subsequent listening feedback

The six-example playback log subsequently records completion. The listener preferred
SOURCE in some Complainiacs passages and described FINAL's voice as harsh/forward,
guitars less prominent and the mix as trebly/lacking body. That feedback supersedes
any inference of preferred balance from this checkpoint's technical success.
[The local reassessment](COMPLAINIACS_REASSESSMENT.md) supplies a new Complainiacs
FINAL and verified SOURCE → CURRENT → REVISED excerpts. Other examples remain intact.
