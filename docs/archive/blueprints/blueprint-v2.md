# GigPies — Project Blueprint v2

## 1. Core idea

GigPies is a modular live-show and venue automation system built around small Linux/Raspberry Pi nodes.

The live system is **not an AI mixer**. It is a deterministic **expert system**:

- measure
- analyze
- prepare
- apply bounded rules
- react only to meaningful exceptions

The goal is to prepare a strong show during soundcheck, then keep it stable.

Primary functions:

- automatic / assisted soundcheck
- live band mixing
- PA management
- monitor mixing
- intelligent lighting
- multitrack recording
- simple operator control
- performer self-service monitoring
- optional venue automation around the live system

The live-critical core must not depend on Internet access or external AI services.

---

## 2. Main architecture

### Pi 1 — Stagebox / Mixer Engine

This is the hard real-time audio node.

Responsibilities:

- physical audio I/O
- low-latency capture and playback
- channel routing
- input trim
- HPF / LPF
- PEQ
- gate / expander
- compression
- internal effects
- main mix
- monitor mixes
- PA routing
- crossover / speaker management
- per-output EQ
- delay
- limiting
- talkback routing
- final DAC routing
- live exception handling

This Pi owns the latency-critical audio path.

### Pi 2 — Brain / Show Engine

This is the analysis, control and show-coordination node.

Responsibilities:

- soundcheck analysis
- tempo / beat analysis
- key / chord / section analysis where useful
- instrument activity detection
- long-term loudness and spectral analysis
- masking analysis
- parameter calculation
- show-state logic
- intelligent DMX lighting
- moving-head scene logic
- operator UI
- generic MIDI controller support
- performer web UI
- multitrack recording to NVMe
- show metadata / timeline
- optional post-show automatic mix

The Brain sends parameters and decisions to the Stagebox/Mixer Pi. It is not inline in the critical audio path.

### Optional Pi 3 — PA / FX offload

Only added if profiling proves it is needed.

Possible responsibilities:

- heavier PA DSP
- additional effects
- FIR / convolution / oversampling
- non-monitor processing where a little extra latency is acceptable

It is not required for the minimum system.

---

## 3. Audio hardware target

Preferred reference setup:

- Behringer UMC1820
- Behringer ADA8200 over ADAT

At 48 kHz:

- 16 analog inputs total
- 18 analog outputs total
- one USB audio device presented to the Stagebox Pi
- one synchronized audio clock domain

The ADA8200 provides:

- 8 analog inputs -> ADAT OUT
- ADAT IN -> 8 balanced analog XLR outputs

### System standard

- 48 kHz
- 24-bit

48 kHz is more than enough for live PA and gives extra DSP/network headroom.

---

## 4. Output flexibility

The 18 analog outputs can be divided between PA, monitors and optional outboard routing.

Examples:

### 2.1 PA

- Top L
- Top R
- mono sub
- 15 outputs remain

### stereo tops + stereo subs

- 4 PA outputs
- 14 outputs remain

### 3-way stereo + mono sub

- 7 PA outputs
- 11 outputs remain

The system must not hard-code a monitor count.

PA management should support flexible configurations up to roughly **4x8-style routing** where hardware permits.

---

## 5. Preparation and calibration

GigPies assumes the venue and connected hardware are prepared correctly.

Before live use:

- interface input reference levels are checked
- output reference levels are established
- amps / powered speakers are set to known working levels
- PA routing is verified
- microphones and inputs are labeled
- monitor outputs are mapped
- loudspeaker protection limits are configured
- lighting fixtures are mapped once

The system should work from a known physical reference, not try to compensate for badly configured hardware.

---

## 6. Soundcheck philosophy

The heaviest mix intelligence happens **before the show**.

### Soundcheck flow

1. musicians plug into known/labeled inputs
2. monitor sound is available immediately
3. performers choose their role / monitor identity
4. the band plays one or more representative loud songs
5. the system records and analyzes every channel
6. PA may remain muted during the analysis pass
7. Brain calculates a polished baseline setup
8. Stagebox/Mixer loads the prepared show state
9. PA is enabled

### Analysis targets

- peak and average levels
- dynamic range
- noise / bleed
- frequency occupancy
- masking between instruments
- vocal intelligibility
- compression requirements
- EQ requirements
- gating / expansion requirements
- likely feedback areas
- effects requirements
- main and monitor balances

The goal is to make the show sound right **before it starts**.

---

## 7. Live-show philosophy

