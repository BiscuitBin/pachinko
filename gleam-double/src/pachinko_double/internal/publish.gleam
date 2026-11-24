//// The specification's Publish section: the Publish table and the
//// `PocketTouched` table.

import gleam/list
import gleam/option.{None, Some}
import gleam/result
import pachinko_double/config
import pachinko_double/internal/domain.{
  type Plan, type Row, type Table, PocketTouched,
}
import pachinko_double/internal/rules.{
  Finish, Impossible, NothingHappens, Now, Position, Refuses, Row, Table,
}
import pachinko_double/model.{
  type Pocket, type PocketStatus, type Reason, type Record, type State, Pocket,
  PocketIdle, PocketParked, PocketQueued, Record, RecordHeld, RecordQueued,
  Stale, State, Superseded, SupersededRecord,
}
import pachinko_double/refusals.{EqualRankRefused}
import pachinko_double/time.{type Timestamp}

/// The input of `Publish(publisher, pocket_key, record_key, rank, payload)`.
/// `publisher` is the publisher's id.
pub type Publication {
  Publication(
    publisher: String,
    pocket_key: String,
    record_key: String,
    rank: Int,
    payload: BitArray,
  )
}

/// `Publish`. The specification gives it five rules with disjoint guards,
/// here in its order, and no rule covers a rank equal to the stored rank:
/// that publish is rejected and changes nothing, whatever its payload (spec
/// note G13).
pub fn publish() -> Table(Publication) {
  Table(
    "Publish",
    [
      publish_into_new_pocket(),
      publish_new_key(),
      supersede(),
      defer_behind_held_version(),
      drop_stale_publish(),
    ],
    Refuses(fn(_state, publication: Publication) {
      EqualRankRefused(
        pocket_key: publication.pocket_key,
        record_key: publication.record_key,
        rank: publication.rank,
      )
    }),
  )
}

/// `Pocket{key: pocket_key}`.
fn pocket(state: State, publication: Publication) -> Result(Pocket, Nil) {
  model.pocket(state.pockets, publication.pocket_key)
}

/// `Record{pocket: pocket, key: record_key}`, where `pocket` is
/// `Pocket{key: pocket_key}`: no pocket, no record.
fn record(state: State, publication: Publication) -> Result(Record, Nil) {
  use pocket <- result.try(pocket(state, publication))
  model.record(state.records, pocket: pocket.key, key: publication.record_key)
}

/// A rule's effect over the record its guard saw. Where that is missing,
/// which only a wrong guard could bring about, the outcome is a defect.
fn stored(
  state: State,
  publication: Publication,
  effect: fn(Record) -> Plan,
) -> Plan {
  case record(state, publication) {
    Error(Nil) ->
      Impossible(
        "no record "
        <> publication.record_key
        <> " in pocket "
        <> publication.pocket_key,
      )
    Ok(record) -> effect(record)
  }
}

/// `SupersededRecord.created(...)` for one version of the publication's key:
/// shed at `now`, kept for `config.superseded_ttl`, and added at the end of
/// the superseded queue.
fn shed(
  state: State,
  publication: Publication,
  rank: Int,
  payload: BitArray,
  reason: Reason,
  now: Timestamp,
) -> State {
  let shed =
    SupersededRecord(
      pocket: publication.pocket_key,
      key: publication.record_key,
      rank:,
      payload:,
      reason:,
      shed_at: now,
      expires_at: now + config.settings(state.config).superseded_ttl,
    )
  State(..state, superseded: list.append(state.superseded, [shed]))
}

/// `PublishIntoNewPocket`. The pocket takes the counter's value and its
/// first record the next one.
fn publish_into_new_pocket() -> Row(Publication) {
  Row(
    "PublishIntoNewPocket",
    fn(state: State, publication: Publication) {
      result.is_error(pocket(state, publication))
    },
    fn(state, publication) {
      use pocket_position <- Position
      use record_position <- Position
      let pocket =
        Pocket(
          key: publication.pocket_key,
          status: PocketQueued,
          position: pocket_position,
          consecutive_expiries: 0,
        )
      let record =
        Record(
          pocket: publication.pocket_key,
          key: publication.record_key,
          rank: publication.rank,
          payload: publication.payload,
          position: record_position,
          status: RecordQueued,
        )
      Finish(
        State(
          ..state,
          pockets: list.append(state.pockets, [pocket]),
          records: list.append(state.records, [record]),
        ),
        [],
        None,
      )
    },
  )
}

/// `PublishNewKey`. It adds a record to an existing pocket, so it touches
/// the pocket.
fn publish_new_key() -> Row(Publication) {
  Row(
    "PublishNewKey",
    fn(state: State, publication: Publication) {
      result.is_ok(pocket(state, publication))
      && result.is_error(record(state, publication))
    },
    fn(state, publication) {
      use position <- Position
      let record =
        Record(
          pocket: publication.pocket_key,
          key: publication.record_key,
          rank: publication.rank,
          payload: publication.payload,
          position:,
          status: RecordQueued,
        )
      Finish(
        State(..state, records: list.append(state.records, [record])),
        [PocketTouched(publication.pocket_key)],
        None,
      )
    },
  )
}

