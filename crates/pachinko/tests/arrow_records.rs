//! The records contract over `ArrowRecords`: the same contract libpachinko runs
//! over its reference store, `MemoryRecords`.

#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use libpachinko::records::contract::check_contract;
use pachinko::arrow::{ArrowRecords, held_records};

#[test]
fn arrow_records_keep_the_records_contract() {
    check_contract::<ArrowRecords>(held_records);
}
