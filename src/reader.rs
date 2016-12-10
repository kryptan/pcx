use std::io;
use std::path::Path;
use std::fs::File;
use byteorder::ReadBytesExt;

use low_level::{Header, PALETTE_START};
use low_level::rle::Decompressor;

/// PCX file reader.
pub struct Reader<R: io::Read> {
    /// File header. All useful values are available via `Reader` methods so you don't actually need it.
    pub header : Header,

    decompressor : Decompressor<R>,
    num_lanes_read : u32,
}

impl Reader<io::BufReader<File>> {
    /// Start reading PCX file.
    pub fn new_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Self::new(io::BufReader::new(file))
    }
}

impl<R: io::Read> Reader<R> {
    /// Start reading PCX file.
    pub fn new(mut stream: R) -> io::Result<Self> {
        let header = Header::load(&mut stream)?;

        Ok(Reader {
            header : header,
            decompressor : Decompressor::new(stream),
            num_lanes_read : 0,
        })
    }

    /// Get width and height of the image.
    pub fn dimensions(&self) -> (u16, u16) {
        self.header.size
    }

    /// The width of this image.
    pub fn width(&self) -> u16 {
        self.header.size.0
    }

    /// The width of this image.
    pub fn height(&self) -> u16 {
        self.header.size.1
    }

    /// Whether this image is paletted or 24-bit RGB.
    pub fn is_paletted(&self) -> bool {
        self.header.palette_length().is_some()
    }

    /// Get number of colors in the palette if this image is paletted. Number of colors is either 2, 4, 8, 16 or 256.
    pub fn palette_length(&self) -> Option<u16> {
        self.header.palette_length()
    }

