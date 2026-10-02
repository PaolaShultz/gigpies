# Local multitrack material

Found in the development workspace's `../shr-lux/recordings/downloads/`:

| Archive | WAV files | Original format | Longest file |
|---|---:|---|---:|
| `Complainiacs_Etc_Full.zip` | 13 | 24-bit PCM, 44.1 kHz | 114.242 s |
| `DarkRide_HammerDown_Full.zip` | 40 | 24-bit PCM, 44.1 kHz | 300.365 s |

Counts and formats were read from local ZIP entries and WAV headers. A stereo WAV
counts as one track file but contains two audio channels. Different end lengths are
present; duration alone does not prove sample alignment or correct placement.

The source archive readmes identify the material as educational-use recordings and
require copyright-holder permission for commercial use. Retain those readmes with
local files. Source library: Mike Senior's Cambridge multitrack library, as recorded
in the existing SHR Lux provenance. This project includes no music or source archives.

## First session: The Complainiacs — Etc

- Kick, snare, stereo overheads, stereo drum room, three toms.
- Bass DI and bass amp.
- Two electric-guitar files; filenames alone do not establish separate performances.
  The current tone experiment treats them as two known paths for one guitar.
- Lead vocal and vocal room.

Use the original 13 WAVs for mixing. SHR Lux's prepared four-source lighting simulation
contains kick, bass and two guitars at 16-bit/48 kHz with fixed attenuation; it is not
a complete-band mixing source.

Local preparation locations:

- `recordings/downloads/`: optional cached original archives.
- `recordings/sessions/complainiacs-etc/`: original extracted WAVs and source notice.
- `sessions/local/`: inventories and later private experiment manifests.
- `artifacts/`: generated measurements and mix comparisons.

All are ignored. No auto-download runs in builds, tests or CI. Keep original files
unaltered; derived audio goes to a separate destination. Future preparation must
preserve sample zero, stereo relationships and relative levels, and explicitly record
padding or resampling. Verify alignment before interpreting cross-track measurements.

## Dark Ride source and processing audit — 2026-10-02

The listener preferred SOURCE over the current FINAL, describing lost power and a
thinner, more processed sound. This audit checks that observation without selecting
another mix. **The files are authentic supplied multitracks; they are not certified
untreated recordings.** SOURCE means bypassing GigPies processing while retaining
the supplied sounds, initial faders/pan and independent export gain.

### What the source evidence establishes

- All 40 WAVs and the supplied readme match the acquisition hashes and the
  corresponding entries in the original local ZIP. The ZIP also matches its recorded
  SHA-256. No earlier GigPies mix was accidentally used as an input.
