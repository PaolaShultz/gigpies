# GP-01 synthetic provider corpus

Contract GP-2026-10-04.1, C-SHOW:1/E01 and C-ROLE:1/E02.
Provider baseline: 99eb0f08050a9a86039af9f1ba2a3541649ca7b6 plus
uncommitted GP-01 source identified in the private Lane A handoff. No hardware IDs
or engine state are represented. Content identities are invented synthetic hashes.

`e01.json` is the prior manifest. Input capabilities audio/C-AUDIO:1 and
lighting/C-LIGHT:2 must produce read_only=true and lighting reason Version;
manifest bytes and both engines remain unchanged. Matching version 1 capabilities
produce compatible metadata only, never recall/arm/start/grant. Missing required
lighting refuses attachment. `tests/gp01.rs` exercises the provider implementation.

`e02.json` is the prior registry. Claim lighting-desk/test-controller-a at generation
7 returns conflict with unchanged prior state. Release audio-desk/test-controller-a
at generation 6 returns stale_generation with unchanged prior state. Two displays
with invented connectors a/b and EDID same yield no automatic binding.

Reproduce hashes: `sha256sum tests/fixtures/gp01/e0{1,2}.json`.

- E01: 5817a494339b4591077ab62312e25d4f1cb6826f2fcd7f9cca8f99a0a9175656
- E02: 055c8d501cde17fcc168fd83071f53c6f731d32f0554bdc4846de056f75a4f5b

Role claims here are generation-checked pure intended state. GP-09 must add verified
native identity/profile checks and process-held per-device locks; saved bindings
alone do not establish live exclusivity. Persistence requires a trusted owned 0700
Linux directory and writes create-new 0600 temporary files, syncing before rename
and syncing the directory afterward. An error after replacement explicitly reports
uncertain durability. No migration is supplied; future schemas are refused.
