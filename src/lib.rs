//! Library for reading and writing PCX image format.
//!
//! PCX is quite old format, it is not recommended to use it for new applications.
//!
//! PCX does not contain any color space information. Today one will usually interpret it as containing colors in [sRGB](https://en.wikipedia.org/wiki/sRGB) color space.

// References:
// https://github.com/FFmpeg/FFmpeg/blob/415f907ce8dcca87c9e7cfdc954b92df399d3d80/libavcodec/pcx.c
// http://www.fileformat.info/format/pcx/egff.htm
// http://www.fileformat.info/format/pcx/spec/index.htm

extern crate byteorder;
#[cfg(test)]
extern crate walkdir;
#[cfg(test)]
extern crate image;

pub use reader::Reader;
pub use writer::{WriterRgb, WriterPaletted};

pub mod low_level;
mod reader;
mod writer;

#[cfg(test)]
mod test_samples;

#[cfg(test)]
mod tests {
    use std::iter;
    use {Reader, WriterRgb, WriterPaletted};

    fn round_trip_rgb(width: u16, height: u16) {
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

                writer.write_row(&r, &g, &b).unwrap();
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
            reader.next_row_rgb(&mut r, &mut g, &mut b).unwrap();

            for x in 0..width {
                assert_eq!(r[x as usize], 88);
                assert_eq!(g[x as usize], (x & 0xFF) as u8);
                assert_eq!(b[x as usize], (y & 0xFF) as u8);
            }
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
                round_trip_rgb(width, height);
                round_trip_paletted(width, height);
            }
        }
    }

    #[test]
    fn large_round_trip_rgb() {
        round_trip_rgb(0xFFFF - 1, 1);
        round_trip_rgb(1, 0xFFFF);
    }

    #[test]
    fn large_round_trip_paletted() {
        round_trip_paletted(0xFFFF - 1, 1);
        round_trip_paletted(1, 0xFFFF);
    }
}
