//// What a reply carries: a pull's answer, a tick's report, and the lease
//// view the `Consuming` surface exposes to a consumer. The owner hands each
//// back as it is; none of them is a trigger.

import pachinko_double/ids.{
  type ConsumerId, type LeaseId, type PocketKey, type RecordKey,
}
import pachinko_double/model.{
  type DeadLetter, type Lease, type Mark, type SupersededRecord,
}
import pachinko_double/time.{type Timestamp}

/// What a rule answers its caller. A pull answers one of these; every other
/// trigger answers nothing, an ack, a nack and a release included.
pub type Answer {
  /// What a pull hands back. `consumer` is the consumer's id, `lease` the id
  /// of the lease the records were handed under and `pocket` the pocket's
  /// key. It carries no expiry: the specification's `Batch` has none.
  /// `records` is the batch, smallest position first.
  Batch(
    consumer: ConsumerId,
    lease: LeaseId,
    pocket: PocketKey,
    records: List(BatchRecord),
  )
  /// The consumer holds no lease and no pocket is queued. An answer, not a
  /// refusal.
  NoPocketAvailable(consumer: ConsumerId)
}

/// One version a pull handed out: the record's key, and the rank and payload
/// it had when the batch was computed.
pub type BatchRecord {
  BatchRecord(key: RecordKey, rank: Int, payload: BitArray)
}

/// What a tick did. `fired` is the names of the rules every outcome that
/// succeeded fired, in order. `lapsed` is the leases it lapsed, in the order
/// they lapsed, as the state keeps them once ended. `superseded` and
/// `dead_letters` are the entries that decayed, in the order they decayed,
/// as they stood.
pub type TickReport {
  TickReport(
    fired: List(String),
    lapsed: List(Lease),
    superseded: List(SupersededRecord),
    dead_letters: List(DeadLetter),
  )
}

/// The consumer's active lease as the surface exposes it: `lease.pocket.key`,
/// `lease.expires_at`, and the pocket's held records. It carries no lease
/// id: the surface exposes none.
pub type LeaseView {
  LeaseView(
    pocket_key: PocketKey,
    expires_at: Timestamp,
    records: List(HeldView),
  )
}

/// One held record as the surface exposes it: `record.key`,
/// `record.held_rank`, `record.held_payload` and `record.mark`.
pub type HeldView {
  HeldView(key: RecordKey, held_rank: Int, held_payload: BitArray, mark: Mark)
}
