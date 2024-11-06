#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate pcx;
use pcx::low_level::Header;

fuzz_target!(|data: &[u8]| {
    let mut data = data;

    // Check that it loads without a panic.
    let _ = Header::load(&mut data);
});
