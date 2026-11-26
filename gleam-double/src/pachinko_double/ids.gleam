//// The ids and keys a caller names. The specification declares each as a
//// `String` and states no format, so each type here wraps any `String` and
//// checks nothing: a caller makes one with its constructor and reads one by
//// matching it. Each is a type of its own so that one cannot be passed where
//// another is expected.

/// A publisher's id: `Publisher.id`, and the `publisher_id` and `publisher`
/// the `Publishing` surface's triggers carry.
pub type PublisherId {
  PublisherId(String)
}

/// A consumer's id: `Consumer.id`, the `consumer_id` and `consumer` the
/// `Consuming` surface's triggers carry, and a reference to a consumer.
pub type ConsumerId {
  ConsumerId(String)
}

/// A pocket's key: `Pocket.key`, the `pocket_key` a publish carries, and a
/// reference to a pocket.
pub type PocketKey {
  PocketKey(String)
}

/// A record's key within its pocket: `Record.key`, the `record_key` a
/// publish, an ack and a nack carry, and the key of a shed version.
pub type RecordKey {
  RecordKey(String)
}

/// A lease's id. The specification declares none; the double gives each
/// lease one, drawn from the owner's lease-id function (spec note G10).
pub type LeaseId {
  LeaseId(String)
}
