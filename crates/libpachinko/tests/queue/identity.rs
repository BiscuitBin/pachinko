//! Identity and config: `PublisherJoins`, `ConsumerJoins`, the unknown-party
//! refusals, the defaults and the config load checks.

use std::time::Duration;

use libpachinko::clock::FakeClock;
use libpachinko::config::Config;
use libpachinko::error::{ConfigError, JoinError, PublishError, PullError};
use libpachinko::ids::{ConsumerId, Payload, PocketKey, PublisherId, Rank, RecordKey};
use libpachinko::lease_ids::CountingLeaseIds;
use libpachinko::queue::{Publication, PullOutcome, Queue};
use libpachinko::records::MemoryRecords;

use crate::fixture::{Fixture, config, invariants, keys};

fn build(config: Config) -> Result<Queue<MemoryRecords, FakeClock, CountingLeaseIds>, ConfigError> {
    Queue::new(config, FakeClock::default(), CountingLeaseIds::default())
}

// scenario: S001
// obligation: config-default.lease_ttl
// obligation: config-default.max_pull_batch
#[test]
fn the_default_lease_ttl_and_pull_batch_apply() {
    let queue = build(Config::default()).unwrap();
    assert_eq!(queue.config().lease_ttl, Duration::from_mins(5));
    assert_eq!(queue.config().max_pull_batch, 100);
}

// scenario: S002
// obligation: config-default.max_consecutive_expiries
// obligation: config-default.superseded_ttl
// obligation: config-default.dead_letter_ttl
#[test]
fn the_default_parking_threshold_and_decay_ttls_apply() {
    let queue = build(Config::default()).unwrap();
    assert_eq!(queue.config().max_consecutive_expiries, 3);
    assert_eq!(queue.config().superseded_ttl, Duration::from_hours(12));
    assert_eq!(queue.config().dead_letter_ttl, Duration::from_hours(12));
}

// scenario: S003
// obligation: invariant.ExpiryLimitPositive
#[test]
fn a_zero_expiry_limit_is_refused() {
    let mut config = Config::default();
    config.max_consecutive_expiries = 0;
    assert_eq!(
        build(config.clone()).err(),
        Some(ConfigError::MaxConsecutiveExpiriesZero)
    );
    config.max_consecutive_expiries = 1;
    assert!(build(config).is_ok());
}

// scenario: S004
// obligation: invariant.PullBatchPositive
#[test]
fn a_zero_pull_batch_is_refused() {
    let mut config = Config::default();
    config.max_pull_batch = 0;
    assert_eq!(
        build(config.clone()).err(),
        Some(ConfigError::MaxPullBatchZero)
    );
    config.max_pull_batch = 1;
    assert!(build(config).is_ok());
}

// scenario: S008
// obligation: guarantee.OldestFirst
#[test]
fn a_pull_batch_of_one_caps_every_pull_at_one_record() {
    let mut config = config();
    config.max_pull_batch = 1;
    let mut fixture = Fixture::with_config(config);
    for key in ["a", "b", "c"] {
        fixture.publish("pk", key, 1, key).unwrap();
    }
    assert_eq!(keys(&fixture.pull("c1", 100).records), ["a"]);
    assert_eq!(keys(&fixture.pull("c1", 2).records), ["b"]);
    assert_eq!(keys(&fixture.pull("c1", 1).records), ["c"]);
}

// scenario: S009
// obligation: rule-success.PublisherJoins
// obligation: rule-failure.PublisherJoins.1
// obligation: rule-entity-creation.PublisherJoins.1
// obligation: entity-fields.Publisher
// obligation: invariant.UniquePublisherId
#[test]
fn a_publisher_joins_once() {
    let mut fixture = Fixture::new();
    let id = PublisherId::new("q");
    assert_eq!(fixture.queue.publisher_joins(id.clone()), Ok(()));
    let before = fixture.snapshot();
    assert_eq!(
        fixture.queue.publisher_joins(id.clone()),
        Err(JoinError::PublisherExists(id.clone()))
    );
    let after = fixture.snapshot();
    assert_eq!(after, before);
    assert_eq!(after.publishers, [PublisherId::new("p"), id]);
    invariants::unique_publisher_id(&after);
}

// scenario: S010
// obligation: rule-success.ConsumerJoins
// obligation: rule-failure.ConsumerJoins.1
// obligation: rule-entity-creation.ConsumerJoins.1
// obligation: entity-fields.Consumer
// obligation: invariant.UniqueConsumerId
#[test]
fn a_consumer_joins_once() {
    let mut fixture = Fixture::new();
    let id = ConsumerId::new("c4");
    assert_eq!(fixture.queue.consumer_joins(id.clone()), Ok(()));
    let before = fixture.snapshot();
    assert_eq!(
        fixture.queue.consumer_joins(id.clone()),
        Err(JoinError::ConsumerExists(id.clone()))
    );
    let after = fixture.snapshot();
    assert_eq!(after, before);
    assert!(after.consumers.contains(&(id, None)));
    invariants::unique_consumer_id(&after);
}

// obligation: surface-actor.Publishing
#[test]
fn a_publish_by_a_publisher_that_never_joined_is_refused_and_changes_nothing() {
    let mut fixture = Fixture::new();
    let before = fixture.snapshot();
    let stranger = PublisherId::new("stranger");
    let publication = Publication::new(
        PocketKey::new("pk"),
        RecordKey::new("a"),
        Rank::new(1),
        Payload::default(),
    );
    assert_eq!(
        fixture.queue.publish(&stranger, publication),
        Err(PublishError::UnknownPublisher(stranger))
    );
    assert_eq!(fixture.snapshot(), before);
}

// obligation: surface-actor.Consuming
#[test]
fn a_pull_or_lease_read_by_a_consumer_that_never_joined_is_refused_and_changes_nothing() {
    let mut fixture = Fixture::new();
    fixture.publish("pk", "a", 1, "a").unwrap();
    let before = fixture.snapshot();
    let stranger = ConsumerId::new("stranger");
    assert_eq!(
        fixture.queue.pull(&stranger, 10),
        Err(PullError::UnknownConsumer(stranger.clone()))
    );
    assert_eq!(
        fixture.queue.lease_view(&stranger),
        Err(PullError::UnknownConsumer(stranger))
    );
    assert_eq!(fixture.snapshot(), before);
    assert!(matches!(
        fixture.try_pull("c1", 10),
        Ok(PullOutcome::Batch { .. })
    ));
}
