---
title: "Make your first change"
kind: "tutorial"
audience: [contributor, agent]
canonical_for: [first_change_tutorial]
requires: []
---

# Make your first change

From a fresh clone to a green gate, through one small change you then put back. The
first run downloads the pixi environment, a Rust toolchain, the tools `tools.txt` and
`tools-source.txt` name, the Allium checker and the hook environments, so it needs the
network, and it needs rustup, pixi, `just` and a C++ compiler installed first;
[Develop locally](../how-to/develop-locally.md) describes all four.
The change is small on purpose; what matters is that it passes through every layer the
repository has today: an item, the tests that state what it does, and the gate.

Start from a clean worktree: `git status --short` prints nothing. The last step puts the
two files it changes back with `git restore`, which would also throw away any edit of
yours already in `crates/pachinko/src/lib.rs` or `crates/pachinko/tests/binary.rs`.

## 1. Get the workspace running

```console
just initialize
just check
```

`just initialize` installs the pixi environment from `pixi.lock`, installs the toolchain
`rust-toolchain.toml` pins, refuses a cargo that is not rustup's, installs cargo-hack,
cargo-release, cargo-fuzz and rustqual into `.tools/bin` and then the Allium checker,
syncs all three lockfiles and installs
the pre-commit hook, from the primary worktree only; a secondary worktree says so and
skips that step. It never formats, stages, commits, tags or pushes. If it complains about
a missing tool, read [Develop locally](../how-to/develop-locally.md).

`just check` should end with this line. It is your baseline:

```text
All checks passed and the worktree is unchanged.
```

## 2. See the crate

```console
just doc
```

Open `target/doc/pachinko/index.html` and follow `run`. Its source is
`crates/pachinko/src/lib.rs`. `run` takes the command line, without the program name,
and a writer: with no argument it writes the line `pachinko <version>` to that writer,
through the private helper `version` beside it, and `serve` starts the HTTP service
instead. The version is `libpachinko::version()`, the library's manifest version. The
binary's `main`, in `crates/pachinko/src/main.rs`, calls `run` on the arguments and
standard output and nothing else. `run` carries an example, and `just test-doc` runs
every such example as a test. Your exercise changes the line `run` writes with no
argument.

## 3. Read what decides the behaviour

Open `docs/specs/pachinko.allium`. It is the root module, and it specifies the queue
the library implements and the service serves: its entities, rules, surfaces and
invariants. A change to any of that is stated there before any code implements it. The
version line is not the domain, though; it is the service's scaffolding, so no clause
decides it. Two tests and one example state it instead:

- `run_writes_the_name_and_the_library_version`, the unit test at the foot of
  `crates/pachinko/src/lib.rs`, calls `run` on a `Vec<u8>` and compares what it wrote.
- `the_binary_prints_its_line_and_exits_zero`, in `crates/pachinko/tests/binary.rs`,
  runs the built binary as a user would and compares its standard output, its standard
  error and its exit status.
- The example in `run`'s doc comment calls `run` with no argument and asserts the same
  bytes; `just test-doc` runs it.

Read the three expected lines now. Each builds the line from `libpachinko::version()`,
`format!("pachinko {}\n", libpachinko::version())`, so the tests hold whatever version
the manifest states and the exercise works the same on any `main`.

## 4. Change something

In `crates/pachinko/src/lib.rs`, change the format string in `version`, the helper
`run` calls with no argument, so the line reads `pachinko version <version>`. The longer
line no longer fits rustfmt's width of 100, so write it as rustfmt would, or `just fix`
will:

```diff
 fn version(out: &mut impl Write) -> Result<(), Error> {
-    writeln!(out, "pachinko {}", libpachinko::version()).map_err(Error::io("writing the version"))
+    writeln!(out, "pachinko version {}", libpachinko::version())
+        .map_err(Error::io("writing the version"))
 }
```

Leave the tests alone and run the suite. nextest should fail in two places, because two
tests state the old line; the example on `run` states it too, but `just test` reaches
the doctests only once nextest passes.

```console
just test
```

