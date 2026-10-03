# Shared local media

The machine's source library is `../waves`, outside the GigPies and
shr-lux repositories. From either sibling project, the relative path is `../waves`.

| Content | Location from GigPies |
|---|---|
| Loose user-supplied WAVs | `../waves/*.wav` |
| Original multitrack sessions | `../waves/recordings/sessions/` |
| Acquisition provenance, hashes and notices | `../waves/recordings/provenance/` |
| Retained acquisition material | `../waves/recordings/downloads/` |
| Shared multitrack catalogue | `../waves/source-library.json` |
| Loose-track catalogue | `../waves/loose-tracks.json` |
| Relocation audit | `../waves/relocation-manifest.json` |

GigPies' ignored `recordings` symlink points to `../waves/recordings`. Existing
session configurations and historical scripts that use `recordings/sessions/...`
therefore continue to work. `sessions/local/source-library.json` and
`sessions/local/SOURCE-LIBRARY.txt` link to the shared catalogue, whose input paths
are now absolute and independent of a project's working directory.

Former loose WAV paths in the project root have moved. New reference-review commands
should use the shared path, for example:

```sh
target/release/gigpies reference-review NEW_VERIFIED_RENDER/processed.wav \
  '../waves/The Complainiacs - Etc. [IW0QixgOylk].wav' new-review 24
```

Historical reports retain their original recorded paths. The shared relocation
manifest maps those names to their new locations; active local reference provenance
also records the current path. Generated mixes, excerpts and experiment evidence
remain under this project's ignored `artifacts/` directory. On 2026-10-03 the user
retired all existing generated renders; the originals and recorded evidence remain.
Create new listening exports after the [mixing-engine work](SUMMING_PLAN.md), and
use a verified new render path in the example above.

## Relocation validation, 2026-10-02

Moved 267 loose WAVs and the complete recording library: 165 files, including 122
WAVs, original notices and provenance. Total relocated size: 18,447,740,373 bytes
(about 17.2 GiB). Every file retained its filesystem device/inode, size and nanosecond
modification time. Same-filesystem renames avoided copying or rewriting audio.
Original filenames, timing, formats and stereo relationships were preserved.

Validated every catalogue destination, the old/new Complainiacs inventory paths,
and the relocated produced-reference file. This was a storage/path change; DSP
code and rendering behavior did not change, so no engine suite or audio render was
needed. No playback, host-audio change, sibling-project modification or push occurred.
The new directory is immediately accessible to shr-lux without changing that project.
