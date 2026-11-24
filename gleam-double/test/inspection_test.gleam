//// The snapshot of a state, field by field, and the snapshot's derived
//// sets: `inspection.views` on snapshots built by hand, so that every set
//// is seen beside members it must leave out. Each test asserts the whole
//// value it reads, so a collection or a set that gained or lost a member,
//// or changed its order, fails it.

import gleam/list
import gleam/option.{type Option, None, Some}
import pachinko_double/config.{Settings}
import pachinko_double/inspection.{
  type Snapshot, ConsumerView, PocketView, Snapshot, Views,
}
import pachinko_double/model.{
  type Lease, type LeaseStatus, type Mark, type Pocket, type Record, Acked,
  Consumer, DeadLetter, Lease, LeaseActive, LeaseExpired, LeaseReleased, Nacked,
  Pocket, PocketLeased, PocketQueued, Publisher, Queue, Record, RecordHeld,
  RecordQueued, Stale, State, Superseded, SupersededRecord, Unsettled,
}
import support/runner.{type Test, Test}

pub fn tests() -> List(Test) {
  [
    Test(
      "a state's snapshot holds every field of the state, each list as it is",
      [],
      snapshot_of_state,
    ),
    Test("a new queue has no view", [], nothing_to_view),
    Test(
      "a pocket's and a consumer's derived sets, in the snapshot's order",
      [],
      derived_sets,
    ),
    Test(
      "a key or an id several entities hold gets a view for each",
      [],
      repeated_keys,
    ),
    Test("a reference that names nothing is in no view", [], names_nothing),
  ]
}

// The pockets, by key.
const busy = "pocket-busy"

const quiet = "pocket-quiet"

fn busy_pocket() -> Pocket {
  Pocket(key: busy, status: PocketLeased, position: 0, consecutive_expiries: 0)
}

fn quiet_pocket() -> Pocket {
  Pocket(key: quiet, status: PocketQueued, position: 6, consecutive_expiries: 0)
}

fn queued(pocket: String, key: String, position: Int) -> Record {
  Record(
    pocket:,
    key:,
    rank: 1,
    payload: <<key:utf8>>,
    position:,
    status: RecordQueued,
  )
}

/// A record of the busy pocket, held at rank 1 with this mark. With a
/// deferred position, its rank has moved on to 2.
fn held(
  key: String,
  position: Int,
  mark: Mark,
  deferred_position: Option(Int),
) -> Record {
  let rank = case deferred_position {
    None -> 1
    Some(_) -> 2
  }
  let nacked_at = case mark {
    Nacked -> Some(700)
    Unsettled | Acked -> None
  }
  Record(
    pocket: busy,
    key:,
    rank:,
    payload: <<key:utf8>>,
    position:,
    status: RecordHeld(
      held_rank: 1,
      held_payload: <<key:utf8>>,
      mark:,
      nacked_at:,
      deferred_position:,
    ),
  )
}

// The busy pocket's records, and the quiet pocket's one.

fn waiting() -> Record {
  queued(busy, "waiting", 1)
}

fn unsettled() -> Record {
  held("unsettled", 2, Unsettled, None)
}

fn elsewhere() -> Record {
  queued(quiet, "waiting", 7)
}

fn acked() -> Record {
  held("acked", 3, Acked, None)
}

fn nacked() -> Record {
  held("nacked", 4, Nacked, None)
}

fn acked_with_deferred() -> Record {
  held("acked-with-deferred", 5, Acked, Some(8))
}

fn lease(
  id: String,
  pocket: String,
  consumer: String,
  status: LeaseStatus,
) -> Lease {
  Lease(id:, pocket:, consumer:, status:, granted_at: 100, expires_at: 900)
}

// `con-a` has held the busy pocket twice: a lease that expired, and the
// active one. `con-b` held the quiet pocket once and released it.

fn expired_lease() -> Lease {
  lease("lease-1", busy, "con-a", LeaseExpired(ended_at: 500))
}

fn released_lease() -> Lease {
  lease("lease-2", quiet, "con-b", LeaseReleased(ended_at: 150))
}

fn active_lease() -> Lease {
  lease("lease-3", busy, "con-a", LeaseActive)
}

/// Two pockets and three consumers. The busy pocket holds one record of
/// each kind a derived set tells apart: queued; held and unsettled; held and
/// acked; held and nacked; and held, acked, with a newer version deferred
/// behind it. The quiet pocket's record and lease stand between the busy
/// pocket's in the lists, and `con-c` has never held a lease.
fn snapshot() -> Snapshot {
  Snapshot(
    ..inspection.empty(config.defaults()),
    next_position: 9,
    consumers: [Consumer("con-a"), Consumer("con-b"), Consumer("con-c")],
    pockets: [busy_pocket(), quiet_pocket()],
    records: [
      waiting(),
      unsettled(),
      elsewhere(),
      acked(),
      nacked(),
      acked_with_deferred(),
    ],
    leases: [expired_lease(), released_lease(), active_lease()],
  )
}