Each failure is an `assert_eq!` on bytes, so the assertion prints both sides as lists of
numbers: `left` is what was written, `right` is what the test expected. The numbers are
the bytes' values; 32 is a space and 10 the newline. When this page was written, at
version 0.1.0, the unit test reported:

```text
thread 'tests::run_writes_the_name_and_the_library_version' (316050970) panicked at crates/pachinko/src/lib.rs:26:9:
assertion `left == right` failed
  left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
 right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
```

`118, 101, 114, 115, 105, 111, 110, 32` after the first space is `version` and its
space. The binary test fails with the same two lists at
`crates/pachinko/tests/binary.rs`, and nextest's summary names both. The counts and the
line number below date from before T20 added the queue's and the service's tests; today far more tests run, and the two `FAIL` lines are the ones to find:

```text
     Summary [   0.205s] 3 tests run: 1 passed, 2 failed, 0 skipped
        FAIL [   0.010s] (2/3) pachinko tests::run_writes_the_name_and_the_library_version
        FAIL [   0.205s] (3/3) pachinko::binary the_binary_prints_its_line_and_exits_zero
error: test run failed
```

You may see only one failure. `.config/nextest.toml` sets `fail-fast = true` for the
default profile, so nextest stops starting tests after the first failure. Both ran here
because they had already started together, and the output between them says so:
`Cancelling due to test failure: 1 test still running`. On a machine with fewer cores the
second may never start. To see every failure at once, run nextest without fail-fast;
`pixi run` puts the pinned nextest on `PATH` as the `Justfile` does:

```console
pixi run --frozen cargo nextest run --workspace --all-features --locked --no-fail-fast
```

`pixi run --frozen cargo nextest run --help` lists the flag. Or fix the failure you see,
run `just test` again, and the second appears.

A change that no test notices is either already covered or untested. Here the failures
are the point: they show the line is stated in two places, and where.

## 5. Change the tests to match

Update the expected line in both tests and in the example, inserting `version` and a
space after `pachinko`. In the unit test in `crates/pachinko/src/lib.rs`:

```diff
-            format!("pachinko {}\n", libpachinko::version()).as_bytes()
+            format!("pachinko version {}\n", libpachinko::version()).as_bytes()
```

In `crates/pachinko/tests/binary.rs`:

```diff
-        format!("pachinko {}\n", libpachinko::version()).as_bytes()
+        format!("pachinko version {}\n", libpachinko::version()).as_bytes()
```

Back in `crates/pachinko/src/lib.rs`, in the example in `run`'s doc comment:

```diff
-/// assert_eq!(out, format!("pachinko {}\n", libpachinko::version()).as_bytes());
+/// assert_eq!(out, format!("pachinko version {}\n", libpachinko::version()).as_bytes());
```

Run `just test` again. Every nextest test passes, then the doctests. In a real change,
the prose of `run`'s doc comment, which names the line, would change in the same commit
too: the behaviour, its tests and its documentation land together. For a behaviour the
specifications define, the clause moves first and the test is named for the clause it
proves.

## 6. Run the whole gate

```console
just check
```

It runs every recipe in order and proves the run did not modify the worktree. It should
end, as your baseline did:

```text
All checks passed and the worktree is unchanged.
```

Read only the first failure; the later ones are often consequences. If a gate fails, do
not work around it — [Troubleshooting](../operations/troubleshooting.md) covers the
common causes.

## 7. Put it back

This was an exercise, so do not commit it. Restore the two files it changed and confirm
the worktree is clean again:

```console
git restore crates/pachinko/src/lib.rs crates/pachinko/tests/binary.rs
git status --short
```

For a real change you would commit instead. The pre-commit hook runs the fast,
read-only part of the gate again. It does not run clippy, the tests or coverage, which is
why `just check` comes first. Nothing is pushed until you ask for it.

## What you just touched

An item, the tests and the example that state its output, and the gate. That is the
whole loop; every change after this one is the same shape, and a change to the queue's
behaviour starts one step earlier, at the rule in `docs/specs/pachinko.allium`.

## Related pages

- [Develop locally](../how-to/develop-locally.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Work with the specifications](../how-to/work-with-the-specs.md)
