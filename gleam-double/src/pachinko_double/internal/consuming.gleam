//// What the `Consuming` surface exposes to a consumer: its lease view. A
//// read, not a trigger: it goes through no table, reads no clock and
//// changes nothing.

import gleam/list
import gleam/option.{type Option}
import pachinko_double/answers.{type LeaseView, HeldView, LeaseView}
import pachinko_double/inspection
import pachinko_double/model.{type State, RecordHeld, RecordQueued}

/// The view of the consumer's active lease, or `None` when it has none:
/// `Leases where consumer = consumer and status = active`, which the
/// invariants make zero or one lease. `consumer` is the consumer's id.
///
/// It is built from the snapshot's derived sets: the consumer's
/// `active_leases`, and that lease's pocket's `held_records`. The surface
/// exposes the held records as a set; the view lists them smallest position
/// first, the order a pull handed them out in.
pub fn lease_view(state: State, consumer: String) -> Option(LeaseView) {
  let views = inspection.views(inspection.snapshot(state))
  views.consumers
  |> list.filter(fn(view) { view.consumer.id == consumer })
  |> list.flat_map(fn(view) { view.active_leases })
  |> list.first
  |> option.from_result
  |> option.map(fn(lease) {
    let records =
      views.pockets
      |> list.filter(fn(view) { view.pocket.key == lease.pocket })
      |> list.flat_map(fn(view) { view.held_records })
      |> model.oldest_first
      |> list.filter_map(fn(record) {
        case record.status {
          RecordQueued -> Error(Nil)
          RecordHeld(held_rank:, held_payload:, mark:, ..) ->
            Ok(HeldView(key: record.key, held_rank:, held_payload:, mark:))
        }
      })
    LeaseView(pocket_key: lease.pocket, expires_at: lease.expires_at, records:)
  })
}
