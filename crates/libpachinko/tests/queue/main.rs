//! The queue's tests from `docs/specs/pachinko.allium`, one module per rule family,
//! over `MemoryRecords`, a `FakeClock` and `CountingLeaseIds`.
//!
//! Tag conventions. Each test names what it pins in comments above it:
//!
//! - `// scenario:` and the id, such as S011, for a scenario of the wave's
//!   catalog, once per scenario across the crate;
//! - `// obligation: <id>` for each obligation id `just plan-spec pachinko` prints
//!   that the test covers, such as `rule-success.Supersede` or
//!   `invariant.OnePocketPerKey`;
//! - `// obligation: guarantee.<Name>` for a surface `@guarantee`, which the checker
//!   gives no id of its own.
//!
//! An invariant test checks its invariant by name, through the fixture's
//! `invariants` functions, over the `Snapshot` after the operations that could
//! break it. Temporal tests move the fake clock and never read the wall clock.

#![cfg(feature = "testing")]

// Every module is `#[cfg(test)]`, which is what clippy reads as test code: the
// fixture and the helpers may unwrap and panic as the tests themselves do.
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod identity;
#[cfg(test)]
mod publish;
#[cfg(test)]
mod pull;
