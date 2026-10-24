//! Smoke benchmark for the foundation, run by `just bench`.

fn main() {
    divan::main();
}

/// Time the library entry point, including an opaque indirect call's overhead.
#[divan::bench]
fn version() {
    let entry: fn() -> &'static str = divan::black_box(libpachinko::version);
    divan::black_box(entry());
}
