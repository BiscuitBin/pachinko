//// The actions a consumer takes under a lease it names. Of the
//// specification's "Renew and release" section this holds the Renew table.

import gleam/option.{None}
import pachinko_double/config
import pachinko_double/domain.{type Table}
import pachinko_double/model.{type State, Lease, LeaseActive, State}
import pachinko_double/refusals.{LeaseFenced}
import pachinko_double/rules.{Finish, Impossible, Now, Refuses, Row, Table}

/// The input of `Renew(consumer, lease)`. `consumer` is the consumer's id
/// and `lease` the lease's id.
pub type Renew {
  Renew(consumer: String, lease: String)
}

/// `Renew`. One rule has the trigger, so a renew its guard does not hold
/// for is rejected: the lease is not active, is not the consumer's, or was
/// never issued. The rule sets the lease's `expires_at` to `now` plus
/// `lease_ttl` and nothing else: a renew is not a new grant, and renewals do
/// not add up.
pub fn renew() -> Table(Renew) {
  Table(
    "Renew",
    [
      Row(
        "Renew",
        fn(state: State, renew: Renew) {
          case model.lease(state.leases, renew.lease) {
            Error(Nil) -> False
            Ok(lease) ->
              lease.status == LeaseActive && lease.consumer == renew.consumer
          }
        },
        fn(state, renew) {
          case model.lease(state.leases, renew.lease) {
            Error(Nil) -> Impossible("no lease " <> renew.lease)
            Ok(lease) -> {
              use now <- Now
              let renewed =
                Lease(
                  ..lease,
                  expires_at: now + config.settings(state.config).lease_ttl,
                )
              Finish(
                State(
                  ..state,
                  leases: model.replace_lease(state.leases, renewed),
                ),
                [],
                None,
              )
            }
          }
        },
      ),
    ],
    Refuses(fn(_state, renew: Renew) {
      LeaseFenced(
        action: "Renew",
        consumer_id: renew.consumer,
        lease_id: renew.lease,
      )
    }),
  )
}
