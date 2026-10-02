# Musicians review the mix after soundcheck

**Planned, not implemented or hardware-verified.** Recorded from the user's idea
on 2026-10-02, after the [source-preservation reassessment](SOURCE_PRESERVATION.md).
This extends the existing [QR station workflow](ARCHITECTURE.md#proposed-instrument-station-soundcheck).

After soundcheck and the expert system's first pass, musicians should be able to
review their own sound through the phones already connected to their stations.
Their preferences are part of the musical evidence. The system's first pass can
also preserve a good sound unchanged; it need not make adjustments to earn review.

## Proposed flow

1. **Offer the prepared mix.** Each musician sees their assigned instrument and
   the current preview, with a short account of any proposed changes. The existing
   unique station link identifies which guitar, vocal or other instrument they own.
   The Brain enforces that assignment across the instrument's verified input group;
   a display name or a claimed track number does not establish ownership.
2. **Collect preferences.** “Keep my sound” is a complete response. Other choices
   use simple controls with a neutral position and reset: for example, a singer
   requests brighter reverb and a little chorus, while one guitarist requests a
   darker guitar. Display the chosen scope: instrument sound in the shared mix or
   the musician's personal monitor. Existing monitor controls remain available.
3. **Mark requests ready.** Ready means that musician's current requests are ready
   to include in a preview. Editing those requests clears their ready state. Show
   whose response is still pending without discarding anyone else's work.
4. **Prepare one shared revision.** Once all participating musicians are ready,
   freeze their requests with the current soundcheck capture and mix settings.
   Apply bounded, deterministic proposals and render a common ensemble preview.
   Shared effects or interacting requests need an explicit resolution when they
   cannot both be honoured; submission order must not silently decide the result.
5. **Listen on phones/headphones.** Everyone receives the same revision, with the
   previous version available for comparison. This is a recorded preview, separate
   from the live monitor path. Phone/headphone playback cannot establish PA/room
   sound, feedback stability or acoustic safety. Synchronous playback is not needed
   for the first version, and phones must not start playing automatically.
6. **Accept or revise.** Retain the accepted settings and each musician's preferences.
   Show any request that was withheld and why. Further edits produce a new revision;
   acceptance of an older preview does not transfer automatically. Applying a chosen
   revision to the live system is a separate, clearly labelled operator action.

The system must assess the ensemble after instrument edits. A darker guitar can
lose articulation; brighter reverb can obscure vocals; chorus can change stereo
relationships. A musician's request supplies intent, but does not prescribe an
unbounded knob move or guarantee that a particular processor will help. Keep the
unchanged comparison eligible and keep makeup separate from artistic balance.

## Deterministic first version

Structured preferences and inspectable rules are sufficient for the first version;
AI is not a dependency. “Brighter reverb” addresses the selected vocal reverb path,
not an implicit brightening of the dry vocal or the whole shared return. If that
reverb does not exist, make the proposed addition explicit. “A little chorus” can
offer a bounded vocal effect preview without inserting effects on other instruments.
If shared routing prevents independent control, explain the coupling before applying
the request. Exact control ranges and mappings need separate DSP and listening tests.

Preserve the last accepted mix if preview generation fails, the Brain restarts or
a phone disconnects. A disconnect does not mean ready or consent. Restore submitted
work on reconnection; the operator can visibly change the participant list when a
musician leaves. Keep requests, readiness, preview identity and application status
distinct so no one waits for an obsolete revision. Retrying a failed render uses
the same frozen requests unless someone explicitly edits them.

Controls need readable labels, keyboard and assistive-technology access, adequate
touch targets and status beyond colour alone. No gesture, fine slider movement or
audio notification should be the only way to submit, reset or learn what is pending.
These are proposed requirements; no performer usability study has been run.

## Proposed tone maps and an amount slider

Follow-up user proposal, 2026-10-02: choose a sound such as “dark metal guitar” in
the planned HTML interface, calculate an EQ toward that reference and let the player
control how much is applied. This supplies an explicit artistic destination and
fits the preservation contract. It does not require declaring the starting sound
defective. The offline [EQ matching backend and local review page](EQ_MATCHING.md) now
implement versioned maps, bounded fitting, amount and reset. The multi-musician
network workflow remains planned.

The underlying technique is matching EQ: compare a measured spectrum with a stored
reference and calculate a filter response. Established implementations support saved
spectra and fitted EQ bands ([FabFilter documentation](https://www.fabfilter.com/help/pro-q/using/eqmatch));
amount and smoothing are also established controls ([iZotope documentation](https://s3.amazonaws.com/izotopedownloads/docs/ozone9/en/match-eq/index.html)).
Those implementations establish the technique, not the quality of our future profiles.

Proposed GigPies behavior:

- Build profiles from documented, comparable instrument examples and musician
  audition. Keep capture type, tuning/register and playing technique with each
  reference. A style name is a selectable example of intent, not a universal genre
  standard. Use a useful tonal range with uncertainty, rather than treating each
  spectral peak as a target.
- Collect representative soundcheck phrases. Use a smoothed spectral envelope over
  frequency, with separate attack/sustain observations where supported. A spectrogram
  can help inspect the capture; matching its individual note trajectories would
  confuse performance differences with tone. A single held note is insufficient
  evidence for a general instrument voicing.
- Compare spectral shape after removing a common level offset from the analysis.
  This is a measurement operation; it must not normalize source files, move the
  artistic fader or introduce hidden makeup. Do not boost a noise-only or unmeasured
  frequency region to fill a reference gap.
- Fit a small, bounded set of broad EQ bands to the supported difference. Keep the
  current sound if the difference is already within the intended range or evidence
  is insufficient. EQ can change tonal balance; it cannot reproduce a different
  distortion process, player, amp dynamics or reverb using magnitude matching alone.
- **0% means exact bypass of this proposed correction. 100% means the full bounded
  proposal**, with intermediate amounts scaling its band gains in dB. For example,
  a proposed −4 dB shelf becomes −2 dB at 50%. This is an EQ amount control, distinct
  from an audio dry/wet blend. Preserve the accepted baseline and calculate every
  slider position from it so repeated changes do not accumulate processing.
- Show the resulting EQ and its scope. Recheck actual production DSP at the selected
  amount in the ensemble, including attack/body, coherent input groups, stereo and
  headroom. A lower spectral error cannot establish preferred sound. Keep the
  reference capture and settings frozen for a preview; do not chase every new note
  during performance. Live parameter smoothing would need separate validation.

The existing tone pass only supplies a limited body/presence feature and explicit
profiles. Extending it to this proposal needs reference curation, filter fitting,
production-DSP tests and musician listening. Regression cases should include exact
0% preservation, an already matching source, known injected coloration, differing
notes/tunings, silent or noisy bands, repeated slider moves and interacting inputs.
No target curves or preferred amount have been selected in this discussion.

## Optional small-model exploration

The user also suggested running a small local model during the quiet period after
soundcheck. A future experiment could translate free-text preferences into the same
bounded controls, showing its interpretation before submission. Ambiguous requests
need clarification, and the model must not infer instrument ownership, prior source
processing or approval. The deterministic path remains usable without it.

Run any such work on the Brain, outside the live audio callback. A pause in analysis
does not mean the whole system is idle: audio, monitoring and protection may still
be active. Model choice, memory/CPU cost, cancellation and available compute need
measurement on the intended hardware before scheduling inference. No model choice,
latency or hardware suitability is established here.

## Verification before implementation is accepted

Cover one and several musicians, two distinct guitar stations, “keep my sound,”
grouped inputs, shared-effect conflicts, edits before/after ready, stale preview
acceptance, disconnect/rejoin, failed render/retry and an explicitly changed
participant list. Verify that another instrument cannot be edited through a station
link, monitor changes do not silently alter the shared mix, and a preview cannot
apply itself to the Stagebox. Check keyboard/screen-reader/touch paths and retention
of preferences across interruptions. Then test the musical proposals through
production DSP and musician listening; neither a passing rule nor a phone preview
establishes hardware acceptance.
