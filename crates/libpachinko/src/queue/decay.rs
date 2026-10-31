//! Decay: `SupersededRecordDecays` and `DeadLetterDecays`, which the clock triggers.

use super::outcome::Decayed;
use super::state::State;
use crate::Timestamp;

impl State {
    /// Removes every superseded record and dead letter whose `expires_at` is at or
    /// before `now`.
    pub(super) fn decay(&mut self, now: Timestamp) -> Decayed {
        let (superseded, dead_letters) = (self.superseded.len(), self.dead_letters.len());
        self.superseded.retain(|shed| shed.expires_at > now);
        self.dead_letters.retain(|letter| letter.expires_at > now);
        Decayed {
            superseded: superseded - self.superseded.len(),
            dead_letters: dead_letters - self.dead_letters.len(),
        }
    }
}
