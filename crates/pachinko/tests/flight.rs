//! The Flight API over loopback: each test starts `serve` in-process on
//! `127.0.0.1:0` and drives it with `arrow_flight::FlightClient`.
//!
//! The expiry rules themselves are proved in libpachinko through `FakeClock`; the
//! lapse test here proves only that the ticker reaches `tick`, by polling `status`
//! under a deadline, and never sleeps to position itself at a lease's expiry.

// Every module is `#[cfg(test)]`, which is what clippy reads as test code: the
// fixture and the helpers may unwrap and panic as the tests themselves do. A test
// crate's root resolves its modules beside itself, so each names its file under
// `flight/`.
#[cfg(test)]
#[path = "flight/fixture.rs"]
mod fixture;
#[cfg(test)]
#[path = "flight/lease.rs"]
mod lease;
#[cfg(test)]
#[path = "flight/publish.rs"]
mod publish;
#[cfg(test)]
#[path = "flight/pull.rs"]
mod pull;
#[cfg(test)]
#[path = "flight/service.rs"]
mod service;
#[cfg(test)]
#[path = "flight/settle.rs"]
mod settle;
