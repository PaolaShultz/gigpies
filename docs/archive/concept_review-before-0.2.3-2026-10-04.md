# Concept artwork review

The supplied AI-generated [architecture image](assets/gigpies-concept.png) is preserved
unchanged. It communicates two nodes, local stage audio, a separate Brain, performer
controls and a soundcheck-first workflow. It is conceptual artwork, not a wiring guide
or evidence that the pictured product exists.

Corrections and caveats:

- The Stagebox box says “soundcheck scene generation.” The Brain calculates the
  prepared state; the Stagebox validates and applies it.
- Workflow step 4 says “AI-assisted analysis.” The live design is a deterministic
  expert system. External AI is optional outside the live-critical path.
- RTP/UDP and PTP are pictured as decided protocols. They remain candidates;
  source sample timing and USB clock ownership need an explicit implementation.
- The ADAT drawing compresses bidirectional audio into one arrow. Audio expansion
  needs the actual directional optical connections and a deliberately configured
  clock master. USB from interface to Stagebox is also not clearly drawn.
- Generated device names, panels and ports are stylized/inaccurate. Use real device
  documentation and physical testing for patching and channel availability.
- “4×8” PA routing is a future GigPies ambition. Existing SHR PA implements fixed
  2-input/6-output processing; the image must not imply a ready 4×8 engine.
- Muted-PA soundcheck cannot establish acoustic response or feedback behavior.
  Speaker protection, level calibration and room/setup measurements are separate.
- Outboard and recording graphics do not specify tap positions or channel ownership.
  Internal FX are the initial direction; Brain recording needs an explicit audio feed.
- “Unattended venue support” and “consistent sound” are ambitions, not acceptance
  claims. Human override and clear faults remain requirements.

The artwork includes rendered third-party product names. It implies no endorsement
and should not be used as an exact installation diagram.
