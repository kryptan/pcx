//! Library for reading and writing PCX images.
//!
//! Example of reading a PCX image:
//!
//!     let mut reader = pcx::Reader::from_file("test-data/marbles.pcx").unwrap();
//!     println!("width = {}, height = {}, paletted = {}", reader.width(), reader.height(), reader.is_paletted());
//!
//!     let mut buffer = vec![0; reader.width() as usize * reader.height() as usize * 3];
//!     reader.read_rgb_pixels(&mut buffer).unwrap();
//!
//! Example of writing a PCX image:
//!
//!     // Create 5x5 RGB file.
//!     let mut writer = pcx::WriterRgb::create_file("test.pcx", (5, 5), (300, 300)).unwrap();
//!     for y in 0..5 {
//!         // Write 5 green pixels.
//!         writer.write_row(&[0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0]);
//!     }
//!     writer.finish().unwrap();
//!
//! This library does not implement its own error type, instead it uses `std::io::Error`. In the case of an invalid
//! PCX file it will return an error with `.kind() == ErrorKind::InvalidData`.

// References:
// https://github.com/FFmpeg/FFmpeg/blob/415f907ce8dcca87c9e7cfdc954b92df399d3d80/libavcodec/pcx.c
// http://www.fileformat.info/format/pcx/egff.htm
// http://www.fileformat.info/format/pcx/spec/index.htm
use std::io;

pub use crate::reader::Reader;
pub use crate::writer::{WriterPaletted, WriterRgb};

pub mod low_level;
mod reader;
mod writer;

#[cfg(test)]
mod test_samples;

#[cfg(any(test, fuzzing))]
pub mod tests;

// Error caused by the incorrect usage of the API.
fn user_error<T>(error: &str) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidInput, error))
}
