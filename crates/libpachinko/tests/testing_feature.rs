//! The guard on the `testing` feature. The queue's suites and the records contract
//! are compiled only with it, so a test run without it would pass having run none
//! of them. This file is compiled only without it, and its one test fails that run.
#![cfg(not(feature = "testing"))]
#![expect(
    clippy::tests_outside_test_module,
    reason = "an integration test file is its own test module"
)]

#[test]
fn the_testing_feature_is_on() {
    panic!("libpachinko's suites need the `testing` feature: run `just test`");
}
