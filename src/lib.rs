//! Library for reading PCX image format.
//!

// References:
// https://github.com/FFmpeg/FFmpeg/blob/415f907ce8dcca87c9e7cfdc954b92df399d3d80/libavcodec/pcx.c
// http://www.fileformat.info/format/pcx/egff.htm
// http://www.fileformat.info/format/pcx/spec/index.htm
extern crate byteorder;

pub use header::{Version, Header};
pub use reader::Reader;

pub mod rle;
mod header;
mod reader;
mod writer;

#[cfg(test)]
mod tests {
    use ::Header;
    use std::fs::File;
    use std::io;

    #[test]
    fn test() {

        let file = File::open("brick.pcx").unwrap();
        Header::load(&mut io::BufReader::new(file)).unwrap();
    }
}