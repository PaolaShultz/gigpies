# GP-UI-COMPARE — 100-screen renderer comparison

State: **ACCEPTED — requested software microbenchmark completed**. This closes the
comparison only; no production migration or terminal/display qualification is
claimed. Owner: GigPies integration; contributors: SHR Desk and SHR Lightdesk.
User requested matched 100-screen rendering measurements before further UI changes.

## Plan executed

Use the current release libraries without modifying production renderers. Generate
matching deterministic scene inputs, compare the bitmap rendering paths with a
Ratatui cell projection, inspect paired output, retain raw timings and reproducible
commands, and state what display work was excluded. Keep providers, authority,
hardware, services and host fonts unchanged.

## Inputs and method

Pi5 Model B Rev1.1; Rust1.97.1 release optimization; Ratatui0.30.2 and Crossterm0.29.0.
Desk `9b64c88bfd4f47d1ba8b405adde4f67b8fc3f8da`;
Lightdesk `9753a96496adf3a634af932a6e0ae88f4db9b66b`.
One process at a time with the shared build reservation, one build job,
CARGO_INCREMENTAL=0 and existing target directories. Initial free space3.6GiB was
reviewed; no unknown/shared cache was removed. Production dependency locks unchanged.

Three trials, each with100 frames per scenario per path after10 warm-ups.
Page sequences cover three Desk and seven Lightdesk fixture layouts; dedicated
16-meter/64-band synthetic scenes and unchanged frames provide separate cases.
600 inputs per path per trial,3600 measured draws total. Both consume the same
primitive/text inputs; state construction/JSON decoding are outside timing.

Desk calls actual `raster::rgba`; Lightdesk calls actual `render::ppm` and the same
RGB-to-RGBA preparation as native upload. Ratatui includes cell construction,
Terminal::draw diffing and Crossterm ANSI encoding to an in-memory byte writer.
Neither native GPU upload/presentation nor terminal PTY/emulator/display work is
measured. This is not an FPS, controller-response or end-to-end latency comparison.

## Results

Median total milliseconds per100 frames, across three trials:

| Matched workload | Current CPU renderer, ms | Ratatui + ANSI, ms | Ratio |
|---|---:|---:|---:|
| Desk: three-page sequence | 602.30 | 97.52 | 6.2× |
| Lightdesk: seven-page sequence | 2087.26 | 66.18 | 31.5× |
| 16 changing meters | 393.09 | 40.59 | 9.7× |
| 64 changing spectrum bands | 399.45 | 45.15 | 8.8× |
| Desk: unchanged Mix | 633.41 | 64.78 | 9.8× |
| Lightdesk: unchanged Stage | 2171.52 | 66.78 | 32.5× |


All121,938 label/value comparisons passed. Two focused release harness tests
passed (eight block heights and exact text/unchanged diff). Paired layouts, meter
and spectrum output were inspected. Thin borders and graph curves are visibly
coarser; a real migration needs cell-native layouts/charts. The adapter preserves
text, snaps positions to cells and bars to fractional blocks; it is not a completed
production frontend. Native screenshots are actual CPU raster output; Ratatui
screenshots are buffer replays with existing text glyphs plus ideal block geometry,
not terminal-emulator screenshots. Disposable ANSI/asciicast output and individual
preview images were removed after this result was recorded.

The implemented Ratatui prototype uses substantially less application rendering
work for these fixtures. This supports the requested Ratatui direction, without
claiming all native renderers are slow or that terminal display processing is free.

## Evidence, reproduction and limitations

Private retained directory: `user/ui-render-compare-20261011/`.
`REPORT.md` has full method/limitations; paired previews can be regenerated;
`results.json`, `summary.json`, `provenance.json` retain timings, checksums, corpus
identities, source pins and environment. Repeated run dumps and per-frame exports
were removed; the six frozen scene inputs remain for the open terminal-performance
comparison. Source harness, pinned Cargo.lock and
build/run/preview/report scripts make the checks reproducible. Run from GigPies:

```sh
bash user/ui-render-compare-20261011/build.sh
flock -xn /home/shome/p/.gigpies-build.lock python3 user/ui-render-compare-20261011/run.py
python3 user/ui-render-compare-20261011/preview.py
python3 user/ui-render-compare-20261011/report.py
```

Initial harness substring/TTY-clear errors and projection contrast/handle/gap
issues were corrected; final measurements follow the fixes. Full engine suites
were not repeated for unchanged production source. Historical/exhaustive, live
provider, physical display/controller and two-Pi load suites were skipped. No
font installation, hardware activation, service/audio change, commit or deployment.

Next: use this comparison to choose the production Ratatui migration scope;
measure the actual selected terminal backend and input-to-display path separately.