/// A state with something in every collection and no setting or counter at
/// its starting value, and the snapshot `inspection.snapshot` makes of it:
/// the configuration as plain settings, the counter, and each of the seven
/// lists member for member, in the state's order. The snapshot is written
/// out, not read back from the state.
fn snapshot_of_state() -> List(String) {
  let settings =
    Settings(
      lease_ttl: 1100,
      max_consecutive_expiries: 4,
      max_pull_batch: 7,
      superseded_ttl: 2200,
      dead_letter_ttl: 3300,
    )
  let assert Ok(config) = config.new(settings)
  let seen = snapshot()
  let publishers = [Publisher("pub-b"), Publisher("pub-a")]
  let superseded = [
    SupersededRecord(
      pocket: busy,
      key: "waiting",
      rank: 0,
      payload: <<"lost":utf8>>,
      reason: Superseded,
      shed_at: 300,
      expires_at: 2500,
    ),
    SupersededRecord(
      pocket: quiet,
      key: "waiting",
      rank: -1,
      payload: <<"late":utf8>>,
      reason: Stale,
      shed_at: 200,
      expires_at: 2400,
    ),
  ]
  let dead_letters = [
    DeadLetter(
      pocket: busy,
      key: "gone",
      rank: 1,
      payload: <<"gone":utf8>>,
      nacked_by: "con-a",
      nacked_at: 400,
      expires_at: 3800,
    ),
    DeadLetter(
      pocket: quiet,
      key: "gone",
      rank: 2,
      payload: <<"gone too":utf8>>,
      nacked_by: "con-b",
      nacked_at: 120,
      expires_at: 3450,
    ),
  ]
  let state =
    State(
      config:,
      queue: Queue(next_position: 9),
      publishers:,
      consumers: seen.consumers,
      pockets: seen.pockets,
      records: seen.records,
      leases: seen.leases,
      superseded:,
      dead_letters:,
    )
  assert inspection.snapshot(state)
    == Snapshot(
      config: settings,
      next_position: 9,
      publishers: [Publisher("pub-b"), Publisher("pub-a")],
      consumers: [Consumer("con-a"), Consumer("con-b"), Consumer("con-c")],
      pockets: [busy_pocket(), quiet_pocket()],
      records: [
        waiting(),
        unsettled(),
        elsewhere(),
        acked(),
        nacked(),
        acked_with_deferred(),
      ],
      leases: [expired_lease(), released_lease(), active_lease()],
      superseded:,
      dead_letters:,
    )
  []
}

fn nothing_to_view() -> List(String) {
  assert inspection.views(inspection.empty(config.defaults()))
    == Views(pockets: [], consumers: [])
  []
}

/// Every set is in the snapshot's order, and holds only the members whose
/// reference is the entity's key. `ending_records` is the held records that
/// are acked or nacked with nothing deferred: not the unsettled one, and
/// not the acked one a newer version waits behind.
fn derived_sets() -> List(String) {
  assert inspection.views(snapshot())
    == Views(
      pockets: [
        PocketView(
          pocket: busy_pocket(),
          records: [
            waiting(),
            unsettled(),
            acked(),
            nacked(),
            acked_with_deferred(),
          ],
          queued_records: [waiting()],
          held_records: [unsettled(), acked(), nacked(), acked_with_deferred()],
          ending_records: [acked(), nacked()],
          leases: [expired_lease(), active_lease()],
          active_leases: [active_lease()],
        ),
        PocketView(
          pocket: quiet_pocket(),
          records: [elsewhere()],
          queued_records: [elsewhere()],
          held_records: [],
          ending_records: [],
          leases: [released_lease()],
          active_leases: [],
        ),
      ],
      consumers: [
        ConsumerView(
          consumer: Consumer("con-a"),
          leases: [expired_lease(), active_lease()],
          active_leases: [active_lease()],
        ),
        ConsumerView(
          consumer: Consumer("con-b"),
          leases: [released_lease()],
          active_leases: [],
        ),
        ConsumerView(consumer: Consumer("con-c"), leases: [], active_leases: []),
      ],
    )
  []
}

/// A snapshot built by hand can hold one key twice. Each entity has its own
/// view, in the snapshot's order, and each view holds every member whose
/// reference is that key: the two views of a key differ only in the entity.
fn repeated_keys() -> List(String) {
  let twin =
    Pocket(
      key: busy,
      status: PocketQueued,
      position: 9,
      consecutive_expiries: 1,
    )
  let repeated =
    Snapshot(
      ..snapshot(),
      next_position: 10,
      consumers: [Consumer("con-a"), Consumer("con-a")],
      pockets: [busy_pocket(), twin],
    )
  let views = inspection.views(repeated)
  let assert [first, second] = views.pockets
  assert first.pocket == busy_pocket()
  assert second == PocketView(..first, pocket: twin)
  assert list.length(first.records) == 5
  assert first.active_leases == [active_lease()]
  let of_con_a =
    ConsumerView(
      consumer: Consumer("con-a"),
      leases: [expired_lease(), active_lease()],
      active_leases: [active_lease()],
    )
  assert views.consumers == [of_con_a, of_con_a]
  []
}

/// A record whose pocket key no pocket has, and an active lease whose
/// pocket and consumer are no entity's, belong to no view: the views are
/// those of the snapshot without them.
fn names_nothing() -> List(String) {
  let strays =
    Snapshot(
      ..snapshot(),
      records: list.append(snapshot().records, [queued("pocket-gone", "k", 8)]),
      leases: list.append(snapshot().leases, [
        lease("lease-4", "pocket-gone", "con-gone", LeaseActive),
      ]),
    )
  assert inspection.views(strays) == inspection.views(snapshot())
  []
}
