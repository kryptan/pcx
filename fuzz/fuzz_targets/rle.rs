#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate pcx;
use std::io::{Read, Write};
use pcx::low_level::rle::{Compressor, Decompressor};

fuzz_target!(|data: &[u8]| {
    round_trip(data);
    round_trip_one_by_one(data);
});

fn round_trip(data: &[u8]) {
    let mut compressed = Vec::new();

    {
        let mut compressor = Compressor::new(&mut compressed, 8);
        compressor.write_all(&data).unwrap();
        compressor.flush().unwrap();
    }

    let mut decompressor = Decompressor::new(&compressed[..]);

    let mut result = Vec::new();
    assert_eq!(decompressor.read_to_end(&mut result).unwrap(), data.len());
    assert_eq!(result, data);
}

fn round_trip_one_by_one(data: &[u8]) {
    let mut compressed = Vec::new();

    {
        let mut compressor = Compressor::new(&mut compressed, 16);
        for &d in data {
            compressor.write_all(&[d]).unwrap();
        }
        compressor.flush().unwrap();
    }

    let mut decompressor = Decompressor::new(&compressed[..]);

    let mut result = Vec::new();
    for _ in 0..data.len() {
        let mut byte = [0; 1];
        decompressor.read_exact(&mut byte).unwrap();
        result.push(byte[0]);
    }
    assert_eq!(result, data);
}