Once soundcheck is complete:

**Do not continuously remix the band.**

Normal show state remains essentially fixed.

Automation reacts only to meaningful deviations.

Examples:

- guitarist suddenly becomes +4 dB louder -> correct quickly
- unexpected clipping -> reduce safely
- feedback begins -> controlled corrective action
- pedal / instrument change causes a strong spectral shift -> bounded correction
- known song / scene change -> recall prepared state

### Three operating layers

1. **Prepared state** — created during soundcheck
2. **Normal live operation** — almost no intervention
3. **Exception handling** — fast, bounded corrections

The system should react in milliseconds to genuine extremes, not slowly play along with the musicians.

---

## 8. Monitor system

Monitor mixing stays on the Stagebox/Mixer Pi because monitor latency matters.

Sources may include:

- raw channels
- groups
- FX returns
- main mix
- click / cues
- talkback

Each performer gets an independent mix.

### Performer UX

Typical flow:

1. open local web page / scan QR
2. choose role: vocal / guitar / bass / drums / keys / etc.
3. system loads a sensible default monitor mix
4. performer gets a little “more me” by default
5. performer adjusts only a few simple controls

Possible controls:

- Me
- Band
- Vocal
- Drums
- FX

The performer should not need to understand buses, sends or routing.

### Performer talkback

A performer may press-and-hold talkback on the phone and speak to FOH/system control.

That talkback path must not reach the PA mains by default.

---

## 9. PA management

PA management is one of GigPies’ core functions.

Functions:

- crossover
- per-output EQ
- delay alignment
- polarity
- output gain
- RMS limiting
- peak limiting
- mute
- preset recall
- speaker protection
- optional RTA / measurement support

The PA engine should be conservative and deterministic.

---

## 10. Internal effects

Initial system assumes internal DSP effects rather than external outboard.

Examples:

- algorithmic reverb
- delay
- chorus
- simple modulation

Avoid expensive effects unless there is clear CPU headroom.

Semantic requests may be translated into bounded parameters:

- darker vocal reverb
- shorter decay
- tiny amount of chorus

The Brain interprets the request; the Mixer applies deterministic settings.

---

## 11. Intelligent lighting

Lighting is handled by the Brain Pi.

Hardware can be simple:

- USB uDMX-compatible dongle
- standard DMX fixtures

The system uses musical context rather than only stereo level.

Possible inputs:

- kick activity
- bass activity
- guitar activity
- vocal activity
- solos
- tempo
- sections
- energy
- mixer state

Examples:

- chorus -> wider/brighter scene
- guitar solo -> focus / movement toward guitar area
- vocal-only section -> calmer front-light focus
- song ending -> controlled fade / blackout

### Moving-head setup

The target is setup without requiring lighting-console knowledge.

Wizard:

- mounting: hanging / floor / side
- position: left / center / right
- facing direction
- confirm home / zero
- optionally point once at center stage
- optional semantic targets: vocalist, drummer, guitar L/R, audience center, back wall

The system works with concepts such as:

- focus = vocalist
- movement = slow sweep
- energy = low
- beam = medium

rather than exposing raw DMX pan/tilt values.

---

## 12. Network

Minimum two-Pi system:

- direct Ethernet cable between Stagebox Pi and Brain Pi
- fixed IP addresses
- no switch required

A separate Wi-Fi/network interface may be used for:

- performer phones
- venue web services
- Internet access
- optional outside inference/services

The show-critical Ethernet link remains isolated.

### Intended protocols

- dedicated 1 GbE
- multichannel audio transport: UDP/RTP-style uncompressed PCM where needed
- control/state: compact custom protocol
- timestamps / sequence numbers
- heartbeat / node status
- PTP / IEEE 1588 or equivalent timing where useful

Audio between Stagebox and Brain is mainly for:

- analysis
- recording
- metadata extraction

Latency-critical PA and monitor processing should remain local to the Stagebox/Mixer Pi whenever possible.

No dependency on QoS.

---

## 13. Operator control

No touchscreen requirement.

Preferred philosophy:

- normal HDMI monitor
- Rust TUI / minimal UI
- generic USB MIDI controller
- knobs / encoders for manual edits
- illuminated pads for pages, states, warnings and scenes

The system must not be tied to one controller model.

Possible pad states:

- green = normal
- yellow = attention
- red = problem / limiter
- blue = selected
- purple = automation
- blinking = action required

The system remains human-operated when desired, but requires little continuous attention.

