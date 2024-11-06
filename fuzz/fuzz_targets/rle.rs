#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate pcx;
use pcx::low_level::rle::tests::{round_trip, round_trip_one_by_one};

fuzz_target!(|data: &[u8]| {
    round_trip(data);
    round_trip_one_by_one(data);
});
