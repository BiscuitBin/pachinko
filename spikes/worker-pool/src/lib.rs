//! The execution-model spike (ticket T20): bounded worker threads running
//! single-threaded state machines over partitions, for the domain
//! `docs/specs/pachinko.allium` specifies.
//!
//! A prototype, not the implementation: its findings are the ticket's hand-back
//! notes, and the implementation ticket starts from the module, never from this
//! code.
