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

pub use header::{Version, Header};
pub use reader::Reader;
pub use writer::{WriterRgb, WriterPaletted};

pub mod rle;
mod header;
mod reader;
mod writer;

const MAGIC_BYTE : u8 = 0xA;
const PALETTE_START : u8 = 0xC;

#[cfg(test)]
mod tests {
   // use ::Header;
  //  use std::fs::File;
  //  use std::io;

    #[test]
    fn test() {

      /*  let file = File::open("brick.pcx").unwrap();
        Header::load(&mut io::BufReader::new(file)).unwrap();*/
    }
}