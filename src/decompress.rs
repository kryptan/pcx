use std::io;
use byteorder::{ReadBytesExt, WriteBytesExt};

/// Decompress variant of RLE (run-length-encoding) used in PCX files.
pub struct RleDecompressor<S : io::Read> {
	stream : S,

	run_count : u8,
	run_value : u8,
}

impl<S : io::Read> RleDecompressor<S> {
    /// Create new decompressor from the stream.
    pub fn new(stream : S) -> Self {
        RleDecompressor {
            stream : stream,
            run_count : 0,
            run_value : 0,
        }
    }

    /// Stop decompression process and get underlying stream.
    pub fn finish(self) -> S {
        self.stream
    }
}

impl<S : io::Read> io::Read for RleDecompressor<S> {
    fn read(&mut self, mut buffer: &mut [u8]) -> io::Result<usize> {
        use std::io::Write;

    	let buffer_len = buffer.len();

    	while buffer.len() > 0 {
			// Write the pixel run to the buffer.
			while self.run_count > 0 {
				if buffer.len() == 0 {
					return Ok(buffer_len);
				}

                buffer.write_u8(self.run_value);
			};

	    	let byte = self.stream.read_u8()?;
	    	if (byte & 0xC0) != 0xC0 { // 1-byte code
                buffer.write_u8(byte);
			} else { // 2-byte code
				self.run_count = byte & 0x3F;
		        self.run_value = self.stream.read_u8()?;
			}
	    }

        Ok(buffer_len)
    }
}