- The readme says “Raw multitrack” but gives no channel processing log.
  [Cambridge's library definition](https://cambridge-mt.com/ms2/mtk/) explicitly
  allows treatments printed during tracking/editing. This definition was retrieved
  from indexed primary-source text; direct page retrieval returned HTTP 403.
- In [April 2017 production notes](https://discussion.cambridge-mt.com/showthread.php?tid=18641),
  band producer Blitzzz confirms Slate drum samples and Sylenth1 synth parts.
  His exact snare/kick model recollection is tentative. In
  [January 2023 notes](https://discussion.cambridge-mt.com/showthread.php?mode=threaded&pid=121967&tid=44380),
  he describes playing an electronic kit to trigger samples and deliberately
  shaping a distorted guitar tone. These are produced sounds, so live acoustic
  microphone assumptions are inappropriate for the sampled drum paths.
- The later notes mention a Neural DSP guitar plugin and, elsewhere in the same
  discussion, limiting in his own mix. They do not establish which processors were
  printed into every file of our archive. The supplied WAV headers name REAPER and
  date their exports to April 2017; they contain no processing history. A DAW export
  header alone does not prove compression or EQ. Exact guitar plugin/version,
  printed vocal processing and bass capture identity remain unresolved.
- `03_SnareFX.wav` is supplied but excluded from both listening versions. Its
  exclusion cannot establish that the remaining tracks are dry or untreated.

### What GigPies adds

The current FINAL inherits the manufacturer-reference experiment's role presets.
All 39 retained channels have unity trims and faders and centre pan; stereo files
retain their internal L/R. There was no song-specific artistic fader optimization.
Both unspecified bass parts have neutral channel strips. The processed master has
a 40 Hz HPF; all channels except kick/bass have 90 Hz HPFs. There are no added FX
or active master limiter in this unmatched render.

Each of the 11 guitars receives the same EQ, including +4 dB at 1.9 kHz,
plus +2.5 dB makeup. In the replayed 48–60 s passage, their compressors produce
effectively zero reduction. Across the full song the largest guitar reduction is
0.687 dB. Additional guitar compression is therefore a weak explanation for this
comparison; EQ, makeup and ensemble balance have clear measurable effects.

The kick compressor reaches 9.60 dB reduction in the excerpt and 10.56 dB in the
full song. In the excerpt it reduces the EQ'd kick's RMS by 5.86 dB before the
explicit +5.5 dB makeup. Makeup restores level, not the original envelope.

The table isolates coherent stereo group contributions in the **48–60 s passage**.
Columns show RMS changes at successive production stages. These are descriptive
measurements, not preferred levels or additive shares of total mix energy.

| Group | Channel HPF | EQ | Compression, no makeup | Makeup | Master HPF | Finished contribution vs SOURCE |
|---|---:|---:|---:|---:|---:|---:|
| Kick | 0.00 | +2.09 | −5.86 | +5.50 | −0.40 | −2.09 dB |
| Snare | −0.17 | +1.45 | −2.20 | +3.50 | −0.00 | −0.84 dB |
| Other drums | −1.01 | +0.19 | 0.00 | 0.00 | −0.04 | −4.27 dB |
| Guitars | −0.50 | +1.85 | approximately 0 | +2.50 | −0.02 | +0.40 dB |
| Synths | −0.02 | 0.00 | 0.00 | 0.00 | −0.00 | −3.44 dB |
| Vocals | −0.09 | +2.86 | −1.93 | +2.00 | −0.00 | −0.58 dB |
| Bass | 0.00 | 0.00 | 0.00 | 0.00 | −0.32 | −3.74 dB |

FINAL requires **3.4215 dB more export attenuation** than SOURCE, included in the
last column. Thus unchanged channel faders do not imply unchanged ensemble balance:
guitars rise about 4.15 dB relative to bass in this passage. Unequal EQ, dynamics and
makeup create this relative change; the common export gain determines its absolute
listening level. The 40 Hz master HPF explains only 0.32 dB of the bass RMS loss here.
The synth HPFs remove very little RMS in this passage. Blanket HPFs deserve source
review, but this evidence does not support blaming them for the entire loss.

The actual listening clips measure:

| Meter | SOURCE | FINAL |
|---|---:|---:|
| Sample peak | −2.381 dBFS | −2.370 dBFS |
| RMS | −19.075 dBFS | −20.541 dBFS |
| Integrated loudness | −16.6 LUFS | −17.4 LUFS |
| Crest | 16.694 dB | 18.170 dB |

Similar peaks coexist with lower average energy. The larger crest does not certify
more musical punch; it also rules out describing the entire final mix as simply
flattened by a master compressor. Raised presence and reduced bass/kit support are
plausible causes of the listener's thinner sound, with moderate confidence in that
perceptual attribution and high confidence in the measured changes.

### Consequences and verification

The workflow weakness is retaining broad role processing without establishing a
benefit for these supplied sounds. File integrity, DSP determinism and technical
export compliance cannot establish musical improvement. Sampled drums and designed
instrument sounds need explicit provenance alongside instrument roles. A generic
“raw” label is insufficient. Unprocessed multitracks in the other examples also
require their own evidence; this finding does not establish their printed chains.

For a subsequent mix pass, retain SOURCE as the listener-preferred comparator and
include minimal processing as a legitimate candidate. Evaluate processing benefits
and artistic balance separately, account for export attenuation, and retain makeup
as an independent decision. Removing or reducing mandatory HPFs would be a separate
foundational-contract proposal. No contract, threshold, mix or DSP code changed here.

Private evidence is in `artifacts/automix/dark-ride-source-audit-v1/`:
`summary.json`, `dsp-stages.json`, `excerpt-verification.json` and the retained
diagnostic Rust source. Its production `Strip` and `Biquad` states ran from sample
zero through 60 s. Both reconstructed float buses matched every stored float32
sample across those 60 s. Both excerpts matched their parent's exact PCM bytes
and manifest hashes. No audio was generated or played during this audit.

To repeat the opt-in diagnostic, temporarily copy `darkride_source_audit.rs` from
that evidence directory into `examples/`, then run:

```sh
CARGO_INCREMENTAL=0 cargo +1.97.1 run --release --locked --example darkride_source_audit -- \
  artifacts/automix/listening-checkpoint-v1/dark-render \
  ../waves/recordings/sessions/darkride-hammer-down/DarkRide_HammerDown_Full
```

Remove the temporary example afterward. This is an evidence renderer without audio
output, not a default-suite test. The normal production and historical suites were
not rerun because implementation and settings were unchanged. Earlier suite results
remain recorded in the workflow review. Final listening acceptance remains open;
Dark Ride's current FINAL has an explicit adverse listener observation.

### Subsequent selection

The [bounded preservation reassessment](SOURCE_PRESERVATION.md) selects this exact
Dark Ride SOURCE as FINAL, revises the universal offline HPF contract explicitly,
and retains both unknown bass parts without a DI claim. The archive/source findings
and the prior audit's scope above remain unchanged. The other five mixes are retained.
