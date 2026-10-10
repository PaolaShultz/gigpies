# Actual remote FOH FX control — GP21-fx:1

This additive software path controls the actual Brain stereo owner processing
`foh-left`, `foh-right`. Each channel has independent delay, feedback, damping,
wet gain and bypass. It does not describe a generic rack or any audio device.
GP05 continues to report the unchanged local fixed fallback, including its
read-only parameter interpretation. PA remains after dry plus admitted wet;
raw recording branches before FX. Hardware acceptance remains separate.

## Admission and authority

The Brain runner's optional `fx_control_sha256` is the lowercase SHA256 of its
explicit `fx_library`. Omit it for existing media-only behavior. Resolve every
`shr_fx_delay_v2_*` symbol and exact capability/layout before advertising controls; old,
partial or incompatible extensions retain v1 media with unavailable GP21.
The published generic `shr_fx_v2_*` API has different signatures and is never
called through this source-timeline stereo-delay adapter. A library exposing
only that generic API retains v1 media; it does not advertise GP21 controls.
On Linux the loaded inode is the open file that was hashed. A replacement owner
needs a fresh authenticated media session. Media negotiation does not grant FX
write authority. Pair the actual owner and operator separately with explicit
`fx_configuration` permission; the operator must obtain a C-AUDIO version2
`fx_configuration` scope lease. Existing `fx` permission covers media only.

Use the standard eleven-key authority envelope, contract `GP21-fx`, version1,
module `audio`. `fx_snapshot` uses null writer, lease, request_id and
expected_revision, and an empty body. `fx_configure` uses the authenticated
writer and current lease/request/revision. Its body has exactly:

- `binding`: session, source_epoch, capability_generation, map_generation,
  owner_instance (decimal-string counters), library_sha256, abi_version2,
  config_size104, status_size168, capabilities_size128, and ordered
  channels `["foh-left","foh-right"]`.
- `expected_generation` and `expected_reset_count`: decimal-string counters;
  initial generation0 is valid.
- `lead_frames`: decimal string,48..48000 inclusive, divisible by48.
  Default operator policy is4800 source frames (100ms at48kHz).
- `configuration_json`: an opaque JSON string, or null for panic. At most4096
  bytes; exact object `{"channels":[LEFT,RIGHT]}`. Each channel has delay_ms
  1..500, feedback0..0.85, damping0..0.99, wet_gain0..1 and boolean bypass.
  All values must be finite; duplicate/unknown fields refuse.
- `panic_mask`: null for configuration, or integer1(left),2(right),3(both).
  Panic clears selected history and filter state; it is not persistent mute.

The integer-only outer codec is unchanged. Fractional DSP values occur only
inside the bounded opaque strings. The owner admits both channels atomically.
A same-session GP21 snapshot less than250ms old, exact binding/generation/reset
count, current actual-owner observation, settled owner and independent scope
are required. Neither GP05 nor ordinary mixer readback substitutes.

## Ticket and frame semantics

One mutation may be outstanding. Preparation reserves shared authority without
stalling dry audio or recording. Brain prepares the complete token off processing.
After PREPARED, the source owner revalidates lease, revision, owner and deadline,
commits authorization once at a source boundary and sends irrevocable PERMIT.
`permitted` means authorization intent only. It never means owner application.

Brain applies only immediately before the complete source block whose first
frame equals apply_frame. Late/missing permits, skipped target, source gaps,
stale generations and mismatched lifetime refuse; the frame never slides.
Preparation/ACK budget is2000ms measured by each node's own monotonic clock;
no raw monotonic timestamps cross machines. The lead policy is a software
acceptance bound, not a physical deadline qualification.

Configuration changes ramp/crossfade for960 source frames at48kHz. Readback
reports actual applied_source_frame, exact exclusive settled_source_frame,
applied/settled generations, next_source_frame, reset_count, remaining_frames
and target owner_json. Bypass ramps excitation down over20ms, then drains old
tails. Control settlement does not mean tail silence. Panic completion separately
pins its exact effective frame, selected mask and owner reset count increment.

Replies distinguish preparing, permitted, applied, settled, refused, cancelled
and unknown. Mutation replies include the original RequestContext, ticket,
apply_frame, current authority revision and actual owner observation. Completion
revision may exceed the authorization revision when another allowed control
commits after PERMIT; consumers must not assume exact expected_revision+1. Snapshot replies
include source show/epoch/revision/frame, availability, observation and pending
ticket. `authorization_replayed` is only the shared authority's historical
outcome, including after relay-history eviction. Its nested `applied` never
means DSP application. Read fresh actual-owner state to resolve uncertainty.

Cancellation is possible only before permit. A cancelled request consumes the
shared authority request ID without advancing revision. After permit, session
or ACK loss becomes unknown; never automatically resend, regrant or restore a
write. A new write needs fresh owner observation and explicit operator review.
The existing wet gate fences old sessions and fades to dry on return loss.
All token allocation/retirement, loading, hashing and JSON happen off the owner
process/commit boundary. Owner calls remain serialized.

## Validation

Normal focused tests cover strict parameter bounds, dedicated authorization,
freshness, binding, cancellation without dry interruption and lost-ACK unknown.
Explicit actual-owner tests additionally exercise independent channel samples,
exact960-frame settlement, panic, source ordering, legacy equivalence and old
library fallback. Run `cargo test --features hardware-host --lib gp21` normally.
Actual artifact tests require `GP21_FX_LIBRARY` and their documented `--ignored`
selection; corpus generation requires explicit `GP21_FIXTURE_OUTPUT` as well.
These tests are synthetic and never open a physical device.
