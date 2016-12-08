extern crate byteorder;

use std::io;

pub mod decompress;

mod header;
mod error;

pub use header::{Version, Header};
pub use error::DecodingError;

use decompress::RleDecompressor;

/// PCX file reader.
pub struct Reader<R: io::Read> {
    /// File header. All useful values are available via `Reader` methods so you don't actually need it.
    pub header : Header,

    decompressor : RleDecompressor<R>,
}

impl<R: io::Read> Reader<R> {
    /// Start reading PCX file.
    pub fn new(mut stream: R) -> Result<Self, DecodingError> {
        let header = Header::load(&mut stream)?;

        Ok(Reader {
            header : header,
            decompressor : RleDecompressor::new(stream),
        })
    }

    /// Get width and height of the image.
    pub fn size(&self) -> (u16, u16) {
        self.header.size
    }

    /// Whether this image is paletted or 24-bit RGB.
    pub fn is_paletted(&self) -> bool {
        self.header.palette_length().is_some()
    }

    /// Get number of colors in the palette if this image is paletted. Number of colors is either 2, 4, 8, 16 or 256.
    pub fn palette_length(&self) -> Option<u16> {
        self.header.palette_length()
    }

    /// Read next row of the image. If image is paletted then buffer size must be equal to image width and each byte in the output buffer will contain index
    /// into image palette. If
    ///
    /// Order of rows is from top to bottom.
   /*pub fn next_row(&mut self, buffer: &mut [u8]) -> io::Result<()> {

    }*/

    /// This is a low-level function and it is not recommended to call it directly. Use `next_row()` instead.
    ///
    /// Read next lane. Format is dependent on file format. Buffer length must be equal to `Header::lane_proper_length()`, otherwise this method will panic.
    ///
    /// Order of lanes is from top to bottom.
    pub fn next_lane(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        use std::io::Read;

        if buffer.len() != self.header.lane_proper_length() as usize {
            panic!("pcx::Reader::next_lane: incorrect buffer size.");
        }

        self.decompressor.read(buffer)?;

        Ok(())
    }

    /// Read color palette.
    ///
    /// If palette contains 256-colors then it is stored at the end of file and this function will read the file to the end.
    ///
    /// Returns number of colors in palette or zero if there is no palette. The actual number of bytes written to the output buffer is
    /// equal to the returned value multiplied by 3. Format of the output buffer is R, G, B, R, G, B, ...
    pub fn read_palette(mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.header.palette_length() {
            Some(palette_length @ 1 ... 16) => {
                // Palettes of 16 colors or smaller are stored in the header.
                for i in 0..(palette_length as usize) {
                    (&mut buffer[(i*3)..((i + 1)*3)]).copy_from_slice(&self.header.palette[i]);
                }
                return Ok(palette_length as usize)
            },
            Some(256) => {
                // 256-color palette is located at the end of file, we will read it below.
            },
            _ => return Ok(0),
        }

        // Stop decompressing and continue reading underlying stream.
        let mut stream = self.decompressor.finish();

        // 256-color palette is located at the end of file. To avoid seeking we are using a bit convoluted method here to read it.
        const PALETTE_LENGTH: usize = 256*3;
        const TEMP_BUFFER_LENGTH: usize = PALETTE_LENGTH + 1;

        let mut temp_buffer = [0; TEMP_BUFFER_LENGTH];
        let mut pos = 0;

        loop {
            let read = stream.read(&mut temp_buffer[pos..(TEMP_BUFFER_LENGTH - pos)])?;
            if read != 0 {
                pos = (pos + read) % TEMP_BUFFER_LENGTH;
            } else {
                // We've reached the end of file, therefore temp_buffer must now contain the palette.
                if temp_buffer[pos] != 0xC {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "no 256-color palette"));
                }

                &mut buffer[0..(TEMP_BUFFER_LENGTH - pos - 1)].copy_from_slice(&temp_buffer[(pos + 1)..TEMP_BUFFER_LENGTH]);
                &mut buffer[(TEMP_BUFFER_LENGTH - pos - 1)..PALETTE_LENGTH].copy_from_slice(&temp_buffer[0..pos]);
            }
        }

        Ok(256)
    }
}

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