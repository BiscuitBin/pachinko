//! The shed queue: superseded and stale payloads, kept visible until they decay
//! (`EveryDropIsVisible`). `Supersede` and `DropStalePublish` feed it; the decay
//! (T26) drains it.

use std::collections::BTreeMap;

use super::types::SupersededRecord;
use crate::clock::Timestamp;

/// The superseded records keyed by expiry and then arrival, so a decay takes a
/// prefix and nothing else, and two shed at one instant keep both entries.
#[derive(Clone, Debug, Default)]
pub(super) struct Shed {
    superseded: BTreeMap<(Timestamp, u64), SupersededRecord>,
    arrivals: u64,
}

impl Shed {
    /// Keeps a superseded or stale payload until it decays.
    pub(super) fn keep(&mut self, record: SupersededRecord) {
        self.arrivals = self.arrivals.saturating_add(1);
        self.superseded
            .insert((record.expires_at, self.arrivals), record);
    }

    /// Every superseded record, by expiry and arrival.
    #[cfg(feature = "testing")]
    pub(super) fn superseded(&self) -> Vec<SupersededRecord> {
        self.superseded.values().cloned().collect()
    }
}
