//! The execution-model spike (ticket T20): bounded worker threads running
//! single-threaded state machines over partitions, for the domain
//! `docs/specs/pachinko.allium` specifies.
//!
//! A prototype, not the implementation: its findings are the ticket's hand-back
//! notes, and the implementation ticket starts from the module, never from this
//! code.
//!
//! The crate divides at the effects boundary decision 0003 draws. The machines
//! ([`OrderMachine`], [`PocketMachine`]), the protocol between them and the
//! single-threaded router [`Direct`] are plain values: no clock, thread, lock or
//! randomness inside, and `now` only as an argument of [`Machine::step`]. The pool
//! owns every effect.

pub mod direct;
pub mod invariants;
pub mod machine;
pub mod order;
pub mod partition;
pub mod pocket;
pub mod pool;
pub mod protocol;
pub mod state;
pub mod time;

pub use direct::Direct;
pub use invariants::Violation;
pub use machine::Machine;
pub use order::OrderMachine;
pub use partition::{fnv1a, partition_of, route};
pub use pocket::PocketMachine;
pub use protocol::{
    Became, ConsumerId, HandedRecord, Input, LeaseId, MachineId, Output, Partition, Payload,
    PocketKey, Published, PublisherId, Rank, Reason, RecordKey, Response,
};
pub use state::{
    DeadLetter, Lease, LeaseStatus, OrderStatus, OrderView, Pocket, PocketStatus, Record,
    RecordStatus, ShedReason, SupersededRecord,
};
pub use time::{Config, Duration, Timestamp};
