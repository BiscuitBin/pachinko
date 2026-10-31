//! The module's invariants, checked by name over the aggregate state of the order
//! machine and every pocket machine, as far as that state shows them.

/// One invariant that does not hold, by its name in the module. Agent (c) fills in the
/// check.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Violation {
    /// The invariant's name in `docs/specs/pachinko.allium`.
    pub invariant: &'static str,
    /// What was found.
    pub detail: String,
}
