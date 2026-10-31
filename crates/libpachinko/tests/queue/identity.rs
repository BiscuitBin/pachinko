//! The config block, the five config invariants, the `Queue` entity and the two
//! identity rules.

use std::time::Duration;

use libpachinko::clock::FakeClock;
use libpachinko::config::Config;
use libpachinko::error::{ConfigError, JoinError};
use libpachinko::queue::Queue;
use libpachinko::records::MemoryRecords;

use crate::fixture::Fixture;

// obligation: config-default.lease_ttl
// obligation: config-default.max_consecutive_expiries
// obligation: config-default.max_pull_batch
// obligation: config-default.superseded_ttl
// obligation: config-default.dead_letter_ttl
#[test]
fn the_config_defaults_are_the_modules() {
    let config = Config::default();
    assert_eq!(config.lease_ttl, Duration::from_mins(5));
    assert_eq!(config.max_consecutive_expiries, 3);
    assert_eq!(config.max_pull_batch, 100);
    assert_eq!(config.superseded_ttl, Duration::from_hours(12));
    assert_eq!(config.dead_letter_ttl, Duration::from_hours(12));
}

// obligation: invariant.ExpiryLimitPositive
#[test]
fn a_queue_refuses_an_expiry_limit_below_one() {
    let mut config = Config::default();
    config.max_consecutive_expiries = 0;
    let built = Queue::<MemoryRecords, _>::new(config, FakeClock::default());
    assert_eq!(built.err(), Some(ConfigError::ExpiryLimitNotPositive));
    config.max_consecutive_expiries = 1;
    assert!(Queue::<MemoryRecords, _>::new(config, FakeClock::default()).is_ok());
}

// obligation: invariant.PullBatchPositive
#[test]
fn a_queue_refuses_a_pull_batch_below_one() {
    let mut config = Config::default();
    config.max_pull_batch = 0;
    let built = Queue::<MemoryRecords, _>::new(config, FakeClock::default());
    assert_eq!(built.err(), Some(ConfigError::PullBatchNotPositive));
    config.max_pull_batch = 1;
    assert!(Queue::<MemoryRecords, _>::new(config, FakeClock::default()).is_ok());
}

// obligation: invariant.LeaseTtlNonNegative
// obligation: invariant.SupersededTtlNonNegative
// obligation: invariant.DeadLetterTtlNonNegative
// The three hold by representation: a `Duration` cannot be negative. Zero, the
// boundary the module admits, is accepted.
#[test]
fn zero_ttls_are_admitted_and_negative_ones_cannot_be_written() {
    let mut config = Config::default();
    config.lease_ttl = Duration::ZERO;
    config.superseded_ttl = Duration::ZERO;
    config.dead_letter_ttl = Duration::ZERO;
    assert!(Queue::<MemoryRecords, _>::new(config, FakeClock::default()).is_ok());
    assert!(
        Duration::ZERO
            .checked_sub(Duration::from_millis(1))
            .is_none()
    );
}

// obligation: entity-fields.Queue
#[test]
fn a_new_queue_starts_its_positions_at_zero() {
    let fixture = Fixture::new();
    assert_eq!(fixture.snapshot().next_position, 0);
}

// obligation: rule-success.PublisherJoins
// obligation: rule-entity-creation.PublisherJoins.1
// obligation: entity-fields.Publisher
#[test]
fn a_publisher_joins_with_its_id() {
    let mut fixture = Fixture::new();
    fixture.queue.publisher_joins("p2").unwrap();
    assert_eq!(fixture.snapshot().publishers, ["p", "p2"]);
}

// obligation: rule-failure.PublisherJoins.1
// obligation: invariant.UniquePublisherId
#[test]
fn a_publisher_id_joins_once() {
    let mut fixture = Fixture::new();
    let before = fixture.snapshot();
    assert_eq!(
        fixture.queue.publisher_joins("p"),
        Err(JoinError::AlreadyJoined("p".to_owned()))
    );
    assert_eq!(fixture.snapshot(), before);
}

// obligation: rule-success.ConsumerJoins
// obligation: rule-entity-creation.ConsumerJoins.1
// obligation: entity-fields.Consumer
#[test]
fn a_consumer_joins_with_its_id_and_no_lease() {
    let mut fixture = Fixture::new();
    fixture.queue.consumer_joins("c3").unwrap();
    let consumers = fixture.snapshot().consumers;
    assert!(consumers.contains(&("c3".to_owned(), None)));
}

// obligation: rule-failure.ConsumerJoins.1
// obligation: invariant.UniqueConsumerId
#[test]
fn a_consumer_id_joins_once() {
    let mut fixture = Fixture::new();
    let before = fixture.snapshot();
    assert_eq!(
        fixture.queue.consumer_joins("c1"),
        Err(JoinError::AlreadyJoined("c1".to_owned()))
    );
    assert_eq!(fixture.snapshot(), before);
}

// A publisher id and a consumer id live in separate namespaces.
#[test]
fn a_publisher_and_a_consumer_may_share_an_id() {
    let mut fixture = Fixture::new();
    assert!(fixture.queue.consumer_joins("p").is_ok());
    assert!(fixture.queue.publisher_joins("c1").is_ok());
}
