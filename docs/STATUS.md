# Product status

GigPies is experimental. The offline mixer and configurable live-engine software
are implemented; full-system physical and acoustic qualification remains open.
This page summarizes capabilities. Detailed contracts define behavior, and
[acceptance records](acceptance/README.md) define the scope of validation.

| Area | Implemented and validated scope | Remaining qualification or planned behavior |
|---|---|---|
| Configurable engine | Channel EQ/compression, buses, monitor sends, owner-native PA, raw recording and source-clock FX; software coverage at 16/32/48 inputs and a 17-input regression | Physical UMC1820 + ADA8200 socket mapping, shared ADAT clock lock and sustained deadlines |
| Channel and monitor control | Four-band parametric EQ, compression, authenticated control; independent raw post-mute, processed pre-fader and processed post-fader sends | Physical routing and listening acceptance |
| Brain audio | One local duplex owner, independent talkback and operator-monitor ASRC crossings, PTT expiry and safe recovery; integrated two-Pi software validation | Actual duplex device mapping, clock drift, acoustic delay and feedback/protection checks |
| PA control | Reviewed whole-configuration/output transactions and narrowly scoped live stereo master EQ | Acoustic protection and physical output acceptance; owner DSP remains in SHR PA |
| Measurement | Bounded raw reference/microphone-slot capture, owner analysis, multi-position proposals and exact candidate validation; Desk software integration | Actual microphone capture and acoustic alignment |
| Remote FX | Actual Brain stereo owner, prepare/permit/applied/settled states, tail-preserving bypass and history reset; Desk software integration | Physical listening and sustained hardware deadlines |
| Analysis and lighting | Named raw analysis, configured source binding, Lux authority and Lightdesk consumption with freshness checks | Physical configured publication awaits capture-age qualification; fixture/DMX output is unqualified |
| Operator consoles | Separate Desk and Lightdesk native frontends with real provider clients and CPU-headless rendering validation | Actual two-display/controller identity, LEDs and combined hardware load |
| Stereo USB bench | Bounded AudioBox USB 96 run with real PA/FX/REC; working-channel electrical delay 5.19–5.23 ms under the recorded H8 conditions | Two one-frame offset changes remain unresolved; weak right return prevents stereo physical acceptance |
| Offline mixing | Soundcheck/freeze/render, source preservation, bounded balance/tone/FX proposals, EQ matching and true-peak delivery | Musical preference requires listening; numerical eligibility does not establish a better mix |
| Performer review | Proposed station preferences, collective readiness and shared previews | Planned; not an implemented performer workflow |

## Capacity and clocks

The reference UMC1820 + ADA8200 system targets **16 analog inputs and 18 analog
outputs at 48 kHz**. Sixteen inputs is the minimum product target; tested 32/48-input
profiles are growth examples, not ceilings or physical qualification. Runtime
counts come from validated configuration and advertised capabilities. PA and
monitor output allocation remains flexible.

Stagebox and its ADAT expansion share the processing reference. DSP, PA, raw REC,
FX and source analysis follow Stagebox frames. Brain's local duplex device has a
separate clock and crosses through two bounded ASRC paths.

## Product documentation

- [Architecture and ownership](architecture/README.md)
- [Processing and protocol reference](reference/README.md)
- [Operating guides](guides/README.md)
- [Software and hardware acceptance](acceptance/README.md)
- [Planned performer review](architecture/PERFORMER_REVIEW.md)

Release changes belong in [CHANGELOG.md](../CHANGELOG.md). Historical execution
records and studies are retained in the [archive](archive/README.md); their old
instructions and measured scopes do not define current product behavior.
