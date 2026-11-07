//! The shed queue: superseded and stale payloads, kept visible until they decay
//! (`EveryDropIsVisible`), and the dead letters beside them. `Supersede` and
//! `DropStalePublish` feed the first, the two nack rules the second; the decay
//! (T26) drains both.

use std::collections::BTreeMap;

use super::types::{DeadLetter, SupersededRecord};
use crate::clock::Timestamp;

/// The superseded records keyed by expiry and then arrival, so a decay takes a
/// prefix and nothing else, and two shed at one instant keep both entries.
#[derive(Clone, Debug, Default)]
pub(super) struct Shed {
    superseded: BTreeMap<(Timestamp, u64), SupersededRecord>,
    /// The dead letters, keyed the same way and counted from the same arrivals.
    dead_letters: BTreeMap<(Timestamp, u64), DeadLetter>,
    arrivals: u64,
}

impl Shed {
    /// Keeps a superseded or stale payload until it decays.
    pub(super) fn keep(&mut self, record: SupersededRecord) {
        self.arrivals = self.arrivals.saturating_add(1);
        self.superseded
            .insert((record.expires_at, self.arrivals), record);
    }

    /// Keeps a nacked payload until it decays.
    pub(super) fn keep_dead_letter(&mut self, letter: DeadLetter) {
        self.arrivals = self.arrivals.saturating_add(1);
        self.dead_letters
            .insert((letter.expires_at, self.arrivals), letter);
    }

    /// Every dead letter, by expiry and arrival.
    #[cfg(feature = "testing")]
    pub(super) fn dead_letters(&self) -> Vec<DeadLetter> {
        self.dead_letters.values().cloned().collect()
    }

    /// Every superseded record, by expiry and arrival.
    #[cfg(feature = "testing")]
    pub(super) fn superseded(&self) -> Vec<SupersededRecord> {
        self.superseded.values().cloned().collect()
    }
}
