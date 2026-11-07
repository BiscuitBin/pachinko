//! The records contract over `MemoryRecords`, the one `PocketRecords` store.
#![cfg(feature = "testing")]
#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

use libpachinko::records::MemoryRecords;
use libpachinko::records::contract::check_contract;

#[test]
fn memory_records_keep_the_records_contract() {
    check_contract::<MemoryRecords>();
}
