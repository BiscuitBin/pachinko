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
}
