//! A single-threaded router over the machines: steps them in place and delivers every
//! message in FIFO order per destination, with no thread and no clock. The replay and
//! invariant tests drive it, and it is the logic reference the pool's one-thread
//! configuration is compared with.

use crate::order::OrderMachine;
use crate::pocket::PocketMachine;

/// The direct router. Agent (c) fills this in.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Direct {
    order: OrderMachine,
    pockets: Vec<PocketMachine>,
}

impl Direct {
    /// The order machine.
    #[must_use]
    pub const fn order(&self) -> &OrderMachine {
        &self.order
    }

    /// The pocket machines, by partition index.
    #[must_use]
    pub fn pockets(&self) -> &[PocketMachine] {
        &self.pockets
    }
}
