# Maintained scripts

These are reviewed reusable tools. `publication-policy.json` is the explicit
publication list; `check_publication.py` checks Git contents before publication.
Put one-off session/render runners and generated results under ignored `artifacts/`.

| Scripts | Purpose |
|---|---|
| `analyze_mix.py`, `bass_evidence.py`, `drum_evidence.py` | Optional analysis and summaries of supplied audio or saved measurements |
| `balance_excerpts.py`, `listening_checkpoint.py` | Exact parent-sample excerpts and verified listening checkpoints |
| `verify_delivery.py` | Independent final PCM true-peak verification with local FFmpeg/libsoxr; no playback or audio output |
| `ensemble_review.py`, `snare_context.py` | Descriptive ensemble and source-context review |
| `preset_experiment.py` | Explicit historical preset experiments; no automatic media downloads |
| `snare_bleed_evidence.py`, `snare_temporal_evidence.py` | Opt-in saved-evidence audits; no processing selection or playback |
| `play_pair.py` | Explicit two-clip playback with preflight; never run by normal tests with a real player |
| `check_publication.py` | Git index/outgoing-history checks for private/generated content |
| `test_*.py` | Fast synthetic regressions; normal suite, no physical audio devices |

`analysis-requirements.txt` contains optional plotting/analysis dependencies.
Normal Python tests use the standard library:

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
```

See [publication rules](../docs/PUBLICATION.md) before adding or changing scripts.
