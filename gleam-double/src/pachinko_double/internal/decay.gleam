//// The specification's "Decay" section: `SupersededRecordDecays` and
//// `DeadLetterDecays`, each the one rule of a clock trigger, an entry whose
//// `expires_at <= now`.
////
//// Only a `Tick` reaches these tables. It offers each entry whose
//// `expires_at` has passed, so that comparison is the tick's filter. The
//// rules have no `requires`, so each guard holds while the entry the
//// trigger binds is still in its queue, and when none holds nothing
//// happens. Neither rule asks for the time.
////
//// Two entries can be equal: two stale publishes of one key at one rank at
//// one instant shed two identical `SupersededRecord`s. The trigger binds
//// one entry, so a decay removes one occurrence, the first equal one, and
//// keeps every other entry in its order.

import gleam/list
import gleam/option.{None}
import pachinko_double/internal/domain.{type Table}
import pachinko_double/internal/rules.{Finish, NothingHappens, Row, Table}
import pachinko_double/model.{
  type DeadLetter, type State, type SupersededRecord, State,
}

/// The clock trigger `shed: SupersededRecord.expires_at <= now`, with its
/// one rule.
pub fn superseded_record_decays() -> Table(SupersededRecord) {
  Table(
    "SupersededRecordDecays",
    [
      Row(
        "SupersededRecordDecays",
        fn(state: State, shed: SupersededRecord) {
          list.contains(state.superseded, shed)
        },
        fn(state, shed) {
          // ensures: not exists shed
          Finish(
            State(..state, superseded: without_one(state.superseded, shed)),
            [],
            None,
          )
        },
      ),
    ],
    NothingHappens,
  )
}

/// The clock trigger `letter: DeadLetter.expires_at <= now`, with its one
/// rule.
pub fn dead_letter_decays() -> Table(DeadLetter) {
  Table(
    "DeadLetterDecays",
    [
      Row(
        "DeadLetterDecays",
        fn(state: State, letter: DeadLetter) {
          list.contains(state.dead_letters, letter)
        },
        fn(state, letter) {
          // ensures: not exists letter
          Finish(
            State(
              ..state,
              dead_letters: without_one(state.dead_letters, letter),
            ),
            [],
            None,
          )
        },
      ),
    ],
    NothingHappens,
  )
}

/// The entries without the first one equal to `entry`, the others in the
/// order they had. An entry that is not there leaves them as they were.
fn without_one(entries: List(a), entry: a) -> List(a) {
  case entries {
    [] -> []
    [first, ..rest] if first == entry -> rest
    [first, ..rest] -> [first, ..without_one(rest, entry)]
  }
}