/// `Supersede`. The loser is a version no consumer holds: the record is
/// queued, or it is held with a newer version already deferred behind the
/// held one, and that deferred version is the loser. It is shed at once. The
/// record keeps its position, and a held record its held version, its mark
/// and its arrival position: only its rank and payload change.
fn supersede() -> Row(Publication) {
  Row(
    "Supersede",
    fn(state: State, publication: Publication) {
      case record(state, publication) {
        Error(Nil) -> False
        Ok(record) ->
          publication.rank > record.rank
          && case record.status {
            RecordQueued -> True
            RecordHeld(deferred_position:, ..) -> deferred_position != None
          }
      }
    },
    fn(state, publication) {
      use record <- stored(state, publication)
      use now <- Now
      let state =
        shed(state, publication, record.rank, record.payload, Superseded, now)
      let raised =
        Record(..record, rank: publication.rank, payload: publication.payload)
      Finish(
        State(..state, records: model.replace_record(state.records, raised)),
        [PocketTouched(publication.pocket_key)],
        None,
      )
    },
  )
}

/// `DeferBehindHeldVersion`. The loser is the held version, and nothing
/// about it changes: its held rank and payload, its mark and its nack time
/// stay in the held set until the lease ends. The newer version takes the
/// record's rank and payload and waits behind the held one, given the
/// counter's value as its arrival position. The record keeps its position.
/// The rule reads no `now`.
fn defer_behind_held_version() -> Row(Publication) {
  Row(
    "DeferBehindHeldVersion",
    fn(state: State, publication: Publication) {
      case record(state, publication) {
        Error(Nil) -> False
        Ok(record) ->
          publication.rank > record.rank
          && case record.status {
            RecordQueued -> False
            RecordHeld(deferred_position:, ..) -> deferred_position == None
          }
      }
    },
    fn(state, publication) {
      use record <- stored(state, publication)
      case record.status {
        RecordQueued ->
          Impossible(
            "record "
            <> publication.record_key
            <> " in pocket "
            <> publication.pocket_key
            <> " is not held",
          )
        RecordHeld(..) as held -> {
          use position <- Position
          let deferred =
            Record(
              ..record,
              rank: publication.rank,
              payload: publication.payload,
              status: RecordHeld(..held, deferred_position: Some(position)),
            )
          Finish(
            State(
              ..state,
              records: model.replace_record(state.records, deferred),
            ),
            [PocketTouched(publication.pocket_key)],
            None,
          )
        }
      }
    },
  )
}

/// `DropStalePublish`. The stored rank is the record's, whatever its
/// status. The stale version itself is shed; no record changes, and the
/// pocket is not touched.
fn drop_stale_publish() -> Row(Publication) {
  Row(
    "DropStalePublish",
    fn(state: State, publication: Publication) {
      case record(state, publication) {
        Error(Nil) -> False
        Ok(record) -> publication.rank < record.rank
      }
    },
    fn(state, publication) {
      use now <- Now
      Finish(
        shed(
          state,
          publication,
          publication.rank,
          publication.payload,
          Stale,
          now,
        ),
        [],
        None,
      )
    },
  )
}

/// `PocketTouched(pocket_key)`. The two revive rules answer it only when
/// the pocket is idle or parked; otherwise nothing happens.
pub fn pocket_touched() -> Table(String) {
  Table(
    "PocketTouched",
    [revive_idle_pocket(), revive_parked_pocket()],
    NothingHappens,
  )
}

/// `ReviveIdlePocket`. A publish left a record in an idle pocket: the
/// pocket is queued again, at a fresh position at the back of the pocket
/// order, with its count of consecutive expiries reset. The position is
/// allotted after the one the publish gave its record.
fn revive_idle_pocket() -> Row(String) {
  revive("ReviveIdlePocket", PocketIdle)
}

/// `ReviveParkedPocket`. A publish added or raised a record in a parked
/// pocket: the pocket is queued again as an idle one is, at a fresh
/// position at the back, with its count of consecutive expiries reset. Only
/// a publish touches a pocket, so nothing else revives a parked one, as the
/// rules stand (open question 7).
fn revive_parked_pocket() -> Row(String) {
  revive("ReviveParkedPocket", PocketParked)
}

/// The two revive rules, which differ in the status they require:
///
/// ```text
/// requires: exists pocket
/// requires: pocket.status = idle        -- or parked
/// let position = queue.next_position
/// ensures: pocket.status = queued
/// ensures: pocket.position = position
/// ensures: pocket.consecutive_expiries = 0
/// ensures: queue.next_position = position + 1
/// ```
fn revive(name: String, from: PocketStatus) -> Row(String) {
  Row(
    name,
    fn(state: State, pocket_key: String) {
      case model.pocket(state.pockets, pocket_key) {
        Error(Nil) -> False
        Ok(pocket) -> pocket.status == from
      }
    },
    fn(state, pocket_key) {
      case model.pocket(state.pockets, pocket_key) {
        Error(Nil) -> Impossible("no pocket " <> pocket_key)
        Ok(pocket) -> {
          use position <- Position
          let queued =
            Pocket(
              ..pocket,
              status: PocketQueued,
              position:,
              consecutive_expiries: 0,
            )
          Finish(
            State(..state, pockets: model.replace_pocket(state.pockets, queued)),
            [],
            None,
          )
        }
      }
    },
  )
}
