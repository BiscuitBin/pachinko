---
name: review-change
description: Review a change in this repository against its invariants, specifications, tests, and documentation contract.
---

# Review a change

1. Read `AGENTS.md` and the specification module the change touches. Review the whole diff, not the summary of it.
2. Check the nine invariants in `AGENTS.md` one at a time: specs decide behaviour; libpachinko performs no effect itself; `Send + Sync + 'static`, `Clone`, `Debug` and `#[non_exhaustive]` on every public type; no `unsafe`; `pachinko` depends on `libpachinko`, never the reverse; pedantic clippy with `expect` only; the coverage floor intact; `Cargo.lock` the pin with caret ranges and `--locked`; no dependency without the decision-0007 note.
3. Check the change against the specification it implements. A rule, guard or threshold decided in code rather than in `docs/specs/` is a finding even when the behaviour looks right.
4. Check the test evidence. A test that only asserts a function was called is not evidence; a test that supplies values through a fake and asserts what the library did with them is. A public item without a doctest that asserts is a finding.
5. Check the boundaries. libpachinko reaches the clock, randomness, the filesystem, the network, the environment and threads only through its own traits; the real implementations live in pachinko; no test replaces a trait with a global or a thread-local. Run `just effects`, and read libpachinko's diff for a direct `std::fs`, `std::time`, `std::env`, `std::net`, `std::process` or `std::thread` call that its std-API bans miss: one newer than the list in `crates/libpachinko/clippy.toml`, or one behind another target's `cfg`.
6. Check documentation ownership. A durable fact belongs on the page that owns its topic in `docs/manifest.yml`, added there rather than restated.
7. Check scope. Unrelated refactors, new dependencies and speculative abstractions are findings in themselves.
8. Report findings by severity with `file:line`, what breaks, and the smallest fix. Run `just check` before concluding.
