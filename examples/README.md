# Examples

The first executable example is the CLI itself:

```sh
cargo run --locked -- inspect /path/to/original-wavs
```

Runnable analysis/mixing examples will be added with their implementation.
Examples and tests must not embed third-party recordings.

Selected-channel spectrum software proof: build `spectrum-source` under the shared
build lock with `CARGO_INCREMENTAL=0 cargo +1.97.1 build --locked --example spectrum-source`.
It accepts `PRIVATE_0700_DIRECTORY SHOW_UUID EPOCH tones|sequence`, opens only the
real local provider socket, and supplies deterministic PCM through
`LocalAudio::tick_with_capture`. `tones` is a steady FFT-bin sinusoid per input;
`sequence` cycles peak amplitudes 0.5, 0.125, silence and 0.25 every 1.5 source
seconds. It never opens PCM/MIDI or outputs audio. The regular provider and Desk
contain no synthetic spectrum generator. Use `preview_panel_providers.py desk
--channel --materials --spectrum --cpu-terminal --suffix channel` for actual Rio
proof; GP-PANEL owns the measured results and final screenshots.

The software-only `spectrum-source` example also accepts `hot`: a −1 dBFS peak
tone for inspecting input meter warning colours and actual post-processing
readback. `scripts/preview_panel_providers.py desk --channel --materials
--cpu-terminal --spectrum-hot` selects it; no physical audio device is opened.
