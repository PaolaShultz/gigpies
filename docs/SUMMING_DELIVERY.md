# Offline summing observations and delivery

This work follows [the execution plan](SUMMING_PLAN.md). Musical preference and
hardware acceptance are separate from file verification. Frozen version 1 session
settings retain their identity and the existing `render` command retains its legacy
sample-peak and comparison behavior.

## Commands

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
# Writes a new explicit sidecar; does not change the prepared session.
target/release/gigpies delivery-policy NEW_POLICY.json
target/release/gigpies render-policy PREPARED.json SOURCES NEW_RENDER NEW_POLICY.json
# Reuse an existing frozen render's retained float bus, with identical enabled DSP:
target/release/gigpies delivery-finalize LEGACY_RENDER SOURCES NEW_DELIVERY NEW_POLICY.json
# Verify identities, current sources, declared static gain, PCM and measured peaks:
target/release/gigpies delivery-check NEW_DELIVERY SOURCES
# Independent final PCM check; requires locally installed FFmpeg with libsoxr:
python3 scripts/verify_delivery.py NEW_DELIVERY
# Read-only measurement, or production stage observations without audio exports:
target/release/gigpies peak-measure STEREO.wav NEW_MEASUREMENT.json
target/release/gigpies observe-stages PREPARED.json SOURCES NEW_OBSERVATION END_SECONDS
```

Every destination must be new. Observation replays continuously from sample zero.
It stops at the requested end or the original timeline plus configured FX tail.
It does not run a planner, change settings or write listening audio.

## Independent controls

The sidecar has version 1 and rejects unknown fields and unsupported formats:

- `final_sample_limiter`: `disabled`, or `enabled` with `threshold_dbfs` and
  `release_ms`. This processor is distinct from the frozen FX rack master.
- `delivery_gain`: `independent_peak` with an explicit `max_boost_db`, or `fixed`
  with `gain_db`. Fixed gain is refused if it lacks the reserved headroom.
- `peak_basis`: `sample` or `true`; `ceiling_db` uses dBFS or dBTP accordingly.
- `comparison`: `none`, or `loudness` with `target_lufs`. Optional matched copies
  reuse the same processed and bypass buses. They do not alter primary delivery
  or select DSP. No real-audio loudness-matched copies were authorized for this run.
- Named `meter`, `sample_format: "pcm24"`, and `dither: "none"`. Integer conversion
  rounds ties away from zero once. There is no per-channel dither or normalization.

The default policy uses disabled final limiting, independent static peak delivery,
−1 dBTP, a +12 dB maximum boost and no comparison copies. Digital silence gets zero
gain. FX master EQ/drive/limiting stays as declared in the session and its action is
reported separately. Changing the ceiling does not change those processors.

Legacy `output_mode` and `ceiling_db` remain authoritative for `render` and the
existing review/planning/checkpoint contracts. Policy-aware rendering overrides
only final limiter and export choices explicitly declared in its sidecar. It does
not rewrite the session or migrate saved EQ/FX state. `delivery-finalize` refuses
a request to change DSP after rendering. A reused legacy bus has the provenance
of its retained render; the study additionally checks its historical hash.

## Meter and uncertainty

`gigpies-blackman-sinc64-8x-min192k-v1` uses a 64-tap Blackman-windowed sinc per
fractional phase, with unity DC gain. It measures both stereo sides independently,
includes sample peaks and zero extension at both boundaries, and uses at least
8× interpolation and 192 kHz. Rate-dependent factor classes are 8, 16 and 32 for
the supported integer 8–192 kHz range. FIR state persists across blocks. The meter
never changes audio or adds exported frames. Nonfinite/overflowing input is refused.

The approach follows [BS.1770-5 Annex 2](https://www.itu.int/rec/R-REC-BS.1770-5-202311-I/en).
[EBU Tech 3341 Table 1](https://tech.ebu.ch/docs/tech/tech3341.pdf) supplies applicable
phased-tone and transient minimum tests. The study predeclared a tighter 0.2 dB
agreement limit through 0.45 cycles/sample against an independent 64×,
256-tap-per-phase Kaiser reference. Near-Nyquist stress is reported separately;
this is a tested estimator, not certification of every possible waveform.

Delivery reserves **0.4001 dB** below its target: 0.4 dB for interpolation/reference
uncertainty and 0.0001 dB for quantization. Sample-peak policy reserves 0.0001 dB.
The exporter measures the actual float32 bus, applies one constant gain and
remeasures the final PCM. At most one computed additional attenuation is allowed
for independent peak finalization; unresolved excess fails completion. The
independent verifier uses zero-padded FFmpeg/libsoxr 32× interpolation at precision
33 in float64, records six-decimal peak readings and refuses any measured result
above the ceiling or disagreement exceeding 0.2 dB. A nominal meter tolerance
never excuses an over-ceiling result.

Reports include sample/true peak positions, RMS, integrated K-weighted loudness
and maximum three-second K-weighted loudness where a complete window exists.
Loudness describes the finished file and does not choose primary delivery gain.
The existing K-weighting implementation is retained; it is not a certified meter.
The normal suite checks silence, short/boundary signals, both sides, phased
intersample overs, rates, independent controls, stale evidence and recovery.

## Stage observations and attribution

`stages.json` names source, post-filter, post-compressor before makeup, routed
channel, direct ensemble, combined returns, master HPF and final master stages.
These taps report sample peaks and RMS. True peaks are measured separately on the
retained bus and PCM. `stage-windows.csv` retains 100 ms mean/maximum gain reduction,
coherent group power, incoherent power and their signed difference; source powers
are not additive percentages of mix power. It also reports stage RMS/peak windows
for crest distributions and group band powers from adjacent Butterworth HPFs at
40, 120, 500, 1500 and 5000 Hz, capped at 0.45 times the sample rate. Bands overlap.
The stage RMS/peak rows use the two named dB columns as RMS and peak respectively.

At the largest output sample, signed source and return contributions pass through
separate copies of the same common linear master filters, then the observed linked
gains. The report reconciles their sum with the production output. This attributes
an observed waveform; it does not predict a nonlinear leave-one-source-out remix.
Mono compatibility uses `(L+R)/2`, separately from stereo power. Role-based groups
are explicit measurement groups and do not establish microphone/capture identity.

`neutral-reference.json` compares each production bypass frame with independent
pan equations and a compensated sum. Its absolute forward-error budget depends on
operation count and the sum of absolute contributions, including cancellation.
Normal fixtures additionally check the float32 rounding and exact PCM conversion.

## Completion and listening checkpoints

`delivery-ready.json` is published last and pins settings, policy, source identities,
engine/delivery reports, the readable `report.txt`, retained bus, primary PCM and
any comparison copies. The text summary reports the new gain and finished PCM
peaks. Reused legacy export observations are retained under
`historical_export_observations`; their old top-level gain fields are cleared so
consumers cannot mistake them for the new delivery.
It certifies internal file verification. `independent-verification.json` supplies
the separate meter evidence required for this study's listening checkpoint.
Interrupted or failed directories without valid completion records are incomplete.
Hash checks detect changed content; they are not signatures or proof of provenance
for arbitrary supplied files. Recover with frozen inputs and a fresh destination.

`scripts/listening_checkpoint.py` accepts a separate `contract: "gigpies-delivery-v1"`
specification with `expected_ids` and examples containing `id`, `delivery`, and
`starts`. A second passage requires `second_passage_reason`. It binds policy,
reports, independent verification and PCM, copies exact 12-second excerpts and
writes its manifest last. The legacy SOURCE/FINAL specification retains its original
−0.01 dBFS contract. Neither branch can silently interpret the other's reports.

The external validation sweep and complete private studies are opt-in. Their
commands and concise results are retained in the ignored execution directory;
recordings and generated audio remain outside Git. No automatic downloads, playback,
PA alignment or hardware operation occur in these commands.
