use std::io;

use rle::Compressor;

/// PCX file writer.
pub struct WriterRGB<R: io::Write> {
    compressor : Compressor<R>,
    num_lanes_read : u32,
}