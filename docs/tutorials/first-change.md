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
`crates/pachinko/src/lib.rs`. With no arguments, `run` writes the service's version
line, `pachinko <version>`, to whatever writer it is given, through the private function
`write_version` further down the file; the version is `libpachinko::version()`, the
library's manifest version. With `serve` it runs the queue instead, which this exercise
leaves alone. The binary's `main`, in `crates/pachinko/src/main.rs`, calls `run` with
the arguments and standard output, and turns an error into one line on standard error
and exit code 1. `run`'s doc comment carries an example, as the library's `version`
does, and `just test-doc` runs every such example as a test. Your exercise changes the
line `write_version` writes.

## 3. Read what decides the behaviour

Open `docs/specs/pachinko.allium`. It is the root module, and it defines the queue:
the pockets, records and leases libpachinko implements and the service serves. The
version line is not part of that domain, so no clause decides the line `write_version`
writes. Three tests state it instead:

- `run_writes_the_name_and_the_library_version`, the unit test at the foot of
  `crates/pachinko/src/lib.rs`, calls `run` with no arguments on a `Vec<u8>` and
  compares what it wrote.
- The example in `run`'s doc comment, in the same file, does the same as a doctest.
- `the_binary_prints_its_line_and_exits_zero`, in `crates/pachinko/tests/binary.rs`,
  runs the built binary as a user would and compares its standard output, its standard
  error and its exit status.

Read the three expected lines now. Each builds the line from `libpachinko::version()`,
`format!("pachinko {}\n", libpachinko::version())`, so the tests hold whatever version
the manifest states and the exercise works the same on any `main`.

## 4. Change something

In `crates/pachinko/src/lib.rs`, change the format string in `write_version` so the line
reads `pachinko version <version>`:

```diff
 fn write_version(out: &mut impl Write) -> Result<(), CliError> {
-    writeln!(out, "pachinko {}", libpachinko::version())
+    writeln!(out, "pachinko version {}", libpachinko::version())
         .map_err(|error| CliError::Output(error.to_string()))
 }
```

Leave the tests alone and run the suite. It should fail in two places, the two tests
nextest runs. The doctest waits for step 5: `just test` runs the doctests only once
nextest passes.

```console
just test
```

Each failure is an `assert_eq!` on bytes, so the assertion prints both sides as lists of
numbers: `left` is what was written, `right` is what the test expected. The numbers are
the bytes' values; 32 is a space and 10 the newline. When this page was written, at
version 0.1.0, the unit test reported:

```text
thread 'tests::run_writes_the_name_and_the_library_version' (354330136) panicked at crates/pachinko/src/lib.rs:235:9:
assertion `left == right` failed
  left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
 right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
```

`118, 101, 114, 115, 105, 111, 110, 32` after the first space is `version` and its
space. The binary test fails with the same two lists at
`crates/pachinko/tests/binary.rs`, and nextest's summary names both:

```text
     Summary [   0.694s] 175/188 tests run: 173 passed, 2 failed, 1 skipped
        FAIL [   0.010s] (158/188) pachinko tests::run_writes_the_name_and_the_library_version
        FAIL [   0.611s] (174/188) pachinko::binary the_binary_prints_its_line_and_exits_zero
warning: 13/188 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
error: test run failed
```

You may see only one failure. `.config/nextest.toml` sets `fail-fast = true` for the
default profile, so nextest stops starting tests after the first failure. Both ran here
because the binary test had already started when the unit test failed, and the output
between them says so: `Cancelling due to test failure: 17 tests still running`. The
summary's warning counts the tests that never started. On a machine with fewer cores the
second failure may never start. To see every failure at once, run nextest without fail-fast;
`pixi run` puts the pinned nextest on `PATH` as the `Justfile` does:

```console
pixi run --frozen cargo nextest run --workspace --all-features --locked --no-fail-fast
```

`pixi run --frozen cargo nextest run --help` lists the flag. Or fix the failure you see,
run `just test` again, and the second appears.

A change that no test notices is either already covered or untested. Here the failures
are the point: they show where the line is stated, and step 5 finds the third place.

## 5. Change the tests to match

Update the expected line in both tests, inserting `version` and a space after
`pachinko`. In `crates/pachinko/src/lib.rs`:

```diff
-            format!("pachinko {}\n", libpachinko::version()).as_bytes()
+            format!("pachinko version {}\n", libpachinko::version()).as_bytes()
```

In `crates/pachinko/tests/binary.rs`:

```diff
-        format!("pachinko {}\n", libpachinko::version()).as_bytes()
+        format!("pachinko version {}\n", libpachinko::version()).as_bytes()
```

Run `just test` again. Every nextest test passes, then the doctests run, and the example
in `run`'s doc comment fails with the same two lists. Its panic names a file rustdoc
generated, not `lib.rs`; the failure list names the example by its file and line:

```text
failures:
    crates/pachinko/src/lib.rs - run (line 47)

test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Update the example's expected line in `crates/pachinko/src/lib.rs` too:

```diff
-/// assert_eq!(out, format!("pachinko {}\n", libpachinko::version()).as_bytes());
+/// assert_eq!(out, format!("pachinko version {}\n", libpachinko::version()).as_bytes());
```

Run `just test` a third time. Every nextest test passes, then every doctest. In a real
change, the prose of `run`'s doc comment, which names the line too, would change in the
same commit: the behaviour, its tests and its documentation land together. For a
behaviour the specification decides, the clause moves first and the test is named for
the clause it proves.

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
every change after this one is the same shape, and one that changes the queue starts with
the clause in `docs/specs/pachinko.allium` that decides it.

## Related pages

- [Develop locally](../how-to/develop-locally.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Work with the specifications](../how-to/work-with-the-specs.md)
