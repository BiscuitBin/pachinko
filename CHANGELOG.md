# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each crate has its own unreleased section and versions. Release headings name the crate
and version, dated on the UTC day its tag is pushed. The release process is described in
[Maintenance](docs/operations/maintenance.md#cut-a-release).

## libpachinko [Unreleased][libpachinko-unreleased]

### Changed

- The crate root is the caller's interface, and one `use libpachinko::{…}` names it:
  `Queue`, `Config`, the names and values, `Timestamp`, `LeaseId`, `ParseLeaseIdError`,
  `HeldRecord`, every outcome and every error enum moved there, and `ids`, `config`,
  `error` and `queue` are private. `clock`, `lease_ids` and `records` stay public as the
  seam modules, each holding its trait and its fake or implementation: `Timestamp` left
  `clock`, `LeaseId` and `ParseLeaseIdError` left `lease_ids`, `HeldRecord` left
  `records`, `Position` moved from `ids` to `records`, and `records::memory` is private,
  so `records::MemoryRecords` is that store's one path. No old path is kept as an alias.
- The `testing` feature is one additive module, `testing`, with two `Queue` methods,
  and nothing it adds changes what the root or a seam module holds. `Snapshot`,
  `PocketView`, `PocketStatus`, `ShedReason`, `SupersededRecord` and `DeadLetter` moved
  there from `queue`, and `check_contract` from `records::contract`.
  `PocketRecords::snapshot` became `testing::InspectRecords::snapshot`, which
  `check_contract` and `Queue::snapshot` now require of a store. `TickReport` lost
  `leases_examined` and `shed_examined` to `testing::TickCost`, which
  `Queue::tick_with_cost` returns beside the report. A test run without the feature
  fails on `tests/testing_feature.rs`, where it used to pass with the integration suites
  compiled to nothing.
- The specification closes open question 8 by decision 0019: a pull, ack, nack, renew or
  release processed after its lease's `expires_at` but before the lapse succeeds, and
  decision 0019 confirms its five configuration defaults for the MVP.
- `Queue::ack` and `Queue::nack` reset their pocket's consecutive-expiry count, so only
  `max_consecutive_expiries` lapses with no intervening ack, nack, release or revive park
  a pocket; lapses before an ack or a nack no longer count toward it. A renew or pull
  leaves the count, and a repeated ack or nack finds it at zero already. The
  specification says that a pull within a lease extends it on an empty batch.
- An ack or a nack marks its held record and removes nothing. `SettleOutcome` is `Acked`
  or `Nacked`, replacing `Settled` and `Deferred`; a repeat of the first mark answers as
  the first did and extends the lease, and the other mark is refused with the new
  `LeaseActionError::MarkConflict`, checked after `RecordNotHeld`. The record stays in
  the lease's held set, at the version handed out, until the lease ends, and
  `LeaseView::records` lists each held record as a new `MarkedRecord` with its new
  `Mark`. A release or a lapse reconciles every held record by its mark: an unsettled
  one is queued again at its position, an acked version is consumed, and a nacked one
  becomes a `DeadLetter` only then, decaying `dead_letter_ttl` after the lease's end.
  Every entry one lease end creates is stamped from that end and comes out smallest held
  position first. Decision 0020 records why.
- A supersede never sheds a held version. A publish onto a held key answers
  `Superseded` and waits behind the held version at a fresh arrival position, a still
  higher publish replaces and sheds only that waiting version, and the lease's end
  queues it: at its arrival position behind an acked or nacked version, or at the held
  version's position behind an unsettled one, which is then a superseded record. A key
  acked or nacked under a lease is not pullable again under it.
- A stale publish no longer touches its pocket, so a parked pocket stays parked with its
  count. `PublishError::EqualRank` is a decided refusal: its message says a publish at
  the stored rank is rejected and changes nothing, and no longer calls it a placeholder.
  The specification closes open questions 1 and 9, answers the two the wave's code
  review raised, a supersede shedding a held version and a stale publish reviving a
  parked pocket, and keeps six open questions.
- The store seam: `PocketRecords` loses `settle`, `defer` and `free_held` and gains
  `defer_behind_held`, `mark`, `held`, which lists the `MarkedRecord`s, and `end_lease`,
  which applies the six lease-end rows and returns the new `records::EndedVersion`s.
  `RecordStatus::Held` carries the mark, the nack's time and the deferred version's
  arrival position, and `RecordStatus::Deferred` is gone. `check_contract` pins the new
  steps.

### Added

- The queue's workloads as divan benches, W1 publish, W2 supersede, W3 pull 16, ack and
  release, W4 publish with every fifth op a consume step, and W5 pull then lapse on
  tick, each at 10, 1,000 and 100,000 pockets in ops per second, one op a library call;
  `just bench` runs them by hand, never in CI or the gate.
- The invariant walk: a seeded proptest walk over every queue trigger that predicts each
  step's outcome and post-state from the rules, checks all 35 invariants and every
  declared transition after every step, and proves its generator reaches every state
  and outcome; and the scenario catalog in the testing reference, mapping each of the
  93 scenarios to the test that pins it.
- Lease expiry and decay: `Queue::tick` lapses every active lease whose `expires_at`
  has passed, in the time of the leases due rather than active, and reports them in
  `TickReport` with the superseded records and dead letters it decayed. A lapse keeps a
  pocket with records at the front, idles an empty one, and parks one after
  `max_consecutive_expiries` lapses with no ack, nack, release or revive between them,
  until a publish that adds or raises a record revives it.
- The lease actions: `Queue::ack`, `Queue::nack`, `Queue::renew` and `Queue::release`,
  each fenced to the lease it names and refusing, in rule order, with
  `LeaseActionError` (`UnknownConsumer`, `LeaseNotActive`, `LeaseNotYours`,
  `RecordNotHeld`, `MarkConflict`); `SettleOutcome` for an acked or nacked record,
  `PocketFate` for a requeued or idled pocket, and `DeadLetter` for a nacked version
  kept beside the superseded records once its lease ends. An ended lease leaves the
  registry, and a publish revives an idle pocket at the back.
- The library tracer: `Queue` over a record store, a clock and a lease-id source, with
  publisher and consumer joins, publish (new pocket, new key, supersede, stale shed),
  pull (claim the front pocket, pull within a lease, no pocket available) and the lease
  view; the `Clock` and `LeaseIds` effect traits with `FakeClock` and
  `CountingLeaseIds`; the `PocketRecords` store seam with `MemoryRecords`; `Config` with
  the specification's defaults and load checks; one `thiserror` error enum per
  operation; and a `testing` feature with the records contract and the queue snapshot.
- The first behavioural specification, `docs/specs/pachinko.allium`: pockets holding one
  record per key, higher ranks superseding in place, exclusive renewable leases, parking
  after consecutive lease expiries, and decaying superseded records and dead letters.
- Manual divan benchmarks and stable byte fuzzing with a weekly crash-reporting job.
- Workspace mutation testing through `just mutants` and a weekly report workflow.
- An Apache-2.0 library, `libpachinko`, exposing its version.
- A pinned toolchain and tools, the `just initialize` setup, and the read-only
  `just check` gate: linting, tests, coverage, API documentation, dependency checks,
  metrics and a probed effects boundary for the library.
- The handbook, architecture decisions, the pinned Allium checker, and an agent contract with skills and provider bridges.
- GitHub Actions for continuous integration and dependency advisory audits, a shared
  pre-commit hook, and both crates' API reference hosted on GitHub Pages.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow,
  with a semver report for the library. Neither crate is published to crates.io.
- Renovate configuration over the repository's pins, each update held for approval on its dashboard
  except vulnerability fixes, with the pins that stay manual named in the handbook.

## pachinko [Unreleased][pachinko-unreleased]

### Added

- An Apache-2.0 service, `pachinko`, that writes its name and the library version.
- `pachinko serve`: an HTTP/1.1 JSON API over one owner task holding the queue, with
  the publisher and consumer joins, single-record publish, pull and the lease read, and
  `GET /healthz`; six flags for the listen address and the specification's five config
  values; the real `SystemClock` and `EpochSerialLeaseIds`; and a clean shutdown on
  SIGTERM or SIGINT (decision 0019).
- The tick lane: a ticker sweeps every `--sweep-ms` (default 1000), on a lane the
  owner polls before its commands, lapsing leases and decaying superseded records and
  dead letters; each sweep that did anything logs one `info` line with its counts.
- The four lease routes, `POST /leases/{lease_id}/ack`, `/nack`, `/renew` and
  `/release`, with the error codes `lease_not_active` (409), `lease_not_yours` (403)
  and `record_not_held` (404), and a 400 `bad_request` for a malformed lease id.
- Batch publish, `POST /publishers/{publisher_id}/publish`: one owner command per
  request answering one outcome per row in row order, an equal rank answered for its row
  alone; and the request-shape limits `--max-batch-rows` (1000), `--max-payload-bytes`
  (1 MiB) and `--max-body-bytes` (16 MiB, replacing axum's 2 MiB default on every route),
  refused as 413 `too_many_rows`, `payload_too_large` and `body_too_large` before any
  row applies, never for want of room.
- Operability for `pachinko serve`: `GET /readyz`, 204 while serving and 503
  `not_ready` while starting or stopping; `--log-level` (default `info`, `RUST_LOG`
  ignored), with the per-request line at `debug`; `--request-timeout-ms` (default
  30000), answering 408 `request_timeout` and closing idle connections; and
  `--shutdown-grace-ms` (default 5000), after which open connections are cut and,
  one more grace period later, anything left is aborted.
- A closed-loop load generator and lapse probe, `just loadgen`, in the excluded
  `loadgen/` crate with its own lockfile: publish, batch, consume and mix workloads and
  the lapse lag against a running `pachinko serve`, failing closed on any error. It
  runs by hand, never in CI or the gate, and decision 0019 records its figures.
- A digest-pinned container image, `just image`, and a required CI build and run without publication.
- Independent crate releases through cargo-release and a guarded GitHub Release workflow.
  The crate is not published to crates.io.

### Changed

- `just check` runs a twentieth gate, `just test-doc-default`, every doctest with
  default features, and CI's `rust` job runs it too.
- `just lint` reads untracked files Git does not ignore as well as tracked ones, and
  `just fix` repairs them; the trailing-whitespace fixer leaves the Markdown under
  `tickets/` alone.
- CI runs the load generator's unit tests in a `loadgen` job, one of `check`'s `needs`.
- An ack or a nack over HTTP resets the pocket's count of consecutive lapses that
  `--max-consecutive-expiries` measures.
- An ack answers 200 `{"outcome": "acked"}` and a nack `{"outcome": "nacked"}`, replacing
  `settled` and `deferred`; a repeat of the first mark answers the same, and the other
  mark is refused with 409 `mark_conflict`. Each record on the lease read,
  `GET /consumers/{consumer_id}/lease`, carries its `mark`, `unsettled`, `acked` or
  `nacked`, and an acked or nacked record stays there until the lease ends. A nacked
  record becomes a dead letter only when its lease is released or lapses. A publish onto
  a held key answers `superseded` while the version the consumer holds stays held.
- A stale publish leaves a parked pocket parked with its count, and the `equal_rank`
  message no longer calls the refusal a placeholder.

[libpachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
[pachinko-unreleased]: https://github.com/BiscuitBin/pachinko/commits/main/
