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
syncs all four lockfiles and installs
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
`crates/pachinko/src/lib.rs`. `run` reads the command line: `serve [flags]` starts the
HTTP service, and no argument calls the private `version`, which writes the line
`pachinko <version>` to whatever writer it is given; the version is
`libpachinko::version()`, the library's manifest version. The binary's `main`, in
`crates/pachinko/src/main.rs`, calls `run` on standard output and `exit_code` on its
result, and nothing else. `run` carries an example, and `just test-doc` runs every such
example as a test. Your exercise changes the line `version` writes.

## 3. Read what decides the behaviour

Open `docs/specs/pachinko.allium`. It is the root module: it declares the queue's
entities, rules, surfaces and invariants, and every behaviour the library and the
service implement is stated there before any code implements it. The version line is
not the queue's behaviour, so no clause decides it. Three tests state it instead:

- `run_writes_the_name_and_the_library_version`, the unit test at the foot of
  `crates/pachinko/src/lib.rs`, calls `run` on a `Vec<u8>` and compares what it wrote.
- `the_binary_prints_its_line_and_exits_zero`, in `crates/pachinko/tests/binary.rs`,
  runs the built binary as a user would and compares its standard output, its standard
  error and its exit status.
- The example in `run`'s doc comment, which `just test-doc` runs, calls `run` with no
  argument and compares what it wrote.

Read the three expected lines now. Each builds the line from `libpachinko::version()`,
`format!("pachinko {}\n", libpachinko::version())`, so the tests hold whatever version
the manifest states and the exercise works the same on any `main`.

## 4. Change something

In `crates/pachinko/src/lib.rs`, change the format string in `version` so the line
reads `pachinko version <version>`:

```diff
 fn version(out: &mut impl Write) -> Result<(), ServeError> {
-    writeln!(out, "pachinko {}", libpachinko::version())
+    writeln!(out, "pachinko version {}", libpachinko::version())
         .map_err(ServeError::io("writing the version"))
 }
```

Leave the tests alone and run the suite. It should fail in three places, because three
tests state the old line.

```console
just test
```

Each failure is an `assert_eq!` on bytes, so the assertion prints both sides as lists of
numbers: `left` is what was written, `right` is what the test expected. The numbers are
the bytes' values; 32 is a space and 10 the newline. When this page was written, at
version 0.1.0, the unit test reported:

```text
thread 'tests::run_writes_the_name_and_the_library_version' (455386462) panicked at crates/pachinko/src/lib.rs:107:9:
assertion `left == right` failed
  left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
 right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
```

`118, 101, 114, 115, 105, 111, 110, 32` after the first space is `version` and its
space. The binary test fails with the same two lists at
`crates/pachinko/tests/binary.rs`.

You will likely see only the unit test's failure. `.config/nextest.toml` sets
`fail-fast = true` for the default profile, so nextest stops starting tests after the
first failure, and `just test` stops before the doctests. To see every nextest failure
at once, run nextest without fail-fast; `pixi run` puts the pinned nextest on `PATH` as
the `Justfile` does:

```console
pixi run --frozen cargo nextest run --workspace --all-features --locked --no-fail-fast
```

Its summary names both failures:

```text
     Summary [   0.632s] 236 tests run: 234 passed, 2 failed, 0 skipped
        FAIL [   0.006s] (183/236) pachinko tests::run_writes_the_name_and_the_library_version
        FAIL [   0.302s] (227/236) pachinko::binary the_binary_prints_its_line_and_exits_zero
error: test run failed
```

The third failure is the doctest, which nextest does not run. `just test-doc` runs it
and reports the same two lists under `crates/pachinko/src/lib.rs - run (line 47)`.
`pixi run --frozen cargo nextest run --help` lists the flag. Or fix the failure you see,
run `just test` again, and the next appears.

A change that no test notices is either already covered or untested. Here the failures
are the point: they show the line is stated in three places, and where.

## 5. Change the tests to match

Update the expected line in all three tests, inserting `version` and a space after
`pachinko`. In `crates/pachinko/src/lib.rs`, the unit test:

```diff
-            format!("pachinko {}\n", libpachinko::version()).as_bytes()
+            format!("pachinko version {}\n", libpachinko::version()).as_bytes()
```

and the example in `run`'s doc comment:

```diff
-/// assert_eq!(out, format!("pachinko {}\n", libpachinko::version()).as_bytes());
+/// assert_eq!(out, format!("pachinko version {}\n", libpachinko::version()).as_bytes());
```

In `crates/pachinko/tests/binary.rs`:

```diff
-        format!("pachinko {}\n", libpachinko::version()).as_bytes()
+        format!("pachinko version {}\n", libpachinko::version()).as_bytes()
```

Run `just test` again. Every nextest test passes, then the doctests. In a real change,
the prose in `run`'s and `version`'s doc comments, which names the line, and the crate
docs would change in the same commit too: the
behaviour, its tests and its documentation land together. For a behaviour the
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

An item, the three tests that state its output, and the gate. That is the whole loop;
every change after this one is the same shape, and a change to the queue's behaviour
starts by changing a rule in the specification module.

## Related pages

- [Develop locally](../how-to/develop-locally.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Work with the specifications](../how-to/work-with-the-specs.md)
