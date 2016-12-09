use std::io;
use byteorder::{ReadBytesExt, WriteBytesExt};

/// Decompress variant of RLE (run-length-encoding) used in PCX files.
///
/// If you are reading PCX file via `Reader` you don't need to use `RleDecompressor`.
pub struct Decompressor<S : io::Read> {
	stream : S,

	run_count : u8,
	run_value : u8,
}

impl<S : io::Read> Decompressor<S> {
    /// Create new decompressor from the stream.
    pub fn new(stream : S) -> Self {
        Decompressor {
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

impl<S : io::Read> io::Read for Decompressor<S> {
    fn read(&mut self, mut buffer: &mut [u8]) -> io::Result<usize> {
        use std::io::Write;

        let mut written = 0;
    	while buffer.len() > 0 {
			// Write the pixel run to the buffer.
			while self.run_count > 0 {
				if buffer.len() == 0 {
					return Ok(written);
				}

                buffer.write_u8(self.run_value);
                written += 1;
			};

	    	let mut byte_buffer = [0; 1];
            if self.stream.read(&mut byte_buffer)? == 0 {
                return Ok(written);
            }

            let byte = byte_buffer[0];

	    	if (byte & 0xC0) != 0xC0 { // 1-byte code
                buffer.write_u8(byte);
                written += 1;
			} else { // 2-byte code
				self.run_count = byte & 0x3F;
		        self.run_value = self.stream.read_u8()?;
			}
	    }

        Ok(written)
    }
}
