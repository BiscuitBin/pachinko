//// The refusals. The specification rejects an action without naming the
//// rejection, so the names are the double's, and they follow the Python
//// double's spec notes where a note names one, so that the two doubles'
//// findings line up.

pub type Refusal {
  /// The configuration breaks these config invariants. No queue is made
  /// from it (spec note G02).
  ConfigRefused(invariants: List(String))
  /// A publisher with this id has already joined (spec note G03).
  PublisherAlreadyJoined(publisher_id: String)
  /// A consumer with this id has already joined (spec note G03).
  ConsumerAlreadyJoined(consumer_id: String)
  /// No publisher with this id has joined, so it cannot publish. Refused
  /// before any rule runs (spec note G12).
  UnknownPublisher(publisher_id: String)
  /// The publish carries the rank the record already has. No Publish rule
  /// covers that, whatever the payload (spec note G13).
  EqualRankRefused(pocket_key: String, record_key: String, rank: Int)
  /// No consumer with this id has joined, so it can neither pull, ack, nack,
  /// renew nor release, nor read a lease. Refused before any rule runs.
  UnknownConsumer(consumer_id: String)
  /// No Pull rule's guard held. In a state that holds the invariants one of
  /// the three always does, so no pull through the owner meets this.
  PullRefused(consumer_id: String)
  /// The action names a lease that is not active, is not the consumer's, or
  /// was never issued. `action` is the trigger's name.
  LeaseFenced(action: String, consumer_id: String, lease_id: String)
  /// The lease is active and the consumer's, and its pocket holds no held
  /// record with this key: the pocket has no such record, or the record is
  /// queued. `action` is the trigger's name, `Ack` or `Nack`.
  RecordNotHeld(action: String, lease_id: String, record_key: String)
  /// The record is held under the lease and already carries the other mark:
  /// an ack of a nacked record, or a nack of an acked one. No rule covers
  /// either, and the first mark stands. `action` is the trigger's name, so
  /// `Ack` says the record is nacked and `Nack` that it is acked.
  MarkConflict(action: String, lease_id: String, record_key: String)
  /// No Release rule's guard held though the lease is active and the
  /// consumer's: its pocket is not leased. In a state that holds the
  /// invariants one of the two always does, so no release through the owner
  /// meets this.
  ReleaseRefused(consumer_id: String, lease_id: String)
}
