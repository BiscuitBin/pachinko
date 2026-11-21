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
}
