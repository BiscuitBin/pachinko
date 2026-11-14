---
title: "Testing"
kind: "reference"
audience: [contributor, maintainer, agent]
canonical_for: [testing_reference]
requires: []
---

# Testing

## Framework

cargo-nextest runs the unit and integration tests. It cannot run doctests, so `just test`
runs nextest and then `cargo test --doc`, and `just test-doc` runs the doctests alone.
Both turn every feature on; `just test-doc-default` runs the doctests again with default
features, the build a user of the crate gets.
`.config/nextest.toml` names the minimum nextest version it needs (the pin itself is in
`pyproject.toml` and `pixi.lock`) and holds the profiles: the default one
stops at the first failure, and a `ci` profile, which runs everything and writes a JUnit
report, is there for a job that wants one; no recipe selects it today. Divan is the
library’s development dependency for benchmarks, and proptest drives the queue's
invariant walk, as a development dependency only. Insta, the snapshot crate, arrives
with the first test that needs it, as a development dependency only, with the note that
[decision 0007](../decisions/0007-dependency-policy.md) asks for beside its range.

## Layout

Tests live in three places, and each is deliberate.

| Where | What |
| --- | --- |
| `#[cfg(test)] mod tests` at the foot of a module | Unit tests of that module, with access to its private items. |
| `crates/libpachinko/tests/*.rs` and `crates/pachinko/tests/*.rs` | Integration tests: the crate as a caller sees it, through its public API only. |
| `///` examples on public items | Doctests: each example compiles and runs as a test. |

Unit tests live in the module they test. This departs from the games' rule that tests are
never colocated with the code: in Rust a unit test can reach a private item only from
inside its module, and [Decision 0009](../decisions/0009-rust-quality-gate.md) records
the deviation.

