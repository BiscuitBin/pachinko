//// The specification's `config` block and its five invariants: S003, S004
//// and S005. A refused configuration leaves no owner to inspect, so each
//// test asserts the refusal itself (spec note G02). Then two rows on the
//// values a configuration holds: S001, the defaults an owner starts over,
//// and S008, a `max_pull_batch` of one, which a pull shows.

import gleam/option.{None}
import gleam/result
import harness
import pachinko_double/config.{type Settings, Settings}
import pachinko_double/owner.{Outcome}
import pachinko_double/refusals.{ConfigRefused}
import pachinko_double/time
import runner.{type Test, Scenario, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a zero expiry limit is refused",
      [Scenario("S003")],
      zero_expiry_limit,
    ),
    Test("a zero pull batch is refused", [Scenario("S004")], zero_pull_batch),
    Test("a negative TTL is refused", [Scenario("S005")], negative_ttls),
    Test(
      "the default lease_ttl and max_pull_batch apply",
      [Scenario("S001")],
      default_lease_ttl_and_pull_batch,
    ),
    Test(
      "a max_pull_batch of one caps every pull at one record",
      [Scenario("S008")],
      pull_batch_of_one,
    ),
    Test("a negative limit is refused", [], negative_limits),
    Test("the defaults are the specification's and are admitted", [], defaults),
  ]
}

/// The settings are admitted, and an owner starts over them and answers an
/// inspection: the Rust's "a queue is built". Nothing has joined, and the
/// queue's first position is not this row's to pin (spec note G04).
fn serves(settings: Settings) -> Bool {
  case config.new(settings) {
    Error(_) -> False
    Ok(config) -> {
      let snapshot = harness.inspect(harness.start(config))
      snapshot.publishers == [] && snapshot.consumers == []
    }
  }
}

fn zero_expiry_limit() -> List(String) {
  let settings = Settings(..config.defaults(), max_consecutive_expiries: 0)
  assert result.map(config.new(settings), config.settings)
    == Error(ConfigRefused(["ExpiryLimitPositive"]))
  assert serves(Settings(..settings, max_consecutive_expiries: 1))
  []
}

fn zero_pull_batch() -> List(String) {
  let settings = Settings(..config.defaults(), max_pull_batch: 0)
  assert result.map(config.new(settings), config.settings)
    == Error(ConfigRefused(["PullBatchPositive"]))
  assert serves(Settings(..settings, max_pull_batch: 1))
  []
}

/// `impl-choice`: the Rust refuses a negative TTL at compile time, and tags
/// only `lease_ttl`'s doctest with S005. Here each of the three is refused
/// when the configuration is made, and zero is admitted.
fn negative_ttls() -> List(String) {
  let refused = fn(settings) {
    result.map(config.new(settings), config.settings)
  }
  let defaults = config.defaults()
  assert refused(Settings(..defaults, lease_ttl: -time.millisecond))
    == Error(ConfigRefused(["LeaseTtlNonNegative"]))
  assert refused(Settings(..defaults, superseded_ttl: -time.millisecond))
    == Error(ConfigRefused(["SupersededTtlNonNegative"]))
  assert refused(Settings(..defaults, dead_letter_ttl: -time.millisecond))
    == Error(ConfigRefused(["DeadLetterTtlNonNegative"]))
  assert refused(Settings(
      lease_ttl: -1,
      max_consecutive_expiries: 0,
      max_pull_batch: 0,
      superseded_ttl: -1,
      dead_letter_ttl: -1,
    ))
    == Error(
      ConfigRefused([
        "ExpiryLimitPositive", "PullBatchPositive", "LeaseTtlNonNegative",
        "SupersededTtlNonNegative", "DeadLetterTtlNonNegative",
      ]),
    )
  assert serves(
    Settings(..defaults, lease_ttl: 0, superseded_ttl: 0, dead_letter_ttl: 0),
  )
  []
}

/// The Rust builds a queue from the default configuration and reads the two
/// values back from it. Here an owner is started over the default
/// configuration, and its snapshot holds them. No rule fires.
fn default_lease_ttl_and_pull_batch() -> List(String) {
  let snapshot = harness.inspect(harness.start(config.default()))
  assert snapshot.config.lease_ttl == 5 * time.minute
  assert snapshot.config.max_pull_batch == 100
  []
}

/// Whatever the limit, 100, 2 or 1, each pull hands out one record: the
/// batch size is `min(limit, config.max_pull_batch)`.
fn pull_batch_of_one() -> List(String) {
  let double =
    harness.fixture_with(
      Settings(..harness.fixture_settings(), max_pull_batch: 1),
    )
  assert harness.publish(double, "pk", "a", 1, "a")
    == Ok(Outcome(["PublishIntoNewPocket"], None))
  assert harness.publish(double, "pk", "b", 1, "b")
    == Ok(Outcome(["PublishNewKey"], None))
  assert harness.publish(double, "pk", "c", 1, "c")
    == Ok(Outcome(["PublishNewKey"], None))
  let first = harness.batch(double, "c1", 100)
  assert first.fired == ["PullClaimsFrontPocket", "HoldRecord"]
  assert harness.keys(first.records) == ["a"]
  let second = harness.batch(double, "c1", 2)
  assert second.fired == ["PullWithinLease", "HoldRecord"]
  assert harness.keys(second.records) == ["b"]
  let third = harness.batch(double, "c1", 1)
  assert third.fired == ["PullWithinLease", "HoldRecord"]
  assert harness.keys(third.records) == ["c"]
  harness.fired(double)
}

/// The two limits are positive, not merely non-zero. No catalog row names a
/// negative limit, so this test pins none.
fn negative_limits() -> List(String) {
  let refused = fn(settings) {
    result.map(config.new(settings), config.settings)
  }
  let defaults = config.defaults()
  assert refused(Settings(..defaults, max_consecutive_expiries: -1))
    == Error(ConfigRefused(["ExpiryLimitPositive"]))
  assert refused(Settings(..defaults, max_pull_batch: -1))
    == Error(ConfigRefused(["PullBatchPositive"]))
  []
}

fn defaults() -> List(String) {
  assert config.defaults()
    == Settings(
      lease_ttl: 300_000,
      max_consecutive_expiries: 3,
      max_pull_batch: 100,
      superseded_ttl: 43_200_000,
      dead_letter_ttl: 43_200_000,
    )
  assert config.new(config.defaults()) == Ok(config.default())
  []
}
