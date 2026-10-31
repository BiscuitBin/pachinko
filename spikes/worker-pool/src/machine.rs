//! The machine: a value and a step function, with no clock, thread, lock or
//! randomness inside.

use crate::protocol::{Input, MachineId, Output};
use crate::time::Timestamp;

/// A single-threaded state machine. `step` takes one input and the current time as a
/// value, changes the state and returns the outputs in the order the pool is to
/// perform them. The shape of the LMAX business logic processor, and the shape
/// decision 0003 asks of library code: nothing inside reaches an effect.
pub trait Machine {
    /// Which machine this is.
    fn id(&self) -> MachineId;

    /// Processes one input at time `now`.
    fn step(&mut self, input: Input, now: Timestamp) -> Vec<Output>;

    /// How many inputs arrived that the protocol never sends this machine in that
    /// state. Zero in every correct run; a test asserts it.
    fn violations(&self) -> u64;
}
