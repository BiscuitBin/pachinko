//! Smoke-test the fuzz harness until specifications supply real targets.
#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;

// A deliberately trivial byte consumer local to this harness. Invalid UTF-8 is
// ordinary input; no parser or public library API is introduced for a smoke test.
fn smoke(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        assert_eq!(text.as_bytes(), data);
    }
    assert_ne!(libpachinko::version(), "");
}

fuzz_target!(|data: &[u8]| smoke(data));
