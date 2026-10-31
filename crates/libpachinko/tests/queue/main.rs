//! The tests `propagate` derives from `docs/specs/pachinko.allium`, one module per
//! rule family, over `MemoryRecords` and `FakeClock`.
//!
//! Each test names the obligations it covers, by the ids `just plan-spec pachinko`
//! prints, in an `// obligation:` comment. Temporal tests move the fake clock and
//! never read the wall clock.

// Every module is `#[cfg(test)]`, which is what clippy reads as test code: the
// fixture and the helpers may unwrap and panic as the tests themselves do.
#[cfg(test)]
mod decay;
#[cfg(test)]
mod expiry;
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod identity;
#[cfg(test)]
mod invariants;
#[cfg(test)]
mod lease;
#[cfg(test)]
mod publish;
#[cfg(test)]
mod pull;
#[cfg(test)]
mod settle;
#[cfg(test)]
mod surfaces;
