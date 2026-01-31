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

extern crate byteorder;
#[cfg(test)]
extern crate image;
#[cfg(test)]
extern crate walkdir;

use std::io;

pub use crate::reader::Reader;
pub use crate::writer::{WriterPaletted, WriterRgb};

pub mod low_level;
mod reader;
mod writer;

#[cfg(test)]
mod test_samples;

// Error caused by the incorrect usage of the API.
fn user_error<T>(error: &str) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidInput, error))
}

#[cfg(test)]
mod tests {
    use crate::{Reader, WriterPaletted, WriterRgb};
    use std::iter;

    fn round_trip_rgb_separate(width: u16, height: u16) {
        let mut pcx = Vec::new();

        {
            let mut writer = WriterRgb::new(&mut pcx, (width, height), (300, 300)).unwrap();

            let r: Vec<u8> = iter::repeat(88).take(width as usize).collect();
            let g: Vec<u8> = (0..width).map(|v| (v & 0xFF) as u8).collect();
            let mut b: Vec<u8> = iter::repeat(88).take(width as usize).collect();
            for y in 0..height {
                for x in 0..width {
                    b[x as usize] = (y & 0xFF) as u8;
                }

                writer.write_row_from_separate(&r, &g, &b).unwrap();
            }
            writer.finish().unwrap();
        }

        let mut reader = Reader::new(&pcx[..]).unwrap();
        assert_eq!(reader.dimensions(), (width, height));
        assert_eq!(reader.is_paletted(), false);
        assert_eq!(reader.palette_length(), None);

        let mut r: Vec<u8> = iter::repeat(0).take(width as usize).collect();
        let mut g: Vec<u8> = iter::repeat(0).take(width as usize).collect();
        let mut b: Vec<u8> = iter::repeat(0).take(width as usize).collect();

        for y in 0..height {
            reader
                .next_row_rgb_separate(&mut r, &mut g, &mut b)
                .unwrap();

            for x in 0..width {
                assert_eq!(r[x as usize], 88);
                assert_eq!(g[x as usize], (x & 0xFF) as u8);
                assert_eq!(b[x as usize], (y & 0xFF) as u8);
            }
        }
    }

    fn round_trip_rgb_interleaved(width: u16, height: u16) {
        let mut pcx = Vec::new();

        let written_rgb: Vec<u8> = (0..(width as usize) * 3)
            .map(|v| (v & 0xFF) as u8)
            .collect();
        {
            let mut writer = WriterRgb::new(&mut pcx, (width, height), (300, 300)).unwrap();

            for _ in 0..height {
                writer.write_row(&written_rgb).unwrap();
            }
            writer.finish().unwrap();
        }

        let mut reader = Reader::new(&pcx[..]).unwrap();
        assert_eq!(reader.dimensions(), (width, height));
        assert_eq!(reader.is_paletted(), false);
        assert_eq!(reader.palette_length(), None);

        let mut read_rgb: Vec<u8> = iter::repeat(0).take((width as usize) * 3).collect();

        for _ in 0..height {
            reader.next_row_rgb(&mut read_rgb).unwrap();
            assert_eq!(written_rgb, read_rgb);
        }
    }

    fn round_trip_paletted(width: u16, height: u16) {
        let mut pcx = Vec::new();

        let palette: Vec<u8> = (0..256 * 3).map(|v| (v % 0xFF) as u8).collect();
        {
            let mut writer = WriterPaletted::new(&mut pcx, (width, height), (300, 300)).unwrap();

            let mut p: Vec<u8> = iter::repeat(88).take(width as usize).collect();
            for y in 0..height {
                for x in 0..width {
                    p[x as usize] = (y & 0xFF) as u8;
                }

                writer.write_row(&p).unwrap();
            }

            writer.write_palette(&palette).unwrap();
        }

        let mut reader = Reader::new(&pcx[..]).unwrap();
        assert_eq!(reader.dimensions(), (width, height));
        assert!(reader.is_paletted());
        assert_eq!(reader.palette_length(), Some(256));

        let mut p: Vec<u8> = iter::repeat(0).take(width as usize).collect();

        for y in 0..height {
            reader.next_row_paletted(&mut p).unwrap();

            for x in 0..width {
                assert_eq!(p[x as usize], (y & 0xFF) as u8);
            }
        }

        let mut palette_read = [0; 3 * 256];
        reader.read_palette(&mut palette_read).unwrap();
        assert_eq!(&palette[..], &palette_read[..]);
    }

    #[test]
    fn small_round_trip() {
        for width in 1..40 {
            for height in 1..40 {
                round_trip_rgb_separate(width, height);
                round_trip_rgb_interleaved(width, height);
                round_trip_paletted(width, height);
            }
        }
    }

    #[test]
    fn large_round_trip_rgb() {
        round_trip_rgb_separate(0xFFFF - 1, 1);
        round_trip_rgb_separate(1, 0xFFFF);
        round_trip_rgb_interleaved(0xFFFF - 1, 1);
        round_trip_rgb_interleaved(1, 0xFFFF);
    }

    #[test]
    fn large_round_trip_paletted() {
        round_trip_paletted(0xFFFF - 1, 1);
        round_trip_paletted(1, 0xFFFF);
    }

    #[test]
    fn fuzzer_test_case() {
        let data: &[u8] = &[
            10, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 8, 255, 255, 255, 255, 255, 255, 255, 39, 3, 3, 3,
            3, 3, 189, 250, 189, 189, 189, 189, 173, 25, 189, 189, 189, 189, 189, 189, 189, 0, 0,
            3, 3, 3, 3, 3, 3, 3, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 3, 3, 3, 3, 10, 0, 0,
            1, 8, 255, 255, 255, 255, 255, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 253, 252, 252, 252,
            252, 252, 218, 252, 3, 3, 0, 0, 3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 253, 189, 253, 189, 189,
            189, 189, 189, 189, 152, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 10, 0, 0, 0, 3, 3, 3, 3,
            3, 189, 189, 3, 3,
        ];

        let mut pcx = Reader::from_mem(data).unwrap();
        let size = pcx.width() as usize * pcx.height() as usize * 3;
        let mut buffer = vec![0; size];
        _ = pcx.read_rgb_pixels(&mut buffer);
    }
}
