//! Implementation of compression/decompression using variant of RLE (run-length-encoding) used in PCX files.

use std::io;
use byteorder::{ReadBytesExt, WriteBytesExt};

/// Decompress RLE.
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
        let mut read = 0;
        while buffer.len() > 0 {
            // Write the pixel run to the buffer.
            while self.run_count > 0 && buffer.len() > 0 {
                buffer.write_u8(self.run_value)?;
                self.run_count -= 1;
                read += 1;
            };

            if buffer.len() == 0 {
                return Ok(read);
            }

            let byte = {
                let mut byte_buffer = [0; 1];
                if self.stream.read(&mut byte_buffer)? == 0 {
                    return Ok(read);
                }
                byte_buffer[0]
            };

            if (byte & 0xC0) != 0xC0 { // 1-byte code
                buffer.write_u8(byte)?;
                read += 1;
            } else { // 2-byte code
                self.run_count = byte & 0x3F;
                self.run_value = self.stream.read_u8()?;
            }
        }

        Ok(read)
    }
}

/// Compress using RLE.
///
/// Warning: compressor does not implement `Drop` and will not automatically get flushed on destruction. Call `finish` or `flush` to flush it.
/// If it would implement `Drop` it would be impossible to implement `finish()` due to
/// [restrictions](https://doc.rust-lang.org/error-index.html#E0509) of the Rust language.
pub struct Compressor<S : io::Write> {
    stream : S,

    lane_length : u16,
    lane_position : u16,

    run_count : u8,
    run_value : u8,
}

impl<S : io::Write> Compressor<S> {
    /// Create new compressor which will write to the stream.
    pub fn new(stream : S, lane_length : u16) -> Self {
        Compressor {
            stream : stream,
            run_count : 0,
            run_value : 0,
            lane_length : lane_length,
            lane_position : 0,
        }
    }

    pub fn pad(&mut self) -> io::Result<()> {
        use std::io::Write;

        while self.lane_position != 0 {
            self.write(&[0])?;
        }

        Ok(())
    }

    /// Stop compression process and get underlying stream.
    pub fn finish(mut self) -> io::Result<S> {
        self.flush_compressor()?;
        Ok(self.stream)
    }

    fn flush_compressor(&mut self) -> io::Result<()> {
        match (self.run_count, self.run_value) {
            (0, _) => {},
            (1, run_value @ 0 ... 0xBF) => {
                self.stream.write_u8(run_value)?;
            },
            (run_count, run_value) => {
                self.stream.write_u8(0xC0 | run_count)?;
                self.stream.write_u8(run_value)?;
            }
        }

        self.stream.flush()
    }
}

impl<S : io::Write> io::Write for Compressor<S> {
    fn write(&mut self, mut buffer: &[u8]) -> io::Result<usize> {
        use std::io::Read;

        let mut written = 0;

        while buffer.len() > 0 {
            let byte = {
                let mut byte_buffer = [0; 1];
                if buffer.read(&mut byte_buffer)? == 0 {
                    return Ok(written);
                }
                byte_buffer[0]
            };

            self.lane_position += 1;
            written += 1;

            if byte == self.run_value && self.run_count < 62 && self.lane_position != self.lane_length {
                self.run_count += 1;
                continue;
            }

            if self.lane_position ==  self.lane_length {
                self.lane_position = 0;
            }

            self.flush_compressor()?;

            self.run_count = 1;
            self.run_value = byte;
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_compressor()?;
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use byteorder::{ReadBytesExt, WriteBytesExt};
    use super::{Compressor, Decompressor};

    fn round_trip(data : &[u8]) {
        use std::io::{Read, Write};

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

    fn round_trip_one_by_one(data : &[u8]) {
        use std::io::{Write};

        let mut compressed = Vec::new();

        {
            let mut compressor = Compressor::new(&mut compressed, 16);
            for &d in data {
                compressor.write_u8(d).unwrap();
            }
            compressor.flush().unwrap();
        }

        let mut decompressor = Decompressor::new(&compressed[..]);

        let mut result = Vec::new();
        for _ in 0..data.len() {
            result.push(decompressor.read_u8().unwrap());
        }
        assert_eq!(result, data);
    }

    #[test]
    fn round_trip_1() {
        let data = [0, 1, 2, 3, 5, 5, 5, 128, 128, 128, 7, 7, 255, 7, 255, 255, 254, 0, 0, 0, 4, 4, 177, 177, 4, 177, 177];
        round_trip_one_by_one(&data);
        round_trip(&data);
    }

    #[test]
    fn round_trip_2() {
        let data = [
            0, 1, 2, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        ];

        round_trip(&data);
        round_trip_one_by_one(&data);
    }
}