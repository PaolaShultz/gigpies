# Shared software build coordination

Task plans/progress live in [the integration plan](MODULE_IMPLEMENTATION_PLAN.md)
or their module owner. This document owns build coordination only; it does not
assign workers, models, repositories or task states. The user controls delegation.

## One build slot per host

Serialize Cargo build/check/test/Clippy/linker work across these runtime owners
using `/home/shome/p/.gigpies-build.lock` on each host. Use the owning pinned
Rust toolchain, locked dependencies, `CARGO_INCREMENTAL=0` and jobs=1.
A parent must hold the lock until the entire child build/check sequence finishes.
Use a nonblocking lock: if occupied, report the resource and continue scoped source/
documentation work; never bypass or terminate another session's build.

Example for an authorized GigPies production suite:

```sh
flock -xn /home/shome/p/.gigpies-build.lock env CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
  cargo +1.97.1 test --locked -j1 --all-targets
```

This example is not build authorization for planning/documentation work. Check live
host/free-space/target ownership before substantial builds. Existing targets and
interactive sessions are preserved; resource pressure does not authorize cleanup
or weakening validation. Follow the stricter changed-owner test/publication policy.
Physical, network-load, audible and long campaigns need their own finite scope and
resource reservation. Runtime Stagebox/Brain assignments follow observed evidence,
not development lane names.

[Historical four-lane assignments and launch cards](archive/tracking-before-2026-10-09/PARALLEL_WORK_PLAN.md)
are retained for provenance and are retired as executable instructions.
