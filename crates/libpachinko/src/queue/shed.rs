//! The shed queue: superseded and stale payloads, kept visible until they decay
//! (`EveryDropIsVisible`), and the dead letters beside them. `Supersede` and
//! `DropStalePublish` feed the first, the two nack rules the second; a tick's
//! decay drains both (`SupersededRecordDecays`, `DeadLetterDecays`).

use std::collections::BTreeMap;

use super::types::{DeadLetter, SupersededRecord};
use crate::timestamp::Timestamp;

/// What [`Shed::decay`] removed.
#[derive(Debug, Default)]
pub(super) struct Decay {
    /// The superseded records that decayed.
    pub(super) superseded: usize,
    /// The dead letters that decayed.
    pub(super) dead_letters: usize,
    /// The entries the decay read: the due ones and, per non-empty queue, the
    /// first later one that stopped the walk.
    pub(super) examined: usize,
}

/// Removes the entries of `queue` whose expiry is at or before `now`, walking the
/// front of the queue and stopping at the first later entry. Returns how many it
/// removed and how many it read, the stopper included.
fn take_due<V>(queue: &mut BTreeMap<(Timestamp, u64), V>, now: Timestamp) -> (usize, usize) {
    let (mut removed, mut examined) = (0, 0);
    while let Some(entry) = queue.first_entry() {
        examined += 1;
        if entry.key().0 > now {
            break;
        }
        entry.remove();
        removed += 1;
    }
    (removed, examined)
}

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

    /// Removes every superseded record and dead letter whose `expires_at` is at or
    /// before `now`: the prefix of each queue, walked from the front, so the cost is
    /// the entries due plus one stopper per non-empty queue, never the whole backlog.
    pub(super) fn decay(&mut self, now: Timestamp) -> Decay {
        let (superseded, superseded_examined) = take_due(&mut self.superseded, now);
        let (dead_letters, dead_letters_examined) = take_due(&mut self.dead_letters, now);
        Decay {
            superseded,
            dead_letters,
            examined: superseded_examined + dead_letters_examined,
        }
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::take_due;
    use crate::timestamp::Timestamp;

    #[test]
    fn take_due_removes_the_prefix_and_reads_one_entry_past_it() {
        let at = Timestamp::from_millis;
        let mut queue: BTreeMap<(Timestamp, u64), u8> = [
            ((at(5), 1), 0),
            ((at(10), 2), 0),
            ((at(10), u64::MAX), 0),
            ((at(11), 0), 0),
        ]
        .into_iter()
        .collect();
        assert_eq!(take_due(&mut queue, at(4)), (0, 1));
        assert_eq!(take_due(&mut queue, at(10)), (3, 4));
        assert_eq!(queue.keys().copied().collect::<Vec<_>>(), [(at(11), 0)]);
        assert_eq!(take_due(&mut queue, at(11)), (1, 1));
        assert_eq!(take_due(&mut queue, at(12)), (0, 0));
    }
}
