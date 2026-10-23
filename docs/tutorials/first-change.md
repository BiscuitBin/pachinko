---
title: "Make your first change"
kind: "tutorial"
audience: [contributor, agent]
canonical_for: [first_change_tutorial]
requires: []
---

# Make your first change

From a fresh clone to a green gate, through one small change you then put back. The
first run downloads the pixi environment, a Rust toolchain, cargo-hack and rustqual, the
Allium checker and the hook environments, so it needs the network, and it needs rustup,
pixi and `just` installed first; [Develop locally](../how-to/develop-locally.md)
describes all three.
The change is small on purpose; what matters is that it passes through every layer the
repository has today: an item, the tests that state what it does, and the gate.

Start from a clean worktree: `git status --short` prints nothing. The last step puts the
crates back with `git checkout`, which would also throw away any edit of yours under
`crates/`.

## 1. Get the workspace running

```console
just initialize
just check
```

`just initialize` installs the pixi environment from `pixi.lock`, installs the toolchain
`rust-toolchain.toml` pins, refuses a cargo that is not rustup's, installs cargo-hack and
rustqual into `.tools/bin` and then the Allium checker, syncs both lockfiles and installs
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
`crates/pachinko/src/lib.rs`. `run` writes the service's one line, `pachinko <version>`,
to whatever writer it is given; the version is `libpachinko::version()`, the library's
manifest version. The binary's `main`, in `crates/pachinko/src/main.rs`, calls `run` on
standard output and nothing else. The library's `version` carries an example, and
`just test-doc` runs every such example as a test. Your exercise changes the
line `run` writes.

## 3. Read what decides the behaviour

Open `docs/specs/pachinko.allium`. It is the root module, and today it declares no
entity, rule, surface or invariant: the domain is not defined yet, and the first change
that names a behaviour states it there before any code implements it. So no clause
decides the line `run` writes. Two tests state it instead, byte for byte:

- `run_writes_the_name_and_the_library_version`, the unit test at the foot of
  `crates/pachinko/src/lib.rs`, calls `run` on a `Vec<u8>` and compares what it wrote.
- `the_binary_prints_its_line_and_exits_zero`, in `crates/pachinko/tests/binary.rs`,
  runs the built binary as a user would and compares its standard output, its standard
  error and its exit status.

Read both expected byte strings now. When this page was written they were
`b"pachinko 0.1.0\n"`; on a later `main` the version may differ, and the exercise works
the same with whatever version they hold.

## 4. Change something

In `crates/pachinko/src/lib.rs`, change the format string in `run` so the line reads
`pachinko version <version>`:

```diff
 pub fn run(out: &mut impl Write) -> io::Result<()> {
-    writeln!(out, "pachinko {}", libpachinko::version())
+    writeln!(out, "pachinko version {}", libpachinko::version())
 }
```

Leave the tests alone and run the suite. It should fail in two places, because two tests
state the old line.

```console
just test
```

Each failure is an `assert_eq!` on bytes, so the assertion prints both sides as lists of
numbers: `left` is what was written, `right` is what the test expected. The numbers are
the bytes' values; 32 is a space and 10 the newline. When this page was written, the
unit test reported:

```text
thread 'tests::run_writes_the_name_and_the_library_version' (316050970) panicked at crates/pachinko/src/lib.rs:26:9:
assertion `left == right` failed
  left: [112, 97, 99, 104, 105, 110, 107, 111, 32, 118, 101, 114, 115, 105, 111, 110, 32, 48, 46, 49, 46, 48, 10]
 right: [112, 97, 99, 104, 105, 110, 107, 111, 32, 48, 46, 49, 46, 48, 10]
```

`118, 101, 114, 115, 105, 111, 110, 32` after the first space is `version` and its
space. The binary test fails with the same two lists at
`crates/pachinko/tests/binary.rs`, and nextest's summary names both:

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

Update the expected bytes in both tests, inserting `version` and a space after
`pachinko` and keeping whatever version follows. In `crates/pachinko/src/lib.rs`:

```diff
-        assert_eq!(out, b"pachinko 0.1.0\n");
+        assert_eq!(out, b"pachinko version 0.1.0\n");
```

In `crates/pachinko/tests/binary.rs`:

```diff
-    assert_eq!(output.stdout, b"pachinko 0.1.0\n");
+    assert_eq!(output.stdout, b"pachinko version 0.1.0\n");
```

Run `just test` again. Every nextest test passes, then the doctests. In a real change,
`run`'s doc comment, which names the line, would change in the same commit too: the
behaviour, its tests and its documentation land together. Once the specifications define
a behaviour, the clause moves first and the test is named for the clause it proves.

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

This was an exercise, so do not commit it. Restore the crates and confirm the worktree is
clean again:

```console
git checkout -- crates/
git status --short
```

For a real change you would commit instead. The pre-commit hook runs the fast,
read-only part of the gate again. It does not run clippy, the tests or coverage, which is
why `just check` comes first. Nothing is pushed until you ask for it.

## What you just touched

An item, the two tests that state its output, and the gate. That is the whole loop;
every change after this one is the same shape, and the first real one starts by adding a
rule to a specification module rather than finding none.

## Related pages

- [Develop locally](../how-to/develop-locally.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Work with the specifications](../how-to/work-with-the-specs.md)
