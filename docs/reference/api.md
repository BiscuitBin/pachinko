---
title: "API reference"
kind: "reference"
audience: [user, contributor, maintainer, agent]
canonical_for: [api_reference]
requires: []
---

# API reference

The API reference is the one rustdoc generates from the doc comments of both crates,
`libpachinko` and `pachinko`. This page does not copy it: a list of items written out
here would be wrong after the first change. It says how to build the reference, where it
lands, where it is hosted, what the public API promises, and what a doc example is for.

## Building it

`just doc` runs `cargo doc` over the workspace with every feature, no dependencies and
`--locked`, with `RUSTDOCFLAGS="-D warnings --cfg docsrs"`. The result opens at
`target/doc/libpachinko/index.html` for the library and `target/doc/pachinko/index.html`
for the service. It is gate 12 of `just check`, so a broken intra-doc link, a link to a
private item or an undocumented public item fails the gate rather than producing a page
with a hole in it.

## What is documented

`missing_docs` is set to warn in the `[workspace.lints.rust]` table, and both `just doc`
and `just clippy` turn warnings into errors. Removing the doc comment from one public
item fails both recipes with a `missing documentation for` error naming its kind. So
every public item carries documentation, and each crate root carries the overview. The
rustdoc lints in `[workspace.lints.rustdoc]` add broken and private intra-doc links,
which fail outright, and a missing crate-level doc and unescaped backticks, which warn.

## Doc examples are tests

[The agent contract](agent-contract.md) asks for an example on every public item; today
`libpachinko::version` carries one. `just test-doc` (gate 10) compiles and runs each
example, so an example that stops being true stops the gate. Examples are not counted
toward the coverage floor, because the coverage tooling cannot measure doctests on a
stable toolchain. The division of labour follows: an example shows the contract to a
reader and proves it, and a unit test earns the coverage. See [Testing](testing.md).

## Hosted

The reference for `main` is hosted at <https://biscuitbin.github.io/pachinko/>, and a
push to `main` republishes it. It documents `main`, not a release, so it may describe an
item no release has. `just site` builds what is hosted: an empty `target/doc`, then
`just doc` with the flags above, then a root `index.html` that links to both crates'
pages, because stable rustdoc writes none. `just site` is not a gate; `just doc` is.
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md) has the workflow and the
rollback, and
[decision 0013](../decisions/0013-api-reference-on-github-pages.md) records why the
reference is hosted there.

## docs.rs

`crates/libpachinko/Cargo.toml` and `crates/pachinko/Cargo.toml` each carry a
`[package.metadata.docs.rs]` table: every feature, `--cfg docsrs`, and one target,
`x86_64-unknown-linux-gnu`. This is prospective. Both crates are `publish = false`
([decision 0006](../decisions/0006-apache-2-0.md)), so nothing is on docs.rs, and the
release process ticket T14 adds keeps it so: a tag cuts a GitHub Release, not a registry
publish. Only a later, separately authorised decision to publish would put a crate on
docs.rs; until then the hosted site is the only API reference.

## What the API promises

Every public type is `Send + Sync + 'static`, `Clone` and `Debug`, so a consumer may move
a value between threads, copy it or print it. Public structs and enums are
`#[non_exhaustive]`, so a field or variant added later breaks no consumer. These are
invariants of [the agent contract](agent-contract.md); the domain types that will carry
them arrive with the specifications, and today the public API is two functions,
`libpachinko::version` and `pachinko::run`. Neither crate has features or dependencies
beyond `pachinko`'s on `libpachinko`. The library performs no effect itself; its effect
traits will appear here beside their fakes, and
[Architecture](../explanation/architecture.md) explains the boundary.

## Related pages

- [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md)
- [Architecture](../explanation/architecture.md)
- [Testing](testing.md)
- [Purpose and scope](../project/purpose-and-scope.md)
- [Decision 0013: The API reference on GitHub Pages](../decisions/0013-api-reference-on-github-pages.md)
