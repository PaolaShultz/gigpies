# Configured named raw analysis

C-ANALYSIS:2 is an additive configured attachment for the existing four sources,
ordered `kick`, `bass`, `guitar-1`, `guitar-2`. It does not introduce another mixer,
analyzer, source clock or device owner. Version 1 and its original fixture/request
bytes retain their exact semantics.

The strict startup mapping is `{"version":1,"inputs":["input-17","input-03",
"input-09","input-01"]}`. All four entries must be distinct canonical logical IDs
present in the admitted topology. IDs follow the existing unsigned 16-bit logical
port domain; this is not a claim that that many strips meet resource admission.
Aliases such as `input-001`, physical USB slots and measurement-only slots are
refused. The producer resolves IDs through its validated logical-strip inventory;
capture-slot permutations do not change the meaning of a logical source.

`gigpies-headless --analysis-map ABS_JSON` selects configured analysis independently
of `--synthetic-source fouraux`. Its existing `--analysis` flag retains the original
explicit fouraux path and cannot be combined with `--analysis-map`. Map JSON is
bounded to 4096 bytes, with duplicate/unknown fields rejected. The authenticated
provider run configuration accepts optional `analysis_mapping` with the same
schema, publishing from its existing local source owner into the same private
directory. This publisher is local same-UID IPC: monotonic acquisition ages are
same-host values, not a cross-node clock synchronization protocol.
The runner rejects `analysis_mapping` combined with `physical_device` before
device, identity-file, credential or listener side effects. Physical capture does
not yet propagate converter/buffer acquisition age; labeling such buffered data
fresh would be incorrect. This admission remains software-only until that
separate timing boundary is implemented and accepted.

Admission occurs before the first processed frame. The descriptor retains its
original field set, with `version:2`, `subscription:"lux.aux.v2"`, actual source
epoch, first frame and topology map revision, and positive calibration revision.
The exact framed subscription request is
`{"subscription":"lux.aux.v2","version":2}`. Each endpoint admits only its own
version, without negotiation or silent downgrade. Version 1 clients cannot receive
version 2 data under a legacy identity. GAW1 and PCM packet layout remain unchanged:
four PCM24 channels, ten 48-frame packets per window, source-frame sequence,
two-window staging, whole-window loss counts, and the existing 100 ms acquisition
age bound. Rendering, FOH controls and incoming packet arrival cannot advance the
analysis timeline independently of the source owner.
The software source timestamp is taken when the provider receives the borrowed
block, before controller work, and retained through publication. It does not
establish the age of samples buffered by an external/physical caller.

The tap reads raw logical input before mute, fader and processing. Float input is
rounded to signed PCM24 only within [-1, 8388607/8388608]. Nonfinite/out-of-range
samples, timestamp/encoding failures, source quiescence or changed topology map
stop the publisher; they cannot leave a live publisher with a stalled cursor or
silently relabel queued samples. No thread join/retirement is introduced into
the processing step: an atomic stop is requested and ownership is retired at
teardown. Analysis failure does not stop otherwise valid mixer processing.
Source recovery uses a fresh epoch and drops the old attachment. Mapping selection
requires an explicit fresh provider startup; no calibration or AUTO grant is
restored or inferred from retained values.

Consumer admission must validate the exact expected ordered configured mapping
on every connection before using the descriptor. A source/map/epoch change must
invalidate calibration and grants. Lux owns analysis and lighting decisions;
Lightdesk owns complete operator review and must validate both current descriptors
and retained contribution provenance. The corresponding Lux/Lightdesk configured
schema/corpus must be explicitly accepted before claiming consumer compatibility.

Normal production checks cover nonfixture 17-input and 32/48-input mappings,
physical-slot permutation, exact raw PCM, wrong-version refusal, topology change,
invalid mapping/sample input, source progression and allocation-free tap work.
The fixtures under `tests/fixtures/gp04/v2` are producer-checked. They do not prove
physical I/O, microphone mapping, lighting output or whole-host deadlines.
