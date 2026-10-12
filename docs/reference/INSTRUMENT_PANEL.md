# Shared instrument panel

The shared package and specimen live in `src/instrument-panel/`; Desk and
Lightdesk expose explicit Ratatui entry points over their existing provider
workers. Providers retain authority. Full physical-control migration is tracked
in [GP-PANEL](../development/MODULE_IMPLEMENTATION_PLAN.md#gp-panel--shared-physical-instrument-panel).

## Rendering contract

Ratatui owns cell layout, widget rendering and incremental terminal updates.
Labels/values remain ordinary text. Original reusable colour glyphs supply
material boundaries, LED lenses, bounded glow and fader parts. No whole-screen
image upload or per-frame glyph generation is used. The current repertoire is 443 tiles,
246,740 base64 payload bytes before protocol headers in experimental protocol mode.
The primary installed-font path uploads no glyphs at startup or during updates.

Supported local presentation: Rio 0.5.28 ARM64/X11 with the generated **SHR Panel
Mono COLRv0** font, 16/20/24 px, default line spacing. Explicit application-local
fontconfig and `SHR_PANEL_FONT='SHR Panel Mono:0.3'` select the tiles. Ordinary
Unicode is the fallback; unsupported terminals must not set that marker.
COLRv1 failed the selected build's colour rendering. Glyph Protocol registration
works, but square fitting breaks tile geometry, so it is experimental only.
Direct Rio is the qualified key-release path. Private tmux is qualified for
rendering/legacy input only: its missing release events cause talkback and
confirmations to be refused. SSH remains unqualified.

COLRv0 layers approximate gradients with bounded bands. Authoring/protocol tiles
use 1000-unit square coordinates; the installed font maps them to 1024-unit
advances and a 2048-unit line box. Ordinary labels use matching solid faces;
no text is baked into tiles. Lit/off LEDs include explicit ON/OFF text. Selected
blue means interaction focus, never proof of an owner ACK. Pending/confirmed,
failed/disconnected/stale and unavailable provider states must remain independent
from the local button appearance. Missing measurements never become zero.

## Asset ownership and reproducibility

Run `python3 scripts/build_panel_glyphs.py` from GigPies. Standard-library Python
writes deterministic original MIT geometry to `src/instrument-panel/src/assets.rs`
and the codepoint registry `src/instrument-panel/assets/registry.json`. There are
no font downloads, host-font edits or generated screen images in that build.
The geometric source is editable, unhinted simple TrueType outlines plus generated
OpenType COLRv0/CPAL tables in protocol containers. The control geometry contains no proprietary or third-party artwork. The
optional text-font assembly uses the separately licensed DejaVu foundation below. U+101000..U+1011BA avoids the current
Nerd Font BMP/PUA-A allocations; query every slot before claiming it because
private-use ranges are not globally exclusive.

Font foundations examined:

- [Hack licence](https://github.com/source-foundry/Hack/blob/master/LICENSE.md):
  MIT additions, public-domain DejaVu changes and Bitstream Vera conditions.
- [Fira Code licence](https://github.com/tonsky/FiraCode/blob/master/LICENSE):
  SIL OFL 1.1. Suitable text foundation, unnecessary to modify for protocol tiles.
- [Powerline Extra Symbols licence](https://github.com/ryanoasis/powerline-extra-symbols/blob/master/LICENSE):
  MIT; geometric separators do not supply the requested material controls.
- Installed DejaVu Sans Mono: Bitstream Vera licence with public-domain DejaVu
  changes, verified in `/usr/share/doc/fonts-dejavu-core/copyright`. The derived font is renamed and retains the licence notice.

## Production workflow preservation inventory

Use source/current native contracts, not simulator screen counts, during migration.

| Consumer and current owner | Workflows that must remain reachable |
|---|---|
| Desk `frontend.rs`, `actions.rs`, `native.rs` | Mix, Channel, Health/Analysis; selection and dynamic banks; FOH/monitor scopes; fader/pan/mute; detached exact dB/pan editors; explicit grants/release/reconnect; MANUAL/ASSIST/AUTO and scoped reviewed release; full review paging and presentation coverage |
| Desk processing/master EQ/sends | Four channel bells/compressor and independent bypass; eight parametric/31 graphic master bands; linked/independent L/R; setup mute and separate rearm; narrow live EQ; per-send tap and value review |
| Desk topology/Brain/meters | Structure/device mapping, physical vs logical/USB identities; Brain duplex review; PFL/AFL/listen; held talkback with release/focus-loss safety; raw/pre-fader GP-METER and independently fresh gain reduction; monitor banking |
| Desk FX/measurement | Prepared FX configuration and permit/applied/settled state; tail bypass/history reset; PA measurement capture/review/candidates; capability/freshness/refusal paths |
| Lightdesk `frontend.rs`, `native_actions.rs`, `native.rs` | Stage, Programmer, Library, Playbacks, Health; selection/bank; capability-checked exact attribute entry; grant; record/GO; explicit zero/Hold; playback/Auto return review; mode/blackout review; full context-bound review paging; console commands and reconnect |
| Lightdesk music/scenes | Read-only named analysis/harmonic/recurrence provenance; source loss; scene inventory, detached map editing, nonmutating owner preview; real null/disarmed output remains labelled physical UNKNOWN |
| Both | Independent provider workers, bounded queues, role gates, no UI restart recall, no lost-input replay, pending vs confirmed, owner epochs/caps/freshness, keyboard/focus/resize and complete review visibility |

The specimen's meter/spectrum motion is explicitly synthetic. It does not create
a spectrum capability in production. Existing providers supply only their actual
advertised observations. A legacy terminal without release events cannot safely
carry held talkback: that action must remain unavailable unless a qualified input
protocol supplies the required edges/focus lifecycle.

## Investigation provenance

[Glyph Protocol specification](https://github.com/raphamorim/rio/blob/59203ccd5fa1e6b760a738c1ad653bb29f2f3465/specs/glyph-protocol.md)
and the corresponding `rio-vt/src/ansi/glyph_protocol.rs` were inspected. It
advertises glyf/COLRv0/COLRv1, supports bounded containers and per-slot clearing.
The [upstream example](https://github.com/raphamorim/glyph-protocol-examples/tree/c4f18b5fde7902af1d01f0c2fcc2a8dadd1a08ad/ratatui)
is plain Crossterm/libc output, not a Ratatui integration. Its comment equating
flush with atomic PTY writes is not a portable guarantee; our parser is incremental.
The original [introduction](https://rapha.land/introducing-glyph-protocol-for-terminals/)
is design background, not a terminal compatibility certificate.

Desk's previous path is `native::Gpu::render` → `raster::rgba` → whole texture;
Lightdesk's is `native::upload` → `render::ppm` → RGB/RGBA expansion → whole texture.
Their blueprint's native-wgpu direction explains the drift. The earlier
[comparison](../archive/records/2026-10-11-gp-ui-compare.md) measures application
work only. Its buffer-replay previews are not real terminal screenshots.

## Local font proof and launch boundary

Actual Rio 0.5.28 ARM64/X11 with Mesa lavapipe on rpi5 rendered COLRv0 from
an application-local SHR Panel Mono font. The initial protocol screenshot
exposed square-fit glyphs: Rio's parser accepts registration but does not
implement the documented `aw/lh/size=stretch` fields. Its current
`glyph_cell_scale` uses uniform em/bounding-box containment. Protocol registration
therefore remains experimental; it is not the primary material path.

SHR Panel Mono adds only the registered PUA tiles to DejaVu Sans Mono and renames
the font. Letter/number outlines are consistently rescaled to an exact half-em advance
and one-em line box, preventing fractional-advance seams at qualified even pixel
sizes (16, 20 and 24). Odd sizes and extra terminal line spacing are unqualified. The font is made available only
to the launched process via FONTCONFIG_FILE; no desktop-wide font setting changes.
`SHR_PANEL_FONT=SHR Panel Mono:0.3` explicitly selects colour tiles in the console;
without that contract the renderer uses ordinary Unicode. Never set it for an
unverified terminal/font combination. `NO_COLOR` must be unset for colour mode.

Production entry points are `shr-desk --terminal` with the same provider arguments
as `--native`, and `shr-lightdesk terminal` with the same provider arguments as
`native`. No default invocation connects. Existing native/offscreen paths remain
available during acceptance. The first adapters preserve the source scene's
160x45 review geometry through shared cells; they do not rasterize a framebuffer.
This transitional layout is not yet the final cell-native redesign of every page.
Keyboard release support is queried, not assumed. Unsupported held talkback and
Lightdesk confirmation are refused visibly; the ordinary fallback remains readable.
The specimen exercises mouse interaction; the production Channel page supports
field selection and existing action buttons. The primary operator input is MIDI,
not a computer keyboard. Physical MIDI channel bindings remain unqualified.

Reproduce local font and specimen from GigPies:

```sh
/usr/bin/python3 scripts/build_panel_glyphs.py --font /usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf --out artifacts/instrument-panel/fonts/SHRPanelMono.ttf
# Build panel-specimen with the package README's locked, parent-held command.
/usr/bin/python3 scripts/preview_panel.py --name specimen-final
```

The authoring input SHA-256 is
`54bf827eb99404e8f430c330ad30f063334f637eba0109b6a18d4f566a8e9dd8`.
The included `src/instrument-panel/assets/DEJAVU-LICENSE.txt` retains the foundation
licence; the original control geometry/code is MIT. No Nerd/Powerline outlines
are bundled. Font generation is independent of protocol-payload generation.
The preview command opens only an owned Xvfb/Rio process pair and forces software
lavapipe; it does not open audio/MIDI/DMX or change a host service.

## Preview commands and qualification boundary

These captures are from the actual ARM64 Rio process on this Pi, using Xvfb
and explicit software terminal backends (tiny-skia or Mesa lavapipe). They are terminal framebuffer screenshots, not browser
mockups or Ratatui buffer exports. They do not qualify HDMI, physical GPU scanout
or a TV. The primary font uses COLRv0; the local COLRv1 experiment rendered flat
monochrome silhouettes and is **not supported** by this selected build.

Provisioned tools: Debian Xvfb, xdotool, Pillow, fontTools and Mesa Vulkan;
Rio 0.5.28 ARM64 X11 was extracted only into
`artifacts/instrument-panel/tools/rio`. The release Debian archive SHA-256 is
`8f838db9aeba500c4836c46dac42201f81470a7cdd6df2d10697b0608e66fbae`.
No host terminal configuration or font selection was changed. Release download:
`https://github.com/raphamorim/rio/releases/download/v0.5.28/rioterm_0.5.28_arm64_x11.deb`.
Verify the upstream checksum before extraction with `dpkg-deb -x` into that path.

```sh
# Actual-terminal specimen, exits after retaining its screenshot:
/usr/bin/python3 scripts/preview_panel.py --name specimen-final
/usr/bin/python3 scripts/preview_panel.py --name specimen-small --font-size 16 --width 1280 --height 760
/usr/bin/python3 scripts/preview_panel.py --name specimen-large --font-size 24 --width 1928 --height 1122
# Explicit private offline owners and read-only production UIs; no writer grant:
/usr/bin/python3 scripts/preview_panel_providers.py desk lightdesk
# Ordinary readable fallback, with no PUA rendering request:
/usr/bin/python3 scripts/preview_panel.py --name specimen-fallback --command /home/shome/p/gigpies/target/debug/panel-specimen
```

The provider preview requires existing `gigpies-headless`, `lux-service`,
`shr-desk` and `shr-lightdesk` binaries at their documented canonical target paths.
It neither builds nor changes the Lux repository. Desk's captured eight-input
legacy fixture is not a product capacity limit. Lightdesk's fixture inventory is
explicitly synthetic and its actual owner reports null/disarmed output.

Direct Rio and an isolated tmux 3.5a server were exercised. The private tmux
configuration enables RGB and extended keys; it is not installed into the user's
normal tmux configuration. SSH qualification remains open: a strict host-verified
localhost connection was refused by public-key authentication. No authentication,
SSH service or key configuration was changed. Do not claim SSH or another
terminal/multiplexer combination inherits the direct-render qualification.

The installed font needs no per-frame or startup glyph upload. The experimental
Glyph Protocol session probes support and each empty slot, registers a bounded
repertoire, and clears only its own slots on normal exit. Fragmented
replies and bounded malformed input are unit-tested. Timeout/reconnect cleanup
and delayed replies are not qualified; use the installed-font primary contract.

The current 0.2 font is byte-reproducible with SHA-256
`361665c643fa77d53f4000f0c7e9b2c27e41d7c2d82e29828dc19821c9be66e8`.
Font-size captures at 16/20/24 px show continuous boundaries after the metric
correction. The palette is visually distinct with bounded highlights/glow;
framebuffer RGB is affected by Rio's colour pipeline and is not bit-exact or a
calibrated physical-display measurement.

The historical 0.1 experimental protocol run registered all 77 tiles in 173.5 ms (30,584
base64 payload bytes, plus protocol headers/queries). A post-exit query in the
same PTY confirmed **all 77 slots empty**. The updated probe reads the current
registry; the expanded 443-slot protocol path remains unqualified. Run explicitly:

```sh
/usr/bin/python3 scripts/preview_panel.py --name specimen-protocol --family 'DejaVu Sans Mono' --system-fonts-only --command sh -c '/home/shome/p/gigpies/target/debug/panel-specimen --glyphs; /usr/bin/python3 /home/shome/p/gigpies/scripts/probe_panel_cleanup.py /home/shome/p/gigpies/artifacts/instrument-panel/protocol-cleanup.json'
```

Its screenshot also demonstrates why it is experimental: uniform square fitting
leaves gaps in the fader track and compresses LEDs vertically. Normal registration
and cleanup succeeding does not establish usable tile geometry.

For this headless Pi, append `--cpu-terminal` to the specimen, provider-preview
or terminal-measurement commands to use Rio's internal tiny-skia renderer. It
preserved COLRv0 appearance and observed all six 100-frame workloads at median
45.5–47.0 ms including framebuffer readback, versus 100.5 ms for the lavapipe
Desk-pages run. This is a terminal backend choice: both applications still emit
Ratatui cell updates. See the [dated evidence and limits](../archive/records/2026-10-11-gp-panel-proof.md).

Current actual renders: [Desk channel](../assets/panel-desk-channel.png) and
[local draft/focus](../assets/panel-desk-channel-draft.png). Rejected earlier
compositions and disposable protocol captures were removed after their findings
were recorded; the source and repeatable commands remain.


## Desk selected-channel page

The dedicated Ratatui Channel page requires the existing `--processing` capability
probe and FOH context. Four bell bands (Hz/dB/Q and independent bypass), global EQ
bypass, compressor threshold/ratio/knee/attack/release/makeup/bypass, pan, fader,
mute, Hold and monitor-send navigation use the existing owner paths. Unsupported
preamp gain, phantom, polarity, HPF and gate are not advertised as inert controls.
Other scopes and all exact-entry/review/modal pages retain their existing routes.

Click a parameter or encoder to open/select its detached processing field. U/I
select fields; J/K adjust fine, N/P coarse; numeric entry plus Enter accepts text
locally. F4 opens the complete existing review; only a separate Enter after all
review pages are presented can confirm. Clicking the output fader/pan opens the
existing exact-value editor. Mute/Hold/release still use owner review. The encoder
is a field-selection target; continuous mouse dragging is not implemented.

The graph is the calculated four-bell **settings response**, using the existing
Desk EQ-response calculator and the owner's 48 kHz RBJ-bell convention. Its window
is ±18 dB; it is not a spectrum or a measurement. Draft settings are amber and
explicitly labelled; settled and target values remain distinct in the inspector.
Gain reduction reads the owner detector independently of drafts. Input sample
peak is the real GP-METER observation; stale data cannot light a live meter.

The 0.3 font preserves corrected colour-layer side bearings and supplies bounded 6×3-cell
metal encoders with 17 indexed positions, a 6×2 slider cap and reusable button
edges. Regenerate the local font before setting the `:0.3` marker. No glyph is
created or uploaded per frame. The historical 77-slot protocol measurements are
not measurements of the expanded repertoire; downloaded glyphs remain experimental.

Reproduce a real terminal capture with the private software engine and explicit
injected role descriptors (no physical devices):

```sh
/usr/bin/python3 scripts/preview_panel_providers.py desk --channel --cpu-terminal --suffix channel
```

`--keys` sends paced keyboard gestures, `--click X,Y` targets a terminal cell in
window pixels, `--fallback` selects ordinary text, `--disconnect` drops only the
owned private provider, and `--resize WIDTH,HEIGHT` exercises recovery. A channel
capture prints a concise independent read-only GP07 owner readback. Temporary
X/terminal configs, logs and private synthetic-owner state are removed on exit;
only the requested PNG remains. No JSON dump is retained by default.
The regular launch remains `shr-desk --terminal ENDPOINT SHOW EPOCH WRITER foh
--processing`, with the documented role-provider options. Preview role descriptors
are synthetic assertions, not evidence of a connected controller/display.


### MIDI-first presentation

The operator chose MIDI as the primary input. The Channel operating page has no
computer-keyboard shortcut strip or shortcut prefixes; the keyboard bindings
remain a development/accessibility path described here, not the panel layout.
The existing controller plan owns physical mapping. Its channel rotary assignment
is still proposed: this rendering change neither opens a MIDI port nor claims a
working learned hardware profile. No unbound knob numbers are advertised.

Shared three-cell indicator lenses use original metallic bezel/dark-lens geometry
and bounded green/yellow/red/blue illumination (24 appended tiles). EQ, compressor
and band lamps follow independently fresh **settled owner** processing, never a
local draft. Mute indicates the owner's mute target; Hold indicates the owner hold.
Blue means a selected draft parameter; amber means a local unapplied draft.
Explicit captions and values remain readable without colour. Unlit lenses remain
visible. The new font requires `SHR_PANEL_FONT='SHR Panel Mono:0.3'`.
