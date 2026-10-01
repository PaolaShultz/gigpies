# Local multitrack material

Found in the development workspace's `../shr-lux/recordings/downloads/`:

| Archive | WAV files | Original format | Longest file |
|---|---:|---|---:|
| `Complainiacs_Etc_Full.zip` | 13 | 24-bit PCM, 44.1 kHz | 114.242 s |
| `DarkRide_HammerDown_Full.zip` | 40 | 24-bit PCM, 44.1 kHz | 300.365 s |

Counts and formats were read from local ZIP entries and WAV headers. A stereo WAV
counts as one track file but contains two audio channels. Different end lengths are
present; duration alone does not prove sample alignment or correct placement.

The source archive readmes identify the material as educational-use recordings and
require copyright-holder permission for commercial use. Retain those readmes with
local files. Source library: Mike Senior's Cambridge multitrack library, as recorded
in the existing SHR Lux provenance. This project includes no music or source archives.

## First session: The Complainiacs — Etc

- Kick, snare, stereo overheads, stereo drum room, three toms.
- Bass DI and bass amp.
- Two electric guitars.
- Lead vocal and vocal room.

Use the original 13 WAVs for mixing. SHR Lux's prepared four-source lighting simulation
contains kick, bass and two guitars at 16-bit/48 kHz with fixed attenuation; it is not
a complete-band mixing source.

Local preparation locations:

- `recordings/downloads/`: optional cached original archives.
- `recordings/sessions/complainiacs-etc/`: original extracted WAVs and source notice.
- `sessions/local/`: inventories and later private experiment manifests.
- `artifacts/`: generated measurements and mix comparisons.

All are ignored. No auto-download runs in builds, tests or CI. Keep original files
unaltered; derived audio goes to a separate destination. Future preparation must
preserve sample zero, stereo relationships and relative levels, and explicitly record
padding or resampling. Verify alignment before interpreting cross-track measurements.
