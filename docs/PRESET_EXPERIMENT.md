# Manufacturer-reference offline experiment

This experiment separates published parameter references, our DSP mapping, measured
processing changes, and static musical fader changes. Manufacturer-derived means
that numerical starting points have a traceable origin. It does not mean console
emulation, manufacturer endorsement, or a verified improvement in sound.

## Reference and use boundaries

Reviewed 2026-10-01. The [Yamaha 01V96 Version 2 manual](https://data.yamaha.com/files/download/other_assets/5/334235/01v96v2_om_en_f0.pdf)
publishes EQ tables on printed pages 271–272, compressor tables on 273–274, and
compressor semantics on 277. The local inventory records selected original values,
units, filter types, intended instruments, unspecified microphones, pages, URL and
manual hash. It remains under ignored `artifacts/`, along with our separate mapping
and adaptation decisions. The repository contains no copied manufacturer collection.

[Yamaha's terms](https://www.yamaha.com/en/terms-of-use/) provide for personal,
non-commercial website use and restrict commercial/public use of its materials.
That supports the scope of this private experiment; it is not an open licence for
manuals or preset collections. The [US Copyright Office](https://www.copyright.gov/help/faq/faq-general.html)
distinguishes facts and methods from protected expression. The
[EU software directive](https://eur-lex.europa.eu/legal-content/en/TXT/?uri=CELEX%3A32009L0024)
also distinguishes underlying ideas/principles from program expression. These
principles support studying parameter values to develop independently implemented
curves; neither establishes blanket permission for every collection or use.
The [EU database directive](https://eur-lex.europa.eu/legal-content/EN/ALL/?uri=celex%3A31996L0009)
adds separate extraction/reuse rights and nationally implemented exceptions.
Applicability, contractual enforceability, and any future commercial distribution
remain unresolved. Private research is not a universal exemption. Copying executable
software, firmware, manuals or a preset library is a different activity from testing
selected documented numbers in independently written DSP.

The [official Allen & Heath Qu resources](https://www.allen-heath.com/hardware/qu/qu-classic/qu-16/resources/)
currently list an Audio Technica ProFactory download. The download link failed in
this review. The page's EULA also restricts adaptation, decoding and competing-product
analysis; its applicability to an individual preset package is not clarified by
public availability. No A&H binary library was decoded or applied. Earlier notes
about Shure/Sennheiser preset names do not supply verified numerical settings.
No firmware extraction, protected access, or third-party replacement library is used.

## Source applicability

| Session | Selected / supplied WAVs | Assessment |
|---|---:|---|
| Complainiacs — Etc | 12 / 13 | DI selected; amp excluded; legacy pan preserved |
| Wild & Co — Run Crawl Walk | 16 / 25 | DI, one path per guitar, stereo overheads, keys/arpeggiators and vocals; alternate microphones excluded |
| Catbite — Bad Influence | 13 / 23 | DI, one guitar amp microphone, close kit, stereo overheads/room, keys and vocals |
| Liz Nelson — Rainfall | 5 RAW / 11 total | RAW only; five processed stems and produced mix excluded from inputs; no bass source |
| Phoenix — Scotch Morris | 3 RAW / 10 total | Direct guitar/flute/violin; two undocumented Main System sides withheld together; no bass source |
| Dark Ride — Hammer Down | 37 / 40 | Short assessment only; two unspecified bass sources and processed snare FX withheld; no verified DI, so full-band rendering deferred |

Every supplied WAV has a local header/hash record and a routing disposition.
Selected BWF tracks share a start reference within each song; Complainiacs retains
its unequal tails. Phoenix has no BWF reference, so supplied sample-zero alignment
is assumed. Dark Ride has many unequal tails, overdubs and an unresolved bass-source
identity. Timing metadata does not establish acoustic phase or absence of editing.
TELEFUNKEN auxiliary microphone choices are explicitly editorial, not quality grades.
All source quality remains ungraded. Prior acquisition hashes and fresh before/after
hashes distinguish verified file integrity from musical readiness.

## DSP mapping

- Legacy JSON EQ bands remain bells when `kind` is absent. Bells now accept Q down
  to 0.1, needed for the broad published bands. Q is used as RBJ peaking Q; console
  EQ type, gain-dependent bandwidth and transfer-function equivalence are unverified.
- `low_shelf` and `high_shelf` use our monotonic RBJ S=1 curve, with `q` explicitly
  fixed to `1/sqrt(2)`. Other shelf Q values are rejected. The manufacturer does not
  publish its shelf slope in these tables, so this choice is logged as an adaptation.
  Frequency and gain come from the reference. Equations follow the
  [W3C Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/).
- Compressor ratio, threshold, attack, release number and output gain are traced
  separately. Our detector takes the larger absolute stereo sample and smooths
  gain reduction exponentially in dB. The console detector and attack law are not
  sufficiently specified to claim equivalence.
- Yamaha describes release as a time for a 6 dB change; ours is an exponential
  time constant. Keeping the printed millisecond number is an explicit experimental
  adaptation, including at 48/96 kHz; it does not reproduce console clock scaling.
- Ordinal soft-knee settings have no verified width in dB. The mapped experiment
  explicitly uses a hard knee and retains the unsupported ordinal in its audit.
  The sharper threshold transition is an audition tradeoff. No invented width is called official. Expanders, companders and unsupported
  filters remain recorded and bypassed, without substitution.
- Manufacturer output gain becomes explicit makeup once. Subsequent adaptation
  and fader search do not add compressor-loss compensation.

`acoustic_guitar`, `backing_vocal` and `other` roles permit accurate source labels
without pretending that keys, flute or violin are guitars. Acoustic guitars enter
existing guitar body measurements and fader grouping. Backing vocals receive phrase
meters but remain outside the lead-vocal optimizer. Other instruments retain their
static faders and HPFs; no unsupported balance model is inferred for them.

## Local layout and reproducibility

The current private run is `artifacts/automix/manufacturer-study-v1/`:

- `parameter-inventory.json`: exact selected reference settings and provenance.
- `plan.json`, per-song `routing-inventory.json`: actual file/header/hash inventories,
  selected and excluded sources, references, bass-DI availability and uncertainty.
- Per-song `neutral.json`, `derived.json`, `mapping.json`, `assignments.json`:
  independently editable musical faders, original-reference assignments and mappings.
- `pilot*`: short native-rate evidence preceding complete-song rendering.
- `neutral/`, `derived/`, `adapted/`: processing experiments with identical faders.
- `adaptation-decisions.json`: training evidence, old/new thresholds and tradeoffs.
- `faders/`: bounded fader-only search from adapted processing, its held-out checks,
  decisions and final render. No produced reference enters any analysis or search.

Run from the repository root after supplying reviewed local configurations and
inventory. Every output destination must be new:

```sh
CARGO_INCREMENTAL=0 cargo build --release --locked
python3 scripts/preset_experiment.py map neutral.json parameter-inventory.json assignments.json new-mapping
target/release/gigpies render new-mapping/settings.json source-dir new-derived-render
target/release/gigpies balance-analyze new-mapping/settings.json source-dir new-derived-analysis
python3 scripts/preset_experiment.py adapt new-mapping/settings.json new-derived-analysis/before/windows.csv new-adaptation
target/release/gigpies render new-adaptation/settings.json source-dir new-adapted-render
target/release/gigpies balance-pass new-adaptation/settings.json source-dir new-fader-pass
python3 scripts/preset_experiment.py excerpts source.wav ours.wav new-excerpts 36 72
```

The adaptation is deliberately limited: one threshold increase of at most 6 dB,
only if at least eight high-confidence active training windows show p90 maximum
reduction above 6 dB. It cannot turn quiet playing or bleed-only pauses into upward
source normalization. All odd 12-second sections are held out. The 6 dB action
budget is our hypothesis, not a manufacturer setting or listener preference.
EQ and makeup remain fixed during this step. The fader search retains the existing
512-candidate budget per stage, ±6 dB bounds and held-out veto. There is no retry
against held-out results. Processing comparisons are retained even when unchanged.

The synchronized 20 ms windows, event/phrase compressor envelopes, guitar body
bands, microphone covariance and bleed-confidence limits are described in
[BALANCE_PASS.md](BALANCE_PASS.md). Activity statistics use the complete recording;
training/held-out separation applies to threshold and fader decisions, not to an
independent-session validation claim. Empty balance policies are not reported as
having met all targets.

`review` compares candidate measurements on identical neutral-selected windows:

```sh
python3 scripts/preset_experiment.py review neutral-analysis/before/windows.csv candidate-analysis/before/windows.csv new-review.json --rate 44100
```

Candidate activity cannot select its own evaluation windows. The local report also
retains hit/phrase attack, body and decay gain-reduction summaries. Reference preset
names describe their published intended application; they do not establish a singer's
identity, voice classification, microphone model or the best processing for a track. In particular, the published
cymbal curve on overheads can affect the entire captured kit and bleed; the bass
finger-style and acoustic stroke labels do not verify the recorded playing style.

## Routing and output contracts

Input trims are unity. Processed channels have 90 Hz HPFs except kick/bass; the
processed master has a 40 Hz HPF. No master notches or channel normalization are
added. This experiment disables FX across all processing variants to isolate EQ
and compression; the existing vocal-reverb minimum of 30 ms remains enforced.
Complainiacs retains its existing pan positions. New sessions have explicit editorial
pans; interleaved stereo stays centred and linked. BWF offsets and source tails are
preserved; Phoenix assumes the supplied common sample-zero timeline. No acoustic
alignment, polarity changes, resampling or independent silence trimming occurs.

SOURCE is the selected original-source sum through the documented initial faders
and pan, with no channel processing or master HPF. OUR MIX is the separately
processed/fader candidate. Both complete PCM exports are independently finalized
to −0.01 dBFS sample peak; the unbounded float sums are retained. No equal-LUFS
target, loudness matching or true-peak claim applies. Exact 12-second PCM excerpts
retain the full-song gain. Produced references retain their supplied gain and format;
identical timeline timestamps do not establish sample alignment to production edits.

## Validation

Normal synthetic regressions cover shelf endpoints/midpoints and native rates,
broad bells, legacy JSON, unsupported shapes, transient preservation, stereo-linked
compression and single makeup application. Existing balance regressions cover
bleed-only pauses, quiet playing, silence, insufficient confidence, correlated
microphones, excessive room contribution, fixed budgets and held-out vetoes.
Python tests cover training-only threshold decisions, unsupported-parameter retention,
assignment identity, excerpt byte integrity and refusal to overwrite.

```sh
CARGO_INCREMENTAL=0 cargo test --locked --all-targets
python3 -m unittest discover -s scripts -p 'test_*.py'
```

Private-media pilots, complete-song renders, source hashes and excerpt checks are
explicit opt-in work. Historical auditions remain ignored. No test establishes
musical preference, physical verification, or that the agent heard the audio.

## Recorded run: 2026-10-01

Six 24-second pilots passed the PCM peak checks. Five sessions then received
complete neutral, manufacturer-derived, adapted and final fader-stage renders.
All 40 complete PCM outputs were checked for the expected frame counts, native
sample rates and −0.01 dBFS sample peaks. All source hashes remained unchanged;
109 files also matched their earlier acquisition hashes. The 13 Complainiacs files
were covered by fresh before/after hashes. No playback occurred.

| Session | 12-second excerpt starts | Fader proposal | Balance policy |
|---|---|---|---|
| Complainiacs | 24 s, 60 s | Accepted | Five held-out relationships remain outside targets |
| Wild & Co | 168 s, 276 s | Rejected; initial faders retained | Targets remain unmet |
| Catbite | 156 s, 168 s | Rejected; initial faders retained | Targets remain unmet |
| Rainfall | 240 s, 252 s | No accepted change | Applicable target met |
| Phoenix | 36 s, 144 s | No change | No applicable ensemble policy; missing coverage |

The local `LISTEN.md` links SOURCE → OUR MIX, processing-only comparisons at fixed
faders, and separate OUR MIX → PRODUCED REFERENCE pairs for both MedleyDB songs.
Sections were selected from confident neutral-source activity, one training and
one held-out section per song. Byte-identical processing variants are omitted from
the listening sequence. Earlier rejected mixes are not included.

The one-step adaptation raised the Complainiacs kick threshold by about 0.57 dB
and two Wild & Co backing-vocal thresholds by about 0.40 and 2.54 dB. No other
threshold changes were supported. The subsequent check still found p90 window
maximum reduction around 6.09 dB in Complainiacs training windows and 6.15 dB in
held-out windows, plus about 6.01/6.05 dB for those Wild & Co training sources.
These remain budget failures; the experiment did not retry against held-out data.
Catbite, Rainfall and Phoenix needed no compression adaptation under this policy.

Complainiacs' adapted median attack/body/decay reduction was approximately
3.39/4.49/2.11 dB for kick, 1.09/0.66/0.28 dB for snare, and
1.47/2.23/2.70 dB for vocal phrase starts. These event-proxy measurements exclude
makeup. Fixed neutral-selected held-out windows showed guitar body energy rising
about 2.65/3.36 dB with derived processing; this includes EQ, compression and output
gain together and does not establish that the guitars sound better.

A separate raw covariance check of Phoenix's Main System pair found frequent
negative zero-lag correlation. That does not identify channel sides, prove wrong
polarity, or justify a flip. Both sources remain excluded together and preserved.

Validation: 41 normal Rust tests and four Python tests passed, along with formatting,
check, clippy and release build. Three historical private-media tests were deliberately
skipped; current pilots, full renders and excerpt checks were explicitly run.
There is no hardware verification or listener verdict.

The retained experiment occupies about 3.9 GiB. Removing task-created pilot source
copies recovered 417 MiB; consolidating 60 byte-identical WAV copies recovered
another 3.60 GiB. Every render path and byte sequence remains available. Identical
new evidence files share hard links, so create a new output or copy before editing
one in place. Originals and prior experiments were not changed.
