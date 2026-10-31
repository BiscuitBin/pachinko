//! The superseded records and dead letters, kept until they decay
//! (`SupersededRecordDecays`, `DeadLetterDecays`).

use std::collections::BTreeMap;

use super::{DeadLetter, SupersededRecord};
use crate::clock::Timestamp;

/// When something is shed and when it decays.
#[derive(Clone, Copy, Debug)]
pub(super) struct Stamp {
    pub(super) at: Timestamp,
    pub(super) expires_at: Timestamp,
}

/// Both decaying queues, each keyed by expiry and then arrival, so a decay takes a
/// prefix and nothing else.
#[derive(Clone, Debug, Default)]
pub(super) struct Shed {
    superseded: BTreeMap<(Timestamp, u64), SupersededRecord>,
    dead_letters: BTreeMap<(Timestamp, u64), DeadLetter>,
    arrivals: u64,
}

impl Shed {
    /// Keeps a superseded or stale payload until it decays.
    pub(super) fn keep_superseded(&mut self, record: SupersededRecord) {
        self.arrivals += 1;
        self.superseded
            .insert((record.expires_at, self.arrivals), record);
    }

    /// Keeps a nacked payload until it decays.
    pub(super) fn keep_dead_letter(&mut self, letter: DeadLetter) {
        self.arrivals += 1;
        self.dead_letters
            .insert((letter.expires_at, self.arrivals), letter);
    }

    /// Removes every entry whose expiry is at or before `now`; returns how many
    /// superseded records and dead letters went. An arrival number never reaches
    /// `u64::MAX`, so the split point sorts after every entry stamped `now`.
    pub(super) fn decay(&mut self, now: Timestamp) -> (usize, usize) {
        let after = (now, u64::MAX);
        let superseded = self.superseded.split_off(&after);
        let dead_letters = self.dead_letters.split_off(&after);
        let decayed = (self.superseded.len(), self.dead_letters.len());
        self.superseded = superseded;
        self.dead_letters = dead_letters;
        decayed
    }

    /// Every superseded record, by expiry and arrival.
    #[cfg(feature = "testing")]
    pub(super) fn superseded(&self) -> Vec<SupersededRecord> {
        self.superseded.values().cloned().collect()
    }

    /// Every dead letter, by expiry and arrival.
    #[cfg(feature = "testing")]
    pub(super) fn dead_letters(&self) -> Vec<DeadLetter> {
        self.dead_letters.values().cloned().collect()
    }
}