---

## 14. Recording

The Brain Pi can use NVMe for multichannel recording.

Possible outputs:

- raw multitrack
- stems
- stereo board mix
- show metadata
- scene timeline
- song markers
- mix parameter history

### Post-show processing

After the live-critical work is finished, the system may optionally produce:

- automatic rough mix
- level balancing
- EQ / compression pass
- noise cleanup
- song splitting
- loudness normalization
- simple mastering

This is outside the live safety path.

---

## 15. Practical resilience

The goal is not aerospace-grade redundancy. The goal is to reduce ordinary live-show failure points to a sensible minimum.

Practical measures:

- known boot sequence
- saved show configuration
- cloned working system images
- spare Pi ready for either Brain or Stagebox role
- power banks used as small UPS units
- simple emergency mute / freeze control
- tested physical patching
- repeatable setup procedure

A venue system can still fail just like a conventional FOH mixer, amp or cable can fail.

GigPies should be no more fragile than normal live gear.

---

## 16. Testing strategy

Use prerecorded multitracks to repeatedly test:

- soundcheck analysis
- gain calculations
- EQ decisions
- monitor behavior
- exception handling
- lighting
- scene transitions
- recording
- reconnect behavior

The same material can be replayed thousands of times while developing.

---

## 17. First experimental performance

The first real performance should use the developer’s own band.

Advantages:

- known instruments
- known songs
- known stage volume
- known monitor preferences
- predictable changes
- easy rehearsal of problem cases

The entire show workflow should be tested before the public performance.

### Temporary proof-of-concept hardware

If full multichannel hardware is not yet available:

- Pi 4 may temporarily provide local analog output
- Bluetooth may temporarily feed another amplifier/speaker path
- Pi 5 runs Brain / control / lighting
- direct Ethernet between Pis
- generic USB MIDI controller
- HDMI monitor
- USB DMX dongle
- several DMX PAR cans

Bluetooth is demonstration-only because latency prevents proper phase alignment.

The prototype proves architecture and software behavior, not final latency performance.

---

## 18. Optional venue automation layer

This is outside the live-critical core.

Possible additional nodes/services:

- venue website
- band booking
- calendar
- rider upload
- venue-capability checking
- automated setup preparation
- ticket sales
- QR ticket validation at entrance
- guest list
- door payments
- bar/mobile ordering
- cashless/mobile payment integration
- show scheduling
- post-show recording delivery
- simple venue reporting

### AI use

AI may be useful here for:

- reading rider PDFs
- comparing rider requests with available venue gear
- summarizing missing requirements
- preparing band communication
- scheduling assistance
- document handling

AI remains outside the deterministic live-audio core.

---

## 19. GigPies venue story

A fully integrated venue flow could look like:

1. band books a date
2. band uploads rider and optional reference track
3. system checks what the venue can provide
4. venue setup is prepared
5. performers arrive and plug in
6. default monitor mixes appear
7. band performs soundcheck
8. GigPies prepares the FOH / PA / FX state
9. show runs with conservative live exception handling
10. lighting follows musical context
11. door/ticket systems operate independently
12. multitrack recording is saved
13. post-show recording can be delivered automatically

The system should remain modular: venues may use only the live core or add surrounding automation gradually.

---

## 20. Design principles

1. **Soundcheck prepares the show; live automation protects it.**
2. **Hard real-time audio stays local whenever possible.**
3. **The live core is a deterministic expert system, not an AI mixer.**
4. **Brain analysis may be heavy because it is outside the critical latency path.**
5. **No Internet dependency for the live show.**
6. **No unnecessary hardware lock-in.**
7. **Use inexpensive standard hardware where practical.**
8. **Human override is always available.**
9. **Lighting can be adventurous; audio must be conservative.**
10. **Add extra Pis only when profiling proves a need.**
11. **Calibrate the physical system once instead of compensating for bad hardware setup in software.**
12. **Reduce realistic failure points; do not over-engineer hypothetical ones.**

---

## 21. Current serious target

A serious first version can be built with:

- 1x Raspberry Pi 5 — Stagebox/Mixer
- 1x Raspberry Pi 5 — Brain/Show/Recording
- UMC1820
- ADA8200
- direct 1 GbE cable
- NVMe on Brain Pi
- HDMI display
- generic USB MIDI controller
- USB DMX interface
- DMX fixtures
- PA + monitors

That two-Pi core is enough to demonstrate the complete GigPies live-show concept.
