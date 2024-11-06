#![no_main]
use libfuzzer_sys::fuzz_target;
use pcx::low_level::rle::tests::{round_trip, round_trip_one_by_one};

fuzz_target!(|data: &[u8]| {
    round_trip(data);
    round_trip_one_by_one(data);
});
