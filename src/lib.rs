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

pub use reader::Reader;
pub use writer::{WriterRgb, WriterPaletted};

pub mod low_level;
mod reader;
mod writer;

#[cfg(test)]
mod tests {
   // use ::Header;
  //  use std::fs::File;
  //  use std::io;
    use std::iter;
    use {Reader, WriterRgb, WriterPaletted};

    fn roundtrip_rgb(width : u16, height : u16) {
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

        println!("{:?}", pcx);

        let mut reader = Reader::new(&pcx[..]).unwrap();
        assert_eq!(reader.size(), (width, height));
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

    #[test]
    fn small_roundtrip() {
        for width in 1..40 {
            for height in 1..40 {
                roundtrip_rgb(width, height);
            }
        }
    }

    #[test]
    fn large_roundtrip() {
        roundtrip_rgb(0xFFFF - 1, 1);
        roundtrip_rgb(1, 0xFFFF);
    }
}