    /// Read next row of the paletted image.  Check that `is_paletted()` is `true` before calling this function.
    ///
    /// `buffer` length must be equal to the image width.
    ///
    /// Order of rows is from top to bottom.
    pub fn next_row_paletted(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        if !self.is_paletted() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "pcx::Reader::next_row_paletted called on non-paletted image"))
        }

        if self.palette_length() == Some(256) {
            self.next_lane(buffer)?;
        } else if self.header.number_of_color_planes == 1 { // All packed formats, max. 16 colors.
            let lane_length = self.header.lane_proper_length() as usize;
            let buffer_len = buffer.len();
            let offset = buffer.len() - lane_length;

            // Place packed row at the end of buffer, this will allow us easily to unpack it.
            self.next_lane(&mut buffer[offset..buffer_len])?;

            macro_rules! unpack_bits {
                ($bits:expr) => {
                    for i in 0..lane_length {
                        for j in 0..(8/$bits) {
                            buffer[i*(8/$bits) + j] = (buffer[offset + i] & (((1 << $bits) - 1) << (8 - $bits*(j + 1)))) >> (8 - $bits*(j + 1));
                        }
                    }
                }
            };

            // Unpack packed bits into bytes.
            match self.header.bit_depth {
                1 => unpack_bits!(1),
                2 => unpack_bits!(2),
                4 => unpack_bits!(4),
                _ => unreachable!(), // bit depth was checked while reading header
            }
        } else { // Planar, 4, 8 or 16 colors.
            let lane_length = self.header.lane_proper_length() as usize;
            let number_of_color_planes = self.header.number_of_color_planes as usize;
            let half_len = buffer.len()/2;

            // Place packed rows at the first half of the buffer, this will allow us easily to unpack them.
            for i in 0..number_of_color_planes {
                self.next_lane(&mut buffer[(lane_length*i)..(lane_length*(i + 1))])?;
            }

            for x in 0..self.width() {
                let m = 0x80 >> (x & 7);
                let mut v = 0;
                for i in (0..number_of_color_planes).rev() {
                    v <<= 1;
                    v  += if buffer[i*lane_length + (x as usize >> 3)] & m != 0 { 1 } else { 0 };
                }
                if x % 2 == 0 {
                    buffer[half_len + (x/2) as usize] = v << 4;
                } else {
                    buffer[half_len + (x/2) as usize] |= v;
                }
            }

            for i in 0..half_len {
                buffer[i*2] = (buffer[half_len + i] & 0xF0) >> 4;
                buffer[i*2 + 1] = buffer[half_len + i] & 0xF;
            }
        }

        Ok(())
    }

    /// Read next row of the RGB image. Check that `is_paletted()` is `false` before calling this function.
    ///
    /// `r`, `g`, `b` buffer lengths must be equal to the image width.
    ///
    /// Order of rows is from top to bottom.
    pub fn next_row_rgb(&mut self, r: &mut [u8], g: &mut [u8], b: &mut [u8]) -> io::Result<()> {
        if self.is_paletted() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "pcx::Reader::next_row_rgb called on paletted image"));
        }

        if self.num_lanes_read % 3 != 0{
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "pcx::Reader::next_row_rgb, invalid use of next_lane"));
        }

        self.next_lane(r)?;
        self.next_lane(g)?;
        self.next_lane(b)
    }

    // Read next lane. Format is dependent on file format. Buffer length must be equal to `Header::lane_proper_length()`.
    //
    // Order of lanes is from top to bottom.
    fn next_lane(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        use std::io::Read;

        if buffer.len() != self.header.lane_proper_length() as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "pcx::Reader::next_lane: incorrect buffer size."));
        }

        self.decompressor.read_exact(buffer)?;

        if self.num_lanes_read + 1 < (self.height() as u32)*(self.header.number_of_color_planes as u32) {
            // Skip padding.
            for _ in 0..self.header.lane_padding() {
                self.decompressor.read_u8()?;
            }
        }

        self.num_lanes_read += 1;

        Ok(())
    }

    /// Read color palette.
    ///
    /// If palette contains 256-colors then it is stored at the end of file and this function will read the file to the end.
    ///
    /// Returns number of colors in palette or zero if there is no palette. The actual number of bytes written to the output buffer is
    /// equal to the returned value multiplied by 3. Format of the output buffer is R, G, B, R, G, B, ...
    pub fn read_palette(self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.header.palette_length() {
            Some(2) => {
                // Special case - monochrome image.

                // Black.
                buffer[0] = 0;
                buffer[1] = 0;
                buffer[2] = 0;

                // White.
                buffer[3] = 255;
                buffer[4] = 255;
                buffer[5] = 255;

                return Ok(2 as usize);
            },
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
            let read = stream.read(&mut temp_buffer[pos..TEMP_BUFFER_LENGTH])?;
            if read != 0 {
                pos = (pos + read) % TEMP_BUFFER_LENGTH;
            } else {
                // We've reached the end of file, therefore temp_buffer must now contain the palette.
                if temp_buffer[pos] != PALETTE_START {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "no 256-color palette"));
                }

                &mut buffer[0..(TEMP_BUFFER_LENGTH - pos - 1)].copy_from_slice(&temp_buffer[(pos + 1)..TEMP_BUFFER_LENGTH]);
                &mut buffer[(TEMP_BUFFER_LENGTH - pos - 1)..PALETTE_LENGTH].copy_from_slice(&temp_buffer[0..pos]);

                return Ok(256);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::{Reader};
    use low_level::header;

    #[test]
    fn gmarbles() {
        let data = include_bytes!("../test-data/gmarbles.pcx");
        let read = &mut &data[..];
        let mut reader = Reader::new(read).unwrap();

        assert_eq!(reader.header.version, header::Version::V5);
        assert_eq!(reader.header.is_compressed, true);
        assert_eq!(reader.header.bit_depth, 8);
        assert_eq!(reader.header.size, (141, 99));
        assert_eq!(reader.header.start, (0, 0));
        assert_eq!(reader.header.dpi, (300, 300));
        assert_eq!(reader.header.number_of_color_planes, 1);
        assert_eq!(reader.header.lane_length, 142);

        assert!(reader.is_paletted());
        assert_eq!(reader.palette_length(), Some(256));

        let mut row : Vec<u8> = iter::repeat(0).take(reader.width() as usize).collect();
        for _ in 0..reader.height() {
            reader.next_row_paletted(&mut row[..]).unwrap();
        }

        let mut palette = [0; 256*3];
        assert_eq!(reader.read_palette(&mut palette).unwrap(), 256);
    }

    #[test]
    fn marbles() {
        let data = include_bytes!("../test-data/marbles.pcx");
        let read = &mut &data[..];
        let mut reader = Reader::new(read).unwrap();

        assert_eq!(reader.header.version, header::Version::V5);
        assert!(reader.header.is_compressed);
        assert_eq!(reader.header.bit_depth, 8);
        assert_eq!(reader.header.size, (143, 101));
        assert_eq!(reader.header.start, (0, 0));
        assert_eq!(reader.header.dpi, (300, 300));
        assert_eq!(reader.header.number_of_color_planes, 3);
        assert_eq!(reader.header.lane_length, 144);

        assert_eq!(reader.is_paletted(), false);

        let mut r : Vec<u8> = iter::repeat(0).take(reader.width() as usize).collect();
        let mut g : Vec<u8> = iter::repeat(0).take(reader.width() as usize).collect();
        let mut b : Vec<u8> = iter::repeat(0).take(reader.width() as usize).collect();
        for _ in 0..reader.height() {
            reader.next_row_rgb(&mut r[..], &mut g[..], &mut b[..]).unwrap();
        }

        let mut palette = [0; 0];
        assert_eq!(reader.read_palette(&mut palette).unwrap(), 0);
    }
}
