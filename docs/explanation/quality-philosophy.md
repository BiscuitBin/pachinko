---
title: "Quality philosophy"
kind: "explanation"
audience: [contributor, maintainer, agent]
canonical_for: [quality_philosophy]
requires: []
---

# Quality philosophy

Gates encode decisions, not taste. Each one exists because someone decided something, and
the gate is what stops the decision quietly reverting. If a gate cannot be traced back to
a decision, it should be deleted rather than tolerated.

The argument on this page was carried from libpawdoku at `ccdd001`; the examples are this
repository's own.

## Checks are read-only

Every recipe under `just check` reports and never repairs. Recipes outside it do write —
`just format`, `just initialize` and the lock recipes among them — and `just fix` is the
one whose job is repairing what a check reported. `bg-project-check`, the runner behind
`just check`, enforces the split by snapshotting every path Git does not ignore and
comparing after every recipe, so a check that rewrites a file fails the run rather than
hiding drift.

This is why the pre-commit configuration is split in two. `.pre-commit-config.yaml` is
the gate and is what gets installed; `.pre-commit-fix.yaml` holds the mutating hooks and
runs only from `just fix`.

## A gate must be able to fire

A check that has never failed has not shown that it can. A ban with no fixture breaking it
proves nothing: it passes on clean code and on code it was never able to see alike, and
the two look the same in the log.

Gate 8, `just effects`, is where this matters most. It holds the rule that libpachinko
performs no effect itself ([decision 0008](../decisions/0008-std-library.md);
[Architecture](architecture.md) explains the boundary). `scripts/check-effects.sh`
carries a positive control for its crate bans: it fails if the tree it read does not name
libpachinko itself, because an empty answer would match no ban and pass. Its bans on the
standard library's clock, filesystem, environment, network, process and thread APIs are
clippy's disallowed paths, and a path clippy cannot resolve is only a warning, so a
misspelt ban would hold nothing. The probe is the control for both: a fixture under
`tests/fixtures/effects-violation/` breaks every ban exactly once, and the script fails
unless every ban fires there and none fires on `pachinko`.

## Fix the cause, not the report

A suppression is a last resort: one rule, one line, with a stated reason. Lowering the
coverage threshold, disabling a lint rule at a call site, or loosening an assertion until
it passes are all ways of deleting the signal while keeping the machinery.

Where a rule is genuinely wrong for this project, the fix is to configure it once, in the
config file, with a comment saying why. Here that file is the `[workspace.lints]` table in
the root `Cargo.toml`, with `clippy.toml` beside it: `multiple_crate_versions` is allowed
there because cargo-deny already owns that check and names the crates, and the comment
says so. At a call
site the only form is `#[expect(lint, reason = "...")]` on the item it names, never a
blanket `allow`, because an expectation that stops being met is itself a warning, so a
suppression that no longer suppresses anything reports itself.

## Unreachable is not untested

Coverage distinguishes two things that look alike. A branch no input can reach is not a
gap in the tests; it is code that should not exist, whether a bounds check after a
modulo, a null fallback after an exhaustive assignment, or a defensive default no caller
can trigger. Delete it rather than cover it with a contrived test.

The opposite case is code every run reaches but no unit test can call. The binary's
`main` is one: it is not excluded from the floor, it is run.
`crates/pachinko/tests/binary.rs` starts the built binary as a user would and asserts its
exit status and output, and because `just coverage` instruments that run, `main` is
covered by the test that proves it works rather than by an exemption that proves nothing.

The corollary: do not chase the last few percent. The floor is 90 per cent of lines over
`crates/*/src/**`, and while the code is small so is the denominator. Two crates hold a
handful of items between them, so one untested branch can breach the floor on its own:
keep each item trivially and fully tested until the specifications bring code. Doctests
are not counted, so an example is documentation and a test of its contract, but not
evidence for the floor.

## Tests inject, they do not stub

A fake is not a mock. No effect trait exists yet, but the fake that
[decision 0003](../decisions/0003-effects-behind-traits.md) puts beside each one must
behave: it gives the values the test chose, in the order the test chose, and reports when
the test asks for more than it supplied. Tests that assert a function was called are not
evidence that anything works.

## The specification is the arbiter

When a test and the code disagree, one of them is wrong and the specification says which.
Neither is adjusted until it passes.

## Related pages

- [Quality gates](../reference/quality-gates.md)
- [Testing](../reference/testing.md)
- [Specifications](specifications.md)
- [Architecture](architecture.md)
- [Test and debug](../how-to/test-and-debug.md)