libpachinko's integration tests are the queue suite under `tests/queue/`,
`tests/records_contract.rs` and the guard `tests/testing_feature.rs`, which "Tests from
the specifications" describes.
`crates/pachinko/tests/binary.rs` is the binary test: it runs the built service as a user
would, through the path Cargo hands it as `CARGO_BIN_EXE_pachinko`, and asserts that the
process exits zero, prints `pachinko`, a space, `libpachinko::version()` and a newline,
and writes nothing to standard error. The file opens with an `#![expect]` of `clippy::tests_outside_test_module` and its
reason, because an integration test file is its own test module.
[Run the binary test](../how-to/test-and-debug.md#run-the-binary-test) says how to run it
and how it divides from the unit test beside `run`.

A helper shared between two files under `tests/` would be compiled into each, and each
would report the functions it does not call as dead code. So each file carries the few
helpers and fixtures it uses, and the snapshot tests, once there are any, share one file.

The property test persists no regression file. Its runner is seeded, with
`failure_persistence: None`, so a red run never writes a `proptest-regressions/` file
into a worktree that a read-only recipe is checking for changes, and every run draws the
same cases. A failure is reproduced from the shrunk case the output prints: the list of
steps, which a scratch test can replay through the same walk.

## Conventions

**Derive a test from the clause it proves, and name it after the clause.** A test named
`the_binary_prints_its_line_and_exits_zero` says which guarantee broke when it fails; a
test named after the function it calls does not.

**A doc example is a test.** It runs under `just test-doc`, so an example that stops
being true stops the gate. Every public item is to carry one.

**Reach an effect through the fake beside its trait; never through the real thing.** The
effect traits arrive with the specifications that need them, each with a fake beside it,
and a test drives the library through that fake and gets exactly the behaviour it chose.
[Architecture](../explanation/architecture.md) explains the boundary, and decisions
[0003](../decisions/0003-effects-behind-traits.md) and
[0008](../decisions/0008-std-library.md) record it.

**Restriction lints relax inside tests only.** `clippy.toml` allows `unwrap`, `expect`,
`panic`, indexing, `dbg!` and printing inside `#[cfg(test)]` code, and nowhere else, so a
test can fail loudly while the library cannot.

## Tests from the specifications

A specification's clauses are what the tests prove.
`just plan-spec <module>` prints, as JSON, the test obligations the Allium checker
derives from one module under `docs/specs/`: what each rule, invariant, transition and
surface clause would have a test for. It plans that one file and reads no import. The
list is a suggestion for a ticket's test list, not a gate: nothing asserts on it and it
sits outside `just check`. [Work with the specifications](../how-to/work-with-the-specs.md)
covers the workflow.

The tests of `docs/specs/pachinko.allium` follow one layout.

- **The `testing` feature.** `libpachinko`'s `testing` feature adds one module,
  `testing`, and two `Queue` methods, and nothing it adds changes what the crate root or
  a seam module holds. Together they are the inspection seam every invariant is checked
  through, `Queue::snapshot` with the `Snapshot` and `PocketView` it returns, over a
  store that implements `InspectRecords`; the tick's cost, `Queue::tick_with_cost` with
  its `TickCost`, the entries a tick read to find what was due, which `TickReport` does
  not carry; the records contract, `check_contract`; and `PocketStatus`, `ShedReason`,
  `SupersededRecord` and `DeadLetter`, values a snapshot shows and no other surface
  exports. The gate's `just test`, `just coverage` and `just clippy` turn the feature on
  with `--all-features`. The queue suite and `tests/records_contract.rs` open with
  `#![cfg(feature = "testing")]`, because one of the three clippy passes of
  `scripts/check-effects.sh` lints libpachinko with `--no-default-features` over all
  targets, the integration tests among them. The suites name the `testing` module, so
  with the feature off they must compile to nothing or that pass could not build them.
  [Quality gates](quality-gates.md#gate-8-the-effects-bans) lists the passes. A doc
  example that uses the feature guards those lines with
  `# #[cfg(feature = "testing")] {` and `# }`, and gate 20, `just test-doc-default`,
  enforces the guard by running every doctest without the feature.
- **The guard.** Compiled to nothing, the suites would let a test run without the
  feature pass on the unit tests and the doctests, with neither suite run.
  `tests/testing_feature.rs` is compiled only without the feature, and its one test,
  `the_testing_feature_is_on`, panics with
  ``libpachinko's suites need the `testing` feature: run `just test` ``. So a plain
  `cargo test -p libpachinko` fails on it, and every recipe that runs libpachinko's
  tests passes `--all-features`, so none runs it.
- **The queue suite.** `crates/libpachinko/tests/queue/` is one test target: `main.rs`
  declares the modules, each `#[cfg(test)]`, and states the tag conventions;
  `fixture.rs` builds a queue over `MemoryRecords`, a `FakeClock` at a fixed instant and
  `CountingLeaseIds`, with helpers by literal keys and each invariant's check by its
  specification name; one file per rule family (`identity.rs`, `publish.rs`, `pull.rs`)
  holds one test per scenario and the invariant tests.
- **The invariant walk.** `invariants.rs` and `walk.rs` in the queue suite run
  `every_invariant_holds_after_every_step_of_any_walk`: 512 seeded proptest walks of 1
  to 80 steps over every trigger, refused ones included, on small domains (two
  publishers, three consumers, three pockets, three keys, ranks 0 to 3, limits -1 to 3,
  the clock moving forwards past `lease_ttl` and backwards). Before each step the walk
  projects the queue's `Snapshot` into a model and applies the step's rules to it, so it
  predicts both the outcome and the whole post-state, branch A of decision 0019 and the
  `EqualRank` placeholder included; a step that should change nothing must leave the
  snapshot equal. After each step it checks all 25 invariants by name, every declared
  transition, `OldestFirst`, `ExclusivePocket` and `OneVisitPerKeyPerLease`, reading
  every limit and TTL from the queue's own `config()`. Its functions are generic over the
  store, the clock and the lease-id source. `the_walk_reaches_every_state_and_every_outcome`
  draws the same walks and fails, naming the labels, when the generator never reaches a
  state or an outcome the module names. A failing walk prints the assertion that fired,
  named after the invariant, the rule's prediction or the transition, and proptest's
  shrunk list of steps; that list replays the failure.
- **The records contract.** `tests/records_contract.rs` runs `testing::check_contract`
  over `MemoryRecords`; a second store would implement `testing::InspectRecords` beside
  `PocketRecords` and get a line of its own.
- **Tags.** A test names what it pins in comments above it: `// scenario:` and the id,
  once per scenario across the crate; `// obligation:` and an id `just plan-spec`
  prints; and `// obligation: guarantee.<Name>` for a surface guarantee, which has no
  checker id. A `compile_fail` doctest may carry the tags when the type system is the
  test, as the negative-TTL scenario does in `config.rs`.

These rules hold for every module the specifications add.

- **The oracle comes from the specification.** Where a test needs to know whether an
  operation should be accepted, the answer is a model written in test code from the
  rule's `requires` clauses, never the implementation.
- **Invariants and surfaces are held by property.** A script of arbitrary operations,
  refused ones included, runs against the module, and after every step the property
  holds each invariant under its own name, as far as the surface shows it, and that an
  operation is accepted exactly when its rule allows it.
- **What the specification leaves open is pinned as the implementation's choice.** Where
  a specification leaves an order or a tie-break to the implementation, the module
  documents its rule, and a test that pins what that rule gives is a test of the rule. A
  change to the pinned value is a change to the rule and is read as one.
- **A test that passed when first run is shown to bite.** The code is broken on purpose
  and the named test is seen to fail; the ticket that built the module lists the breaks.
- **A path no input reaches is called directly.** A conversion that only a refusal
  further down could reach is tested by calling it with every variant.
- **What no test can see is said.** A call to a function, rather than through a
  boundary, leaves no trace a test can observe; a clause about such a call is read from
  the code, and the ticket says so.
- **A test file is split before the metrics gate's limit.** `rustqual.toml` holds test
  code to a thousand lines per file and a hundred per function; a module whose tests
  would pass that divides them between files by what each proves.

## Scenario catalog

The wave's 77 scenarios, each with the one library test whose `// scenario:` tag pins
it, found by `rg -n 'scenario: S0NN'`, and for the twelve `wire` scenarios the test that
also exercises it over HTTP. Tags: `core`, the scenario a reader would expect; `edge`, a
boundary the rules decide; `wire`, exercised over HTTP too; `oq:N`, a labelled
placeholder for open question N; `decision:0019`, a scenario decision 0019 settled. S066
and S067 each have two halves in two tests; the tag sits on the first and the row names
both.

| Id | Description | Tags | Owning test | Wire test |
| --- | --- | --- | --- | --- |
| S001 | Default `lease_ttl` and `max_pull_batch` apply | core | `crates/libpachinko/tests/queue/identity.rs`, `the_default_lease_ttl_and_pull_batch_apply` | |
| S002 | Default parking threshold and decay TTLs apply | core | `crates/libpachinko/tests/queue/identity.rs`, `the_default_parking_threshold_and_decay_ttls_apply` | |
| S003 | `max_consecutive_expiries = 0` is refused | edge | `crates/libpachinko/tests/queue/identity.rs`, `a_zero_expiry_limit_is_refused` | |
| S004 | `max_pull_batch = 0` is refused | edge | `crates/libpachinko/tests/queue/identity.rs`, `a_zero_pull_batch_is_refused` | |
| S005 | Negative TTLs are refused | edge | `crates/libpachinko/src/config.rs`, the `compile_fail` doctest on `Config` | |
| S006 | A zero `lease_ttl` lapses at the instant of grant | edge | `crates/libpachinko/tests/queue/expiry.rs`, `a_zero_lease_ttl_lapses_at_the_instant_of_grant` | |
| S007 | Zero superseded and dead-letter TTLs decay at once | edge | `crates/libpachinko/tests/queue/decay.rs`, `zero_superseded_and_dead_letter_ttls_decay_at_once` | |
| S008 | `max_pull_batch = 1` caps every pull at one record | edge | `crates/libpachinko/tests/queue/identity.rs`, `a_pull_batch_of_one_caps_every_pull_at_one_record` | |
| S009 | A publisher joins once | core | `crates/libpachinko/tests/queue/identity.rs`, `a_publisher_joins_once` | |
| S010 | A consumer joins once | core | `crates/libpachinko/tests/queue/identity.rs`, `a_consumer_joins_once` | |
| S011 | Publish into a new pocket creates a queued pocket and record | core | `crates/libpachinko/tests/queue/publish.rs`, `a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record` | |
| S012 | Publish of a new key appends to an existing pocket | core | `crates/libpachinko/tests/queue/publish.rs`, `a_publish_of_a_new_key_appends_to_an_existing_pocket` | |
| S013 | The same record key in two pockets is two records | edge | `crates/libpachinko/tests/queue/publish.rs`, `the_same_record_key_in_two_pockets_is_two_records` | |
| S014 | Supersede replaces rank and payload in place | core | `crates/libpachinko/tests/queue/publish.rs`, `a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser` | |
| S015 | Repeated supersede keeps the first slot and sheds every loser | edge | `crates/libpachinko/tests/queue/publish.rs`, `repeated_supersede_keeps_the_first_slot_and_sheds_every_loser` | |
| S016 | A stale publish is shed, not applied and not an error | core | `crates/libpachinko/tests/queue/publish.rs`, `a_stale_publish_is_shed_not_applied_and_not_an_error` | |
| S017 | An equal-rank publish is rejected (OQ1 placeholder, labelled) | oq:1 | `crates/libpachinko/tests/queue/publish.rs`, `an_equal_rank_publish_is_rejected_as_the_open_question_1_placeholder` | |
| S018 | Negative and zero ranks order like integers | edge | `crates/libpachinko/tests/queue/publish.rs`, `negative_and_zero_ranks_order_like_integers` | |
| S019 | Large ranks keep full precision across supersede and stale | wire | `crates/libpachinko/tests/queue/publish.rs`, `large_ranks_keep_full_precision_across_supersede_and_stale` | `crates/pachinko/tests/http.rs`, `large_ranks_keep_full_precision_across_supersede_and_stale` (driver `large_ranks`) |
| S020 | Payload bytes and keys round-trip unchanged | wire | `crates/libpachinko/tests/queue/publish.rs`, `payload_bytes_and_keys_round_trip_unchanged` | `crates/pachinko/tests/http.rs`, `payload_bytes_and_keys_round_trip_unchanged` (driver `round_trip`) |
| S021 | Publishing into a leased pocket neither requeues it nor escapes the lease | core | `crates/libpachinko/tests/queue/publish.rs`, `publishing_into_a_leased_pocket_neither_requeues_it_nor_escapes_the_lease` | |
| S022 | Publishing into a queued pocket does not move it in the pocket order | edge | `crates/libpachinko/tests/queue/publish.rs`, `publishing_into_a_queued_pocket_does_not_move_it_in_the_pocket_order` | |
| S023 | A publish revives an idle pocket at the back | core | `crates/libpachinko/tests/queue/lease.rs`, `a_publish_revives_an_idle_pocket_at_the_back` | |
| S024 | A supersede or a stale publish revives a parked pocket (OQ11 as the rules stand, the stale half) | edge, oq:11 | `crates/libpachinko/tests/queue/expiry.rs`, `a_supersede_or_a_stale_publish_revives_a_parked_pocket` | |
| S025 | An equal-rank publish does not revive a parked pocket | oq:1 | `crates/libpachinko/tests/queue/expiry.rs`, `an_equal_rank_publish_does_not_revive_a_parked_pocket` | |
| S026 | Only a publish revives a parked pocket (OQ7 as the rules stand) | oq:7 | `crates/libpachinko/tests/queue/expiry.rs`, `only_a_publish_revives_a_parked_pocket` | |
| S027 | A pull claims the front pocket, leases it and holds the batch | core | `crates/libpachinko/tests/queue/pull.rs`, `a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch` | |
| S028 | A positive limit returns at most that many, oldest first, and later pulls continue | core, wire | `crates/libpachinko/tests/queue/pull.rs`, `a_positive_limit_returns_at_most_that_many_oldest_first_and_later_pulls_continue` | `crates/pachinko/tests/http.rs`, `a_positive_limit_returns_at_most_that_many_oldest_first_and_later_pulls_continue` (driver `limit_pages`) |
| S029 | A limit above `max_pull_batch` is capped | edge | `crates/libpachinko/tests/queue/pull.rs`, `a_limit_above_max_pull_batch_is_capped` | |
| S030 | A limit above the queued count returns every queued record | edge | `crates/libpachinko/tests/queue/pull.rs`, `a_limit_above_the_queued_count_returns_every_queued_record` | |
| S031 | A zero-limit pull claims a pocket and returns an empty batch | edge, wire | `crates/libpachinko/tests/queue/pull.rs`, `a_zero_limit_pull_claims_a_pocket_and_returns_an_empty_batch` | `crates/pachinko/tests/http.rs`, `a_zero_limit_pull_claims_a_pocket_and_returns_an_empty_batch` (driver `zero_limit`) |
| S032 | A negative limit behaves like zero | edge | `crates/libpachinko/tests/queue/pull.rs`, `a_negative_limit_behaves_like_zero` | |
| S033 | A pull with no queued pocket returns `NoPocketAvailable`, not an error | core, wire | `crates/libpachinko/tests/queue/pull.rs`, `a_pull_with_no_queued_pocket_returns_no_pocket_available_not_an_error` | `crates/pachinko/tests/http.rs`, `a_pull_with_no_queued_pocket_returns_no_pocket_not_an_error` (driver `no_pocket`) |
| S034 | Idle, parked and leased pockets are never claimed | edge | `crates/libpachinko/tests/queue/expiry.rs`, `idle_parked_and_leased_pockets_are_never_claimed` | |
| S035 | A consumer with a lease never gets `NoPocketAvailable` or another pocket | edge | `crates/libpachinko/tests/queue/lease.rs`, `a_consumer_with_a_lease_never_gets_no_pocket_available_or_another_pocket` | |
| S036 | A pull under a lease extends the lease | core | `crates/libpachinko/tests/queue/pull.rs`, `a_pull_under_a_lease_extends_the_lease` | |
| S037 | A held record is not handed out again under the same lease | core | `crates/libpachinko/tests/queue/pull.rs`, `a_held_record_is_not_handed_out_again_under_the_same_lease` | |
| S038 | Pockets are claimed in the order they entered | core | `crates/libpachinko/tests/queue/pull.rs`, `pockets_are_claimed_in_the_order_they_entered` | |
| S039 | A second consumer never receives records from a leased pocket | core | `crates/libpachinko/tests/queue/pull.rs`, `a_second_consumer_never_receives_records_from_a_leased_pocket` | |
| S040 | An ack settles a held record | core, wire | `crates/libpachinko/tests/queue/settle.rs`, `an_ack_settles_a_held_record` | `crates/pachinko/tests/http.rs`, `an_ack_settles_a_held_record_over_http` (driver `ack_settles`) |
| S041 | An ack extends the lease | core | `crates/libpachinko/tests/queue/settle.rs`, `an_ack_extends_the_lease` | |
| S042 | Rejected acks change nothing, not even the lease's expiry | edge | `crates/libpachinko/tests/queue/settle.rs`, `rejected_acks_change_nothing_not_even_the_leases_expiry` | |
| S043 | A supersede while the key is held defers it on ack | edge | `crates/libpachinko/tests/queue/settle.rs`, `a_supersede_while_the_key_is_held_defers_it_on_ack` | |
| S044 | A deferred record restarts at the back of its pocket | edge | `crates/libpachinko/tests/queue/settle.rs`, `a_deferred_record_restarts_at_the_back_of_its_pocket` | |
| S045 | Two supersedes while held: both losers are shed and the held version is untouched (OQ10 as the rules stand: the first loser is the version still held) | edge, oq:10 | `crates/libpachinko/tests/queue/settle.rs`, `two_supersedes_while_held_shed_both_losers_and_leave_the_held_version` | |
| S046 | A stale check while held compares with the latest rank, not the held one | edge, oq:1 | `crates/libpachinko/tests/queue/settle.rs`, `a_stale_check_while_held_compares_with_the_latest_rank_not_the_held_one` | |
| S047 | A nack dead-letters the held version on the first nack, without redelivery | core, wire | `crates/libpachinko/tests/queue/settle.rs`, `a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery` | `crates/pachinko/tests/http.rs`, `a_nack_does_not_redeliver_over_http` (driver `nack_dead_letters`) |
| S048 | A supersede while the key is held defers it on nack, and the lapse frees it | edge | `crates/libpachinko/tests/queue/expiry.rs`, `a_supersede_while_held_is_deferred_on_nack_and_the_lapse_frees_it` | |
| S049 | Rejected nacks change nothing and create no dead letter | edge | `crates/libpachinko/tests/queue/settle.rs`, `rejected_nacks_change_nothing_and_create_no_dead_letter` | |
| S050 | A renew extends the lease to now plus `lease_ttl` | core, wire | `crates/libpachinko/tests/queue/lease.rs`, `a_renew_extends_the_lease_to_now_plus_lease_ttl` | `crates/pachinko/tests/http.rs`, `a_renew_answers_the_new_expiry_over_http` (driver `renew_extends`) |
| S051 | Renewals do not stack | edge | `crates/libpachinko/tests/queue/lease.rs`, `renewals_do_not_stack` | |
| S052 | A renew of an ended lease is rejected and moves nothing | edge | `crates/libpachinko/tests/queue/lease.rs`, `a_renew_of_an_ended_lease_is_rejected_and_moves_nothing` | |
| S053 | A release requeues the pocket at the back and its held records in place | core, wire | `crates/libpachinko/tests/queue/lease.rs`, `a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place` | `crates/pachinko/tests/http.rs`, `a_release_requeues_or_idles_the_pocket_over_http` (driver `release_requeues`) |
| S054 | A release with only deferred records requeues the pocket rather than idling it | edge | `crates/libpachinko/tests/queue/lease.rs`, `a_release_with_only_deferred_records_requeues_the_pocket_rather_than_idling_it` | |
| S055 | A release idles an empty pocket | core | `crates/libpachinko/tests/queue/lease.rs`, `a_release_idles_an_empty_pocket` | |
| S056 | A release resets the consecutive-expiry count | edge | `crates/libpachinko/tests/queue/expiry.rs`, `a_release_resets_the_consecutive_expiry_count` | |
| S057 | A second release is rejected; a release at the grant instant is legal | edge | `crates/libpachinko/tests/queue/lease.rs`, `a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal` | |
| S058 | A released lease never lapses afterwards | edge | `crates/libpachinko/tests/queue/expiry.rs`, `a_released_lease_never_lapses_afterwards` | |
| S059 | A lease lapses exactly at `expires_at` | core | `crates/libpachinko/tests/queue/expiry.rs`, `a_lease_lapses_exactly_at_expires_at` | |
| S060 | A lapse keeps the pocket at the front and requeues its records | core | `crates/libpachinko/tests/queue/expiry.rs`, `a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records` | |
| S061 | A lapse with no records idles the pocket | edge | `crates/libpachinko/tests/queue/expiry.rs`, `a_lapse_with_no_records_idles_the_pocket` | |
| S062 | Three consecutive lapses park the pocket, and a publish revives it | edge | `crates/libpachinko/tests/queue/expiry.rs`, `three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it` | |
| S063 | At the threshold an empty pocket parks rather than idling | edge | `crates/libpachinko/tests/queue/expiry.rs`, `at_the_threshold_an_empty_pocket_parks_rather_than_idling` | |
| S064 | A pull, a nack or a renew under a lease does not reset the expiry count | edge, wire | `crates/libpachinko/tests/queue/expiry.rs`, `a_pull_a_nack_or_a_renew_under_a_lease_does_not_reset_the_expiry_count` | `crates/pachinko/tests/http.rs`, `a_nack_leaves_the_expiry_count_over_http` (driver `nack_keeps_count`; the nack alone) |
| S065 | A lapse fires once per lease | edge | `crates/libpachinko/tests/queue/expiry.rs`, `a_lapse_fires_once_per_lease` | |
| S066 | A renew or release after `expires_at` but before the lapse is processed succeeds (branch A) | decision:0019 | `crates/libpachinko/tests/queue/expiry.rs`, `a_renew_after_expires_at_but_before_the_lapse_succeeds`; release half: `a_release_after_expires_at_but_before_the_lapse_succeeds` | |
| S067 | An ack or pull after `expires_at` but before the lapse is processed succeeds (branch A) | decision:0019 | `crates/libpachinko/tests/queue/expiry.rs`, `an_ack_after_expires_at_but_before_the_lapse_succeeds`; pull half: `a_pull_after_expires_at_but_before_the_lapse_extends_the_lease` | |
| S068 | A consumer that re-claims the same pocket cannot use its old lease | edge, wire | `crates/libpachinko/tests/queue/lease.rs`, `a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease` | `crates/pachinko/tests/http.rs`, `an_old_lease_is_refused_after_the_pocket_is_claimed_again_over_http` (driver `old_lease_fenced`) |
| S069 | After a lapse, the old lease is fenced even when the key has a newer rank | edge | `crates/libpachinko/tests/queue/expiry.rs`, `after_a_lapse_the_old_lease_is_fenced_even_when_the_key_has_a_newer_rank` | |
| S070 | A consumer cannot act on another consumer's lease | edge | `crates/libpachinko/tests/queue/lease.rs`, `a_consumer_cannot_act_on_another_consumers_lease` | |
| S071 | A key acked, then published again within the same lease, is a new queued record (OQ9 as the rules stand, labelled) | oq:9 | `crates/libpachinko/tests/queue/settle.rs`, `oq9_a_key_acked_then_published_again_within_the_lease_is_a_new_queued_record` | |
| S072 | A key nacked, then published again within the same lease, is a new queued record (OQ9 as the rules stand, labelled) | oq:9 | `crates/libpachinko/tests/queue/settle.rs`, `oq9_a_key_nacked_then_published_again_within_the_lease_is_a_new_queued_record` | |
| S073 | Superseded entries decay at `shed_at` plus `superseded_ttl` | core | `crates/libpachinko/tests/queue/decay.rs`, `superseded_entries_decay_at_shed_at_plus_superseded_ttl` | |
| S074 | Dead letters decay at `nacked_at` plus `dead_letter_ttl` | core | `crates/libpachinko/tests/queue/decay.rs`, `dead_letters_decay_at_nacked_at_plus_dead_letter_ttl` | |
| S075 | Pockets and records form one strict, deterministic order | edge | `crates/libpachinko/tests/queue/expiry.rs`, `pockets_and_records_form_one_strict_deterministic_order` | |
| S076 | The consumer view exposes only the active lease and its held versions | core | `crates/libpachinko/tests/queue/lease.rs`, `the_consumer_view_exposes_only_the_active_lease_and_its_held_versions` | |
| S077 | An ack resets the consecutive-expiry count | edge, wire | `crates/libpachinko/tests/queue/expiry.rs`, `an_ack_resets_the_consecutive_expiry_count` | `crates/pachinko/tests/http.rs`, `an_ack_resets_the_expiry_count_over_http` (driver `ack_resets_count`) |

## Coverage

cargo-llvm-cov runs every nextest test under instrumentation, and `just coverage` fails
when line coverage over `crates/*/src/**` falls below 90 per cent. The floor is the
`coverage_floor` variable in the `Justfile`. Doctests are not measured, because
llvm-cov's doctest support needs a nightly toolchain, so `test-doc` runs them uncovered
and the floor is a statement about unit and integration tests. A binary an integration
test runs is instrumented too, so the binary test counts towards the service's lines.
While the crates are small the denominator is too: one untested line moves the
percentage by several points. The recipe also writes `target/llvm-cov/lcov.info`, which
CI keeps as an artifact once the floor has passed.

Coverage counts a snapshot test like any other test. That count proves no specification
clause: every clause needs its own asserting test, even when a snapshot runs the same code.

Distinguish an untested branch from an unreachable one. Defensive code no input can reach
should be deleted rather than covered; see
[Quality philosophy](../explanation/quality-philosophy.md).

## Mutation testing

`just mutants` runs cargo-mutants over both crates with every feature, mutating a copy
of the tree and leaving its report in the ignored `mutants.out/`. The previous report
moves to `mutants.out.old`. Gitignored paths are excluded from the copy. Additional
arguments can be passed through the recipe, for example `just mutants --list`.
It uses `cargo test` so doctests participate; nextest would miss those assertions.

- **Caught:** the mutation compiled and a test failed.
- **Missed:** the changed program still passed its tests; inspect the missing assertion.
- **Unviable:** the mutation did not compile.
- **Timed out:** the test exceeded its time limit; inspect the log before drawing a conclusion.

Exit 0 means every viable mutant was caught, 2 means a survivor, 3 a timeout and 4 a
failing baseline. A red weekly report asks for investigation; it blocks no merge.
The tool never mutates functions named `new`, so constructor logic still needs tests
written by a person. Test code is excluded too. Equivalent mutants may be excluded
through `exclude_re` in `.cargo/mutants.toml`, with a comment explaining why no
observable behaviour changes. Create that file only when needed; do not add the
`mutants` crate or a skip attribute. The policy is explained in
[Quality philosophy](../explanation/quality-philosophy.md).

## Snapshots

No test takes a snapshot yet. The recipes are in place and cargo-insta is pinned with the
other tools, but the `insta` crate becomes a development dependency with the first value
worth snapshotting; until then `just snapshots-check` runs the tests and finds nothing to
compare. The rule below holds from that first snapshot.

A snapshot has two purposes and no other:

1. **Show a reviewer what a value looks like.** It is print-based debugging kept in the
   repository: a reviewer sees the value without running anything.
2. **Cheap change detection.** A change to what an interface hands out appears as a diff
   in a committed file, as a component snapshot shows a changed front end.

**A snapshot never verifies correctness.** The first snapshot records what the code
printed that day; it was not checked against the specification. No clause is proved by
a snapshot. Each clause keeps its own test that asserts, and a ticket's table from
clause to test never names a snapshot test.

Write snapshot tests only after the behaviour's own tests are green, never as the failing
test of a red-green loop. Their names begin `snapshot_`, so readers and filters can
distinguish them. Use a fixed input and a fixed script, never a property-test input.

Render a text picture in test code from what the public surface hands out. Add no
`Display`, method or field to the library to feed a snapshot. A small value, such as an
error, may use its `Debug` or serialised form. Every picture has no trailing whitespace
and ends in a newline.

Snapshots are `.snap` files, never inline in Rust source. They appear as their own diffs,
and acceptance rewrites no source. A test in `src/foo.rs` stores files in
`src/snapshots/`; a test in `src/foo/bar.rs` stores them in `src/foo/snapshots/`; a test
in `tests/foo.rs` stores them in `tests/snapshots/`, beside the source file's directory.
These files are committed. Pending `*.snap.new` and `*.pending-snap` files from runs
outside the recipes are ignored.

`just test`, `just coverage` and `just snapshots-check` fail on a mismatch without
writing a snapshot or pending file. The snapshot gate also rejects a `.snap` file no
test refers to. It runs the entire suite through cargo-insta with nextest and every
feature, so every reference is seen. cargo-insta cannot pass `--locked` to nextest;
the snapshot recipes validate all four locks first and run cargo-insta offline.

A changed snapshot asks whether the change was intended. The person who changed it reads
the diff, reviews pending snapshots through `just snapshots-review` at a terminal, and
accepts intended changes through `just snapshots-accept`, which reruns the tests,
accepts without prompting and deletes any `.snap` file no test refers to. Give the reason
in the pull request or hand-back notes.
The [snapshot workflow](../how-to/test-and-debug.md#read-and-accept-a-changed-snapshot)
explains the commands in order.

## Benchmarks

`just bench` runs two divan benches over libpachinko, with every feature. `version` is
the harness's smoke bench over `libpachinko::version()`. `workloads` runs the queue's
five workloads, W1 to W5, at 10, 1,000 and 100,000 pockets:

| Workload | One sample |
| --- | --- |
| W1 publish | 10,000 publishes of fresh keys, round-robin over the pockets |
| W2 supersede | 10,000 publishes of each pocket's one key at its rank plus one |
| W3 pull 16, ack, release | up to 1,000 cycles of a pull with limit 16, 16 acks and a release |
| W4 publish with every fifth op a consume step | up to 1,000 rounds of four publishes and one W3 cycle |
| W5 pull then lapse on tick | up to 1,000 cycles of a pull, the fake clock past `lease_ttl`, and a tick |

Each sample runs over one `Queue<MemoryRecords, FakeClock, CountingLeaseIds>` with
`Config::default()`, payloads of 64 bytes and the pockets pre-created. divan's
`with_inputs` builds the queue and the sample's publications outside the timed region.
Every call's result passes through `divan::black_box`, and each sample asserts the
outcomes it expects. The unit of one op is one library call on the queue, and divan's
`ItemsCount` counter turns the table's throughput column into ops per second. The bench's
doc comment gives each workload's call count and cycle bound. It also notes that W5's
clock step is timed but not counted. Extra Cargo options may follow the recipe.
[Decision 0019](../decisions/0019-service-runtime-tokio-and-http.md#evidence-from-the-mvp)
records the figures.

Timings depend on the machine and its load, so benchmarks run manually and never in CI
or `just check`. Clippy still compiles the bench targets in the gate.

## Load generation

`just loadgen <workload>` runs the load generator in the excluded `loadgen/` crate,
`pachinko-loadgen`, against a `pachinko serve` that is already running. It speaks HTTP
only and names no workspace crate. Each client is a std thread with its own `ureq`
agent, so it has its own keep-alive connection. The clients run a closed loop: send, read
the whole reply, parse it, count it, send the next. The workloads are `publish`
(single-record `PUT`, C=64), `batch` (batch publish, b=256 at C=8 by default), `consume`
(an untimed fill, then pull, ack each, release, C=64), `mix` (half the clients publish,
half consume) and `lapse`, the lapse probe. Run `just loadgen --help` for the flags and
their defaults. The joins and the fill run before the timed window, and each run's ids
carry the process id and a nonce, so two runs against one service never collide.

A run fails closed. A reply counts only when its status is 2xx and its body parses and
carries the outcome the workload expects. Anything else is an error, and the run prints
it. A run with any error prints no rate, and the process exits non-zero, so a refused
connection or a 404 route can never produce a number. In `consume`, a `no_pocket`
inside the window ends that consumer's loop. The `lapse` run fails in four
cases: the probe pocket is not the front, no batch landed before `expires_at`, the claim
takes more than 60 s, or the queue still holds a pocket after the drain.

The recipe runs `cargo run --locked` against `loadgen/Cargo.lock`, which `just lock-check`
checks beside the workspace, fuzz and pixi lockfiles. The gates `fmt-check` and `clippy` cover the crate
by `--manifest-path`, as they cover `fuzz/`. `loadgen/Cargo.toml` copies the root lint
table, so keep it aligned. `just test-loadgen` runs the crate's own unit tests, which
`just test` cannot reach, and CI's `loadgen` job runs them; they are not a gate in
`just check`. Nothing runs the load generator itself, because timings measure the
machine.

## Fuzzing

`just fuzz` runs the `smoke` target for 300 seconds; `just fuzz smoke 20` selects a
shorter run. Compilation is additional time. The target feeds arbitrary bytes to a
local UTF-8 round-trip check and calls the library’s version function. It is a harness
smoke test, with no claim about a domain parser. A panic fails the run and writes a
reproducer under `fuzz/artifacts/smoke/`.

The recipe uses the pinned stable toolchain with `--sanitizer none`; it does not need
nightly and provides no sanitizer instrumentation. It checks the locks before running
cargo-fuzz offline, since cargo-fuzz cannot take `--locked`. Run `just sync` after a
lock update. `fuzz/` is excluded from the workspace, has its own committed lockfile and
repeats the workspace’s unsafe-code and non-ASCII-identifier forbid levels and Clippy
policy. Keep the copied lint levels aligned with the root table. The existing
`fmt-check`, `clippy` and `lock-check` recipes reach it by `--manifest-path`, as do the
matching format, fix, sync and lock operations. libfuzzer-sys compiles C++, so the gate
needs Xcode command-line tools on macOS or a C++ compiler such as `g++` on Linux.
The cargo-fuzz 0.13.2 macOS binary is x86 only and needs Rosetta 2 on Apple silicon;
the recipe selects the native rustc host triple explicitly.

The separate `fuzz.yml` workflow runs weekly and by hand, for 300 seconds per target,
with crash artifacts uploaded on failure. It is outside the required CI aggregate.
Corpora, crash artifacts, coverage output and builds are ignored; reproducer files must
be promoted to regression tests when domain targets arrive. See
[decision 0017](../decisions/0017-mutation-bench-and-fuzz.md) for the scope and tradeoffs.

## What the current suite proves

| Suite | Covers |
| --- | --- |
| In-module tests, `crates/libpachinko/src/lib.rs` | `the_version_is_the_manifests`: `version` returns the version the manifest states. |
| Doctests | The example on `libpachinko::version`, which asserts the same value from outside the crate. |
| In-module tests, `crates/pachinko/src/lib.rs` | `run_writes_the_name_and_the_library_version`: `run` writes `pachinko`, the library's version and a newline into a buffer. |
| `crates/libpachinko/tests/queue/invariants.rs` | `every_invariant_holds_after_every_step_of_any_walk`: over 512 seeded walks every step's outcome and post-state are the ones the rules predict from the state before it, and all 25 invariants, the declared transitions and the `Consuming` batch guarantees hold after every step. `the_walk_reaches_every_state_and_every_outcome`: the same walks reach every pocket and record state, every outcome and refusal of every trigger, branch A on each lease action, and a renew and a release with the clock behind the queue's `now`. |
| `crates/pachinko/tests/binary.rs` | `the_binary_prints_its_line_and_exits_zero`: the built binary exits zero, prints that line on standard output and writes nothing to standard error. |

## Related pages

- [Test and debug](../how-to/test-and-debug.md)
- [Quality gates](quality-gates.md)
- [Architecture](../explanation/architecture.md)
- [Work with the specifications](../how-to/work-with-the-specs.md)
- [Decision 0009: The Rust quality gate](../decisions/0009-rust-quality-gate.md)
