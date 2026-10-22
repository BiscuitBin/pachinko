---
name: rust-change
description: Implement or review a change to either crate with the effects boundary, tests and documentation evidence the invariants require.
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the procedure for implementing or reviewing a change to either crate. It will cover the two crates, `crates/libpachinko/` and `crates/pachinko/`, and the rule that an effect trait goes in libpachinko, with its fake beside it, while the real implementation goes in pachinko.

The recipes it will name include `just clippy`, `just test`, `just effects` and `just coverage`. Run the narrowest of them while working, then `just check` before handing the work back.
