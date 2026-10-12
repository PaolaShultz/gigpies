# GP-PANEL terminal evidence — 2026-10-11

Owner: [GP-PANEL](../../development/MODULE_IMPLEMENTATION_PLAN.md#gp-panel--shared-physical-instrument-panel).
This is dated evidence, not a completed-task record. The current contract and
reproducible authoring/launch commands live in [Instrument panel](../../reference/INSTRUMENT_PANEL.md).

## Actual renders

The initial three compositions were rejected by the operator on October 12.
Their captures established terminal/font mechanics, not visual acceptance. Those
superseded PNGs and per-experiment scratch directories have been removed.

Current retained captures: [Desk channel](../../assets/panel-desk-channel.png),
[local draft/focus](../../assets/panel-desk-channel-draft.png). These are actual
Rio 0.5.28 ARM64/X11 tiny-skia windows under Xvfb on the Pi, not browser mockups.

The original 77-glyph proof established COLRv0 colour, bounded tile footprints,
16/20/24 px joins, resize/fallback and direct keyboard edges. COLRv1 failed colour;
Glyph Protocol square fitting failed continuous fader geometry. Normal cleanup
left its 77 queried slots empty. tmux lacked release events and SSH authentication
was unavailable. These limits remain; no physical display/output qualification.

## Matched application work

Release profile, Rust 1.97.1, CARGO_INCREMENTAL=0, parent build reservation held,
10 warmup frames then exactly 100 frames in the original order per workload.
Previous renderer executables are the frozen `user/ui-render-compare-20261011/`
fixtures. Their regenerated scene JSON was byte-identical to the input replayed
by the shared Ratatui `examples/bench.rs`. Logical canvas: 1920x1080 versus
160x45 cells at 12x24; the actual-terminal observer verified 160x45.

| Workload | Previous CPU median ms | Ratatui median ms | Ratatui p95 ms | ANSI bytes /100 |
|---|---:|---:|---:|---:|
| desk pages | 6.063 | 1.045 | 1.182 | 1,487,832 |
| desk meters | 3.818 | 0.388 | 0.400 | 185,643 |
| desk spectrum | 3.826 | 0.426 | 0.435 | 442,898 |
| desk unchanged | 6.220 | 0.672 | 0.677 | 19,935 |
| light pages | 20.847 | 0.699 | 0.922 | 313,644 |
| light unchanged | 21.702 | 0.796 | 0.806 | 15,431 |

Previous times include CPU raster and RGBA preparation, excluding GPU upload and
presentation. New times include cell construction, Ratatui diff and ANSI generation
into a counting sink, excluding PTY/emulator/presentation. New scene JSON decoding
and adapter conversion are outside the timed draw loop, just as old scene creation
is excluded. These numbers do **not** establish an end-to-end display speedup.
The benchmark uses `scene::draw`; the dense `instrument_text` page projection and
actual input/provider processing are not included in this timing comparison.
Pages include changing parameter values; meters/spectrum are labelled synthetic
benchmark signals. This is not evidence of a production spectrum capability.

Unchanged frames emit 19 bytes after the first frame, rather than another screen.
The installed-font path registers zero glyph bytes at startup; the experimental
protocol path sent 30,584 base64 payload bytes plus headers/queries and measured
173.5 ms registration. `/usr/bin/time -v` includes parsing/warmup/process costs:
Desk app benchmark 0.60 CPU seconds, 121,136 KiB peak RSS. These are whole-process
figures for four workloads, not per-frame renderer allocations.

## Terminal observation

`examples/present.rs` draws a unique binary marker in the last row. The Python
observer reads the actual X framebuffer and sends the next-frame ACK only after
seeing that marker. This prevents a successful PTY write being called a presented
frame. Readback/polling and process scheduling are included; physical scanout is
excluded. The marker adds a changed row even to the unchanged workload. Terminal
RGB is colour-processed, so binary luminance markers replace exact-RGB matching.

The first Desk-pages run observed all 100 frames: median 100.54 ms, p95 158.99 ms,
startup to first visible frame 438.13 ms. Rio plus child: 24.03 CPU seconds,
142,000 KiB peak RSS, 12.24 s wall time. This software Vulkan path is substantially
slower than the application-only draw. It does not qualify a fast physical-display
path or compare the old native renderer's GPU presentation.

Reproduce after building the shared examples with the package's documented build
reservation and release flags:

```sh
target/release/examples/bench user/ui-render-compare-20261011/desk artifacts/instrument-panel/desk-ratatui.json
/usr/bin/python3 scripts/measure_panel_terminal.py user/ui-render-compare-20261011/desk/pages-scenes.json --name presentation-desk-pages
```

The local frozen workload hashes are:

- `desk/pages-scenes.json`: `b4da1171b119bec8b70993efedee43f80fca0d4ca5fc9ef6623ffa9d86792c41`
- `desk/meters-scenes.json`: `c0060694caa433502fc7d7e5d97f95eca43433d344ca36fe3d6bbc88a1b68047`
- `desk/spectrum-scenes.json`: `88521231e72463956db138360da236291880928f06c25cff1d7a237c13b09c44`
- `desk/unchanged-scenes.json`: `cc46c37cba19035d12634902617304ca210bf3cbd171fac9d179cb79f6776821`
- `light/pages-scenes.json`: `cea7cd96f85508d4cbb9c2153b77baf2f120aa7242aa7764024f865287062876`
- `light/unchanged-scenes.json`: `5179d69ba76c8776f82d0f1c116943800e246d8e22fa3aad1531504cbd06b953`

## Final software validation

Exact shared package `0.1.2`, Rust 1.97.1, one Cargo job, incremental disabled,
canonical target directories and the parent build reservation:

| Suite | Passed | Intentionally ignored |
|---|---:|---:|
| Shared package, all targets | 11 | 0 |
| Desk default, all targets | 329 | 21 |
| Desk native feature, all targets | 332 | 21 |
| Lightdesk default, all targets | 135 | 5 |
| Lightdesk native feature, all targets | 136 | 6 |

All three passed rustfmt and Clippy with `-D warnings`; consumers were checked
with and without the native feature. Shared tests cover cell width/uniqueness,
small-rectangle clipping, fractional endpoints, unavailable versus zero,
protocol fragmentation/bounds and retention of review text. Existing consumer
suites protect grants, exact reviews, persistence/recovery and provider contracts.
Ignored tests require separately accepted provider/module artifacts, explicit
historical galleries/long experiments, or GPU/physical acceptance; they were not
silently added to the default run. No physical audio/MIDI/DMX test ran.

Commands from each consumer root, under the shared reservation, were
`cargo +1.97.1 test --locked -j1 --all-targets`, repeated with `--features native`,
and corresponding `clippy ... -- -D warnings`. Shared commands use
`--manifest-path src/instrument-panel/Cargo.toml` and GigPies' existing target.
Candidate publication checks used temporary indexes for each repository and
passed without staging the user's actual index or committing anything.

Actual-window checks also exercised F2 page changes, resizing below the minimum,
and stopping the task-owned provider after connection. Lightdesk retained its
last values and changed freshness to Stale; the shared red unavailable LED turned
on and the green fresh LED turned off. Preview startup waits for a listening
socket and reads Lux's newly reserved epoch, rather than reusing epoch 1.

## Headless terminal backend comparison

Rio's own tiny-skia CPU backend preserved the colour-font appearance and reduced
observed presentation time versus lavapipe. Applications still use Ratatui cells;
this does not introduce an application pixel framebuffer. The option is documented
in the [exact Rio release source](https://github.com/raphamorim/rio/blob/v0.5.28/rio-backend/src/config/renderer.rs).

All rows below used 100 frames at a verified 160x45 cells/1920x1080 content,
identical scene files and order. CPU/RSS are Rio plus its child, excluding the
Python observer and Xvfb server. Timings include the observer's readback overhead.

| Workload | Median ms | p95 ms | Startup ms | CPU seconds | Peak RSS KiB |
|---|---:|---:|---:|---:|---:|
| desk-meters | 46.46 | 67.37 | 148.30 | 0.96 | 27,072 |
| desk-pages | 45.81 | 65.30 | 314.24 | 1.23 | 120,192 |
| desk-spectrum | 45.79 | 53.39 | 156.57 | 0.92 | 27,008 |
| desk-unchanged | 45.94 | 51.74 | 196.87 | 0.95 | 31,456 |
| light-pages | 47.02 | 67.39 | 165.84 | 1.02 | 27,504 |
| light-unchanged | 45.52 | 54.35 | 176.16 | 0.94 | 26,976 |

Reproduce with the same `measure_panel_terminal.py` command plus `--cpu-terminal`.
The headless preview contract supports this faster terminal backend explicitly;
physical GPU/HDMI and previous native-renderer presentation remain unmeasured.

## Retention

Publication-policy unit tests also passed: GigPies 6, Desk 9, Lightdesk 9.
Actual Git indexes stayed unchanged. No commits, pushes, deployments, host
audio/service changes, hardware activation, extra worktrees or delegated workers.

Cleanup removed 20,331,483,056 bytes of completed task test executables and about
203 MiB of duplicate baseline renders/data, the extracted download and disposable
font/Python caches. Final free space was 26 GiB. Targets retained: GigPies 4.7 GiB,
Desk 6.0 GiB and Lightdesk 3.0 GiB; Desk remains above the review threshold because
reusable libraries/debug information and existing output were preserved. The
32 MiB terminal evidence/tool directory includes the runnable Rio binary, generated
font and unique captures. Task-owned terminal/provider processes were stopped.

## Input-edge and unsupported-protocol qualification

The opt-in `examples/keys.rs` probe opened the same shared Console lifecycle and
recorded actual injected T/Enter press and release events. Direct Rio reported
`release_events=true` and delivered both edges plus focus events. The private
tmux 3.5a path reported `release_events=false` and delivered presses only.
Consequently tmux is qualified for rendering/legacy input only, not held talkback
or confirmation; the production adapters explicitly refuse those actions there.
No owner/device was involved in this input test. The source example retains the reproducible probe; disposable text logs were
removed after recording the result here.

A Glyph Protocol request through that tmux path timed out in 505.5 ms and the
specimen automatically displayed its readable fallback. Its disposable capture
was removed after recording this result. The capture helper
waits for application-area pixels, excluding title-bar antialiasing, before
injecting keys and taking the final screenshot.


## Selected-channel correction — 2026-10-12

The shared source is now 0.2.1, with 419 reusable tiles (231,348 base64 bytes;
installed-font mode sends no uploads). Correct per-layer horizontal bearings
removed seams through the multi-cell encoders. The current font SHA-256 is
`a8e179dec365a6be2f1e4004ca7276a5acf776cc18802b5d04064dbff2b0798b`.
The expanded protocol repertoire is not requalified; primary deployment is the
local COLRv0 font with the explicit `SHR Panel Mono:0.2` marker.

The Channel page reads actual GP07/GP03/GP-METER data and uses the existing
semantic edit/review/confirm paths. An actual terminal keyboard episode enabled
EQ, changed band 1 to +3.0 dB, enabled compression and set 4:1. Independent GP07
readback confirmed revision 1, ready/settled current and target, and live detector
feedback (0.540 dB in one sampled observation). This is a private software source,
not physical audio. A separate mouse click selected the ratio field and opened a
local draft while independent owner settings remained unchanged.

Reproduce the settled capture (the long field sequence is a UI exercise, not a
new automation or provider path):

```sh
python3 - <<'PY'
import subprocess
keys=['g','e','k','i','i','3','Return']+['i']*15+['k','i','i','4','Return','F4','Next','Return']
subprocess.run(['/usr/bin/python3','scripts/preview_panel_providers.py','desk',
 '--channel','--cpu-terminal','--suffix','channel-confirmed','--keys',','.join(keys)],check=True)
PY
```

The ordinary Unicode fallback was inspected using unmodified DejaVu Sans Mono
at 20 px (its metrics differ from the panel font), in the same window dimensions.
The 160×45 minimum remains enforced. Complete all-target validation and cleanup
results are recorded in the owning GP-PANEL card rather than separate run reports.
Historical six-workload timings above describe the previous transitional scene
adapter, not this new dedicated channel layout; no new display-rate claim is made.


## MIDI presentation correction — 2026-10-12

The operator accepted the rotary appearance and clarified that MIDI is the
primary input. The two linked captures have been replaced: the operating page
now uses plain control names, no keyboard legend, and larger three-cell lens LEDs.
The settled capture shows EQ/compression active (+3 dB, 4:1 independently read
back) and fresh input sample peak; the draft capture shows blue selection and
amber unapplied state with processor lamps dark because the actual owner is
bypassed. A draft never lights the settled-owner processing lamps.

Shared package 0.3.1/font 0.3 adds 24 reusable lens tiles, for 443 total and
246740 base64 bytes. The installed COLRv0 font SHA-256 is
`361665c643fa77d53f4000f0c7e9b2c27e41d7c2d82e29828dc19821c9be66e8`.
The primary font marker is now `SHR Panel Mono:0.3`. No terminal upload or
per-frame asset generation was added. This capture uses Rio/Xvfb on the Pi,
not physical MIDI input; proposed channel controller mappings are not qualified
by a rendering change. The existing reproduction command still applies.


## Publication revalidation — 2026-10-12

Shared 0.3.1 and both consumers passed all five normal configurations: 948 Rust
checks total, with 24 publication-policy checks; formatting, warning-denied Clippy
and release builds passed. The owning GP-PANEL card records exact commands,
skipped opt-ins and cleanup. Publication/source synchronization is now explicitly
authorized; the earlier no-commit notes describe their original capture passes.
No fresh visual or physical-device qualification is implied.
