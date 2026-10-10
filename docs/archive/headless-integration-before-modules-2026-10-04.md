# Local audio and lighting integration

This 2026-10-04 software milestone connects the real GigPies offline mixer and
Lux null-output engine to the independently built Desk and Lightdesk clients.
The [implementation map](plans/MODULE_IMPLEMENTATION_MAP.md#execution-checkpoint--2026-10-04)
tracks final acceptance. Native windows/controllers and physical outputs remain
separate work. No audio device, MIDI endpoint or DMX driver is opened here.

## Owning implementations

| Owner | Implemented path | Important boundary |
|---|---|---|
| GigPies | Eight mono inputs, stereo FOH, two monitor sends, human holds, proposals, rendered command completion and private Unix service | Synthetic input; offline-unprotected; meters unavailable; no PA/FX/REC binding in this new graph |
| SHR Desk | Strict provider decoding, explicit writer grants, scoped edits, engine previews, confirmation, renewal and exact retries | Client of GigPies; no mixer or arbitration algorithm in the surface |
| SHR Lux | Fixture/patch/programmer/Hold, cue/palette/playback authority, master/blackout, 500 ms release, durable checkpoints and private Unix service | Null output, restart disarmed; no physical-light observation or automatic output arm |
| SHR Lightdesk | Strict Lux pages, source attribution, real operator commands, reviewed release, renewal, uncertain recovery and checkpoint requests | Client of Lux; no replacement lighting engine |

The services use canonical owned directories with mode `0700` and Unix sockets
with mode `0600`. Same-user access, bounded frames/connections and deadlines are
implemented. This local binding does not close the production remote
authentication gate. Show/module/epoch, writer/lease, request identity and
revision are checked before a command can change authority state.

Audio commands enter at the next 48-frame boundary and ramp for 240 frames.
Pending acceptance is separate from rendered completion; actual coefficients
remain separate from target intent. A monitor send is post-mute and pre-FOH-fader.
Lux preview/release operates on its own authoritative look; the surface displays
the returned transition and does not calculate a competing fade.

## Run an explicit local session

Build each owning repository independently with Rust 1.97.1 and its Cargo.lock.
Follow the [one-build-per-host procedure](plans/PARALLEL_WORK_PLAN.md#one-build-slot-per-host)
before Cargo; use `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1` and `-j 1`.
The relevant release binaries are `gigpies-headless`, `shr-desk`, `lux-service`
and `shr-lightdesk`. No sibling path dependency or provider source copy is needed.

Create two fresh private directories, one per service, and provide their absolute
canonical paths. The synthetic Lux profile uses the show ID below. In separately
owned terminals, launch:

```sh
gigpies-headless --directory /absolute/private/audio \
  --show 11111111-1111-4111-8111-111111111111 --epoch 1
lux-service --synthetic-private-dir /absolute/private/lighting --durable
```

Both directories must already exist with the stated ownership and mode. Keep
their identity/checkpoint files when restarting. Audio requires an explicitly
increasing epoch; Lux reserves the next epoch itself. A crash may leave the owned
socket path: remove only that known endpoint after confirming its service has
exited. Never remove identity state to bypass a refused restart.

An audio batch script can contain:

```text
grant
input-release
set input-01 fader -3000
wait 50
status
release
```

Run it with a fresh writer identity:

```sh
shr-desk --audio-local /absolute/private/audio/audio.sock \
  11111111-1111-4111-8111-111111111111 1 desk-session-1 foh \
  --script /absolute/private/audio-commands.txt
```

The fader unit is integer milli-dB. A `monitor1` or `monitor2` writer can only
edit its named monitor send. Mode changes and release previews require explicit
confirmation. See [Desk provider instructions](https://github.com/PaolaShultz/shr-desk/blob/main/docs/PROVIDER_CLIENT.md)
for the full bounded script vocabulary and [audio service rules](../reference/AUDIO_LOCAL_SERVICE.md)
for lifetime, framing, restart and failure behavior.

A lighting batch can contain:

```text
grant
select fixture-11
touch intensity 700
record cue look-1
go look-1 playback-1
touch intensity 0
clearHold
preview intensity
release
release-input
wait 650
refresh
status
checkpoint
quit
```

Run against the service's current epoch:

```sh
shr-lightdesk lux-control --socket-path /absolute/private/lighting/lux.sock \
  --show-id 11111111-1111-4111-8111-111111111111 --epoch 1 \
  --script /absolute/private/lighting-commands.txt
```

Intensity is integer tenths of a percent. This example stores a cue, activates it,
establishes a manual zero, reviews the engine's release and waits for its endpoint.
Checkpointing preserves the current look, master and blackout. Restart restores
holds under a new epoch with no playback, writer grant or in-progress transition;
output stays `null_disarmed`. See the [Lightdesk owning plan](https://github.com/PaolaShultz/shr-lightdesk/blob/main/docs/GIGPIES_IMPLEMENTATION.md)
and [Lux owning plan](https://github.com/PaolaShultz/shr-lux/blob/main/docs/notes/0027-gigpies-implementation.md).

## Evidence and remaining work

The accepted normal Rust suites passed: GigPies **243**, Lux **89**, Desk **82**
and Lightdesk **81** tests, **495** total. GigPies also passed **37 Python tests**;
Lux passed **8 terminal checks**. All four passed formatting, warning-denied
Clippy and release builds. Focused checks cover callback allocations, contract
corpora, local transport, expired leases, stale confirmation and durable epochs.

The joint synthetic show used all four exact accepted executables concurrently.
Desk changed FOH while Lightdesk completed a Lux release. After the test killed
its own Lux child, Desk still changed monitor 1 without changing FOH. Restarted
Lux restored Hold/master/blackout at epoch 2 with null output disarmed and no
playback, grants or release transition. All test processes were joined and their
temporary endpoints removed. This establishes functional independence in the
bounded test, without making a combined-load or real-time scheduling claim.

Normal suites include fast production, contract, safety, recovery and focused
regressions. Accepted real-process checks additionally cover rendered audio edits,
monitor independence, preview/release, dropped replies with exact request retries,
retained state on reconnect, lighting fade samples and durable restart after two
crashes. Provider executable hashes and source manifests are retained in the
private coordination handoff; provider data fixtures remain in their owners.

Historical media, auditions, exhaustive matrices, long benchmarks, native display
acceptance, physical audio/MIDI/DMX and combined load were intentionally skipped.
Changed audio corpus generators were invoked explicitly, then remain opt-in.
The [remaining task dependencies](plans/MODULE_IMPLEMENTATION_MAP.md#dependency-order)
include native role binding, named analysis streams, owner PA/FX/REC integration
and production remote control. This milestone makes no new hardware claim.


The final workers exited successfully and reviewed source returned to each owning
repository without source commits or public publication. Licensed assets,
unrelated edits and earlier private evidence were preserved. The shared GigPies
build cache was reviewed and retained at about 10.2 GiB with 27 GiB free; deleting
it would discard reusable builds and shared evidence. Temporary integration
children and endpoints were removed, and no compiler/build slot remains held.
