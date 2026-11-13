//! What a test of the queue needs and no caller does, behind the `testing` feature.
//!
//! The feature is additive: it adds this module and the two [`Queue`](crate::Queue)
//! methods below, and changes nothing the crate root or a seam module holds.
//!
//! - The inspection seam: [`Queue::snapshot`](crate::Queue::snapshot) returns a
//!   [`Snapshot`], every entity of the queue as a value, enough to check every
//!   invariant without a second seam. It needs a store that implements
//!   [`InspectRecords`].
//! - The tick's cost: [`Queue::tick_with_cost`](crate::Queue::tick_with_cost)
//!   returns the tick's report beside a [`TickCost`], the entries the tick read.
//! - The records contract: [`check_contract`] is what every
//!   [`PocketRecords`](crate::records::PocketRecords) implementation must show.
//!
//! [`PocketStatus`], [`ShedReason`], [`SupersededRecord`] and [`DeadLetter`] are
//! exported here and nowhere else. No surface exposes them and the service does
//! not serve them: who reads the shed queue is the specification's second open
//! question.

pub use crate::queue::{
    DeadLetter, PocketStatus, PocketView, ShedReason, Snapshot, SupersededRecord, TickCost,
};
pub use crate::records::contract::{InspectRecords, check_contract};
