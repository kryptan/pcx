use byteorder::ReadBytesExt;
use std::fs::File;
use std::io;
use std::path::Path;

use crate::low_level::rle::Decompressor;
use crate::low_level::{Header, PALETTE_START};
use crate::user_error;

#[derive(Clone, Debug)]
enum PixelReader<R: io::Read> {
    Compressed(Decompressor<R>),
    NotCompressed(R),
}

impl<R: io::Read> io::Read for PixelReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match *self {
            PixelReader::Compressed(ref mut decompressor) => decompressor.read(buffer),
            PixelReader::NotCompressed(ref mut stream) => stream.read(buffer),
        }
    }
}

/// PCX file reader.
#[derive(Clone, Debug)]
pub struct Reader<R: io::Read> {
    /// File header. All useful values are available via `Reader` methods so you don't actually need it.
    pub header: Header,

    pixel_reader: PixelReader<R>,
    num_lanes_read: u32,
}

impl Reader<io::BufReader<File>> {
    /// Start reading PCX file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Self::new(io::BufReader::new(file))
    }
}

impl<'a> Reader<io::Cursor<&'a [u8]>> {
    /// Start reading PCX file from memory buffer.
    pub fn from_mem(data: &'a [u8]) -> io::Result<Self> {
        Self::new(io::Cursor::new(data))
    }
}

impl<R: io::Read> Reader<R> {
    /// Start reading PCX file.
    pub fn new(mut stream: R) -> io::Result<Self> {
        let header = Header::load(&mut stream)?;
        let pixel_reader = if header.is_compressed {
            PixelReader::Compressed(Decompressor::new(stream))
        } else {
            PixelReader::NotCompressed(stream)
        };

        Ok(Reader {
            header,
            pixel_reader,
            num_lanes_read: 0,
        })
    }

    /// Get width and height of the image.
    #[inline]
    pub fn dimensions(&self) -> (u16, u16) {
        self.header.size
    }

    /// The width of this image.
    #[inline]
    pub fn width(&self) -> u16 {
        self.header.size.0
    }

    /// The height of this image.
    #[inline]
    pub fn height(&self) -> u16 {
        self.header.size.1
    }

    /// Whether this image is paletted or 24-bit RGB.
    #[inline]
    pub fn is_paletted(&self) -> bool {
        self.header.palette_length().is_some()
    }

    /// Get number of colors in the palette if this image is paletted. Number of colors is either 2, 4, 8, 16 or 256.
    #[inline]
    pub fn palette_length(&self) -> Option<u16> {
        self.header.palette_length()
    }

    /// Read next row of the paletted image.  Check that `is_paletted()` is `true` before calling this function.
    ///
    /// `buffer` length must be equal to the image width.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right.
    pub fn next_row_paletted(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        if !self.is_paletted() {
            return user_error("pcx::Reader::next_row_paletted called on non-paletted image");
        }

        if self.palette_length() == Some(256) {
            self.next_lane(buffer)?;
        } else if self.header.number_of_color_planes == 1 {
            // All packed formats, max. 16 colors.
            let width = self.width() as usize;
            let lane_length = self.header.lane_proper_length() as usize;
            let buffer_len = buffer.len();
            let offset = buffer.len() - lane_length;

            // Place packed row at the end of buffer, this will allow us to easily unpack it.
            self.next_lane(&mut buffer[offset..buffer_len])?;

            macro_rules! unpack_bits {
                ($bits:expr) => {{
                    let n = 8 / $bits;

                    for i in 0..(width * $bits) / 8 {
                        for j in 0..n {
                            buffer[i * n + j] = (buffer[offset + i]
                                & (((1 << $bits) - 1) << (8 - $bits * (j + 1))))
                                >> (8 - $bits * (j + 1));
                        }
                    }

                    let i = (width * $bits) / 8;
                    for j in 0..width - i * n {
                        buffer[i * n + j] = (buffer[offset + i]
                            & (((1 << $bits) - 1) << (8 - $bits * (j + 1))))
                            >> (8 - $bits * (j + 1));
                    }
                }};
            }

            // Unpack packed bits into bytes.
            match self.header.bit_depth {
                1 => unpack_bits!(1),
                2 => unpack_bits!(2),
                4 => unpack_bits!(4),
                _ => unreachable!(), // bit depth was checked while reading the header
            }
        } else {
            assert!(self.header.bit_depth == 1);
            // Planar, 4, 8 or 16 colors.
            let lane_length = self.header.lane_proper_length() as usize;
            let number_of_color_planes = self.header.number_of_color_planes as usize;

            buffer.fill(0);

            for d in 0..number_of_color_planes {
                for j in 0..lane_length {
                    let val = self.pixel_reader.read_u8()?;

                    for b in 0..8 {
                        if 8 * j + b < buffer.len() {
                            buffer[8 * j + b] |= ((val >> (7 - b)) & 0x1) << d;
                        }
                    }
                }
                self.skip_padding()?;
            }
        }

        Ok(())
    }

    /// Read next row of the RGB image to separate R, G and B buffers. Check that `is_paletted()` is `false` before calling this function.
    ///
    /// `r`, `g`, `b` buffer lengths must be equal to the image width.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right.
    pub fn next_row_rgb_separate(
        &mut self,
        r: &mut [u8],
        g: &mut [u8],
        b: &mut [u8],
    ) -> io::Result<()> {
        if self.is_paletted() {
            return user_error("pcx::Reader::next_row_rgb_separate called on paletted image");
        }

        // API for reading lanes is not exposed so users have no way of messing that up.
        assert_eq!(self.num_lanes_read % 3, 0);

        self.next_lane(r)?;
        self.next_lane(g)?;
        self.next_lane(b)
    }

    /// Read next row of the RGB image to one buffer with interleaved RGB values. Check that `is_paletted()` is `false` before calling this function.
    ///
    /// `rgb` buffer length must be equal to the image width multiplied by 3.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right.
    pub fn next_row_rgb(&mut self, rgb: &mut [u8]) -> io::Result<()> {
        if self.is_paletted() {
            return user_error("pcx::Reader::next_row_rgb called on paletted image");
        }

        // API for reading lanes is not exposed so users have no way of messing that up.
        assert_eq!(self.num_lanes_read % 3, 0);

        if rgb.len() != (self.width() as usize) * 3 {
            return user_error("pcx::Reader::next_row_rgb: buffer length must be equal to the width of the image multiplied by 3");
        }

        for color in 0..3 {
            for x in 0..(self.width() as usize) {
                rgb[x * 3 + color] = self.pixel_reader.read_u8()?;
            }
            self.skip_padding()?;
        }

        Ok(())
    }

    fn skip_padding(&mut self) -> io::Result<()> {
        if self.num_lanes_read + 1
            < u32::from(self.height()) * u32::from(self.header.number_of_color_planes)
        {
            // Skip padding.
            for _ in 0..self.header.lane_padding() {
                self.pixel_reader.read_u8()?;
            }
        }

        self.num_lanes_read += 1;
        Ok(())
    }

    // Read next lane. Format is dependent on file format. Buffer length must be equal to `Header::lane_proper_length()`.
    //
    // Order of lanes is from top to bottom.
    fn next_lane(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        use std::io::Read;

        if buffer.len() != self.header.lane_proper_length() as usize {
            return user_error("pcx::Reader::next_lane: incorrect buffer size.");
        }

        self.pixel_reader.read_exact(buffer)?;
        self.skip_padding()
    }

    /// Read color palette.
    ///
    /// If palette contains 256-colors then it is stored at the end of file and this function will read the file to the end.
    ///
    /// Returns number of colors in palette or zero if there is no palette. The actual number of bytes written to the output buffer is
    /// equal to the returned value multiplied by 3. Format of the output buffer is R, G, B, R, G, B, ...
    ///
    /// Consider using `get_palette` instead.
    pub fn read_palette(self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some(palette_size) = self.get_small_palette(buffer) {
            return Ok(palette_size);
        }

        // Stop decompressing and continue reading underlying stream.
        let mut stream = match self.pixel_reader {
            PixelReader::Compressed(decompressor) => decompressor.finish(),
            PixelReader::NotCompressed(stream) => stream,
        };

        // 256-color palette is located at the end of file. To avoid seeking we are using a bit convoluted method here to read it.
        const PALETTE_LENGTH: usize = 256 * 3;
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
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "no 256-color palette",
                    ));
                }

                buffer[0..(TEMP_BUFFER_LENGTH - pos - 1)]
                    .copy_from_slice(&temp_buffer[(pos + 1)..TEMP_BUFFER_LENGTH]);
                buffer[(TEMP_BUFFER_LENGTH - pos - 1)..PALETTE_LENGTH]
                    .copy_from_slice(&temp_buffer[0..pos]);

                return Ok(256);
            }
        }
    }

    fn get_small_palette(&self, buffer: &mut [u8]) -> Option<usize> {
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

                return Some(2 as usize);
            }
            Some(palette_length @ 1..=16) => {
                // Palettes of 16 colors or smaller are stored in the header.
                for i in 0..(palette_length as usize) {
                    (&mut buffer[(i * 3)..((i + 1) * 3)]).copy_from_slice(&self.header.palette[i]);
                }
                return Some(palette_length as usize);
            }
            Some(256) => {
                // 256-color palette is located at the end of file.
                None
            }
            _ => return Some(0),
        }
    }
}

impl<R: io::Seek + io::Read> Reader<R> {
    /// Read the entire RGB image, converting from paletted to RGB if necessarry.
    ///
    /// `rgb` buffer length must be equal to `width*height*3`.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right. Format of the
    /// output buffer is R, G, B, R, G, B, ...
    pub fn read_rgb_pixels(&mut self, rgb: &mut [u8]) -> io::Result<()> {
        let width = self.width() as usize;
        let height = self.height() as usize;
        let row_size = width * 3;

        if self.is_paletted() {
            let mut palette = [0; 256 * 3];
            self.get_palette(&mut palette)?;

            for y in 0..height {
                match self.next_row_paletted(&mut rgb[y * row_size..(y * row_size + width)]) {
                    // parse some weird images that appear in the wild
                    Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {}
                    Err(error) => {
                        return Err(error);
                    }
                    _ => {}
                }

                for x in (0..width).rev() {
                    let color_index = rgb[y * row_size + x] as usize;
                    rgb[y * row_size + x * 3 + 0] = palette[color_index * 3 + 0];
                    rgb[y * row_size + x * 3 + 1] = palette[color_index * 3 + 1];
                    rgb[y * row_size + x * 3 + 2] = palette[color_index * 3 + 2];
                }
            }
        } else {
            for y in 0..height {
                self.next_row_rgb(&mut rgb[y * row_size..(y + 1) * row_size])?;
            }
        }

        Ok(())
    }

    /// Get color palette.
    ///
    /// Returns number of colors in palette or zero if there is no palette. The actual number of bytes written to the output buffer is
    /// equal to the returned value multiplied by 3. Format of the output buffer is R, G, B, R, G, B, ...
    pub fn get_palette(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some(palette_size) = self.get_small_palette(buffer) {
            return Ok(palette_size);
        }

        let stream = match &mut self.pixel_reader {
            PixelReader::Compressed(decompressor) => &mut decompressor.stream,
            PixelReader::NotCompressed(stream) => stream,
        };

        let original_pos = stream.stream_position()?;

        stream.seek(io::SeekFrom::End(-256 * 3 - 1))?;
        let result = Self::get_palette_impl(stream, buffer);
        stream.seek(io::SeekFrom::Start(original_pos))?;
        result?;

        Ok(256)
    }

    fn get_palette_impl(stream: &mut R, buffer: &mut [u8]) -> io::Result<()> {
        let mut magic = [0];
        stream.read_exact(&mut magic)?;
        if magic[0] != PALETTE_START {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no 256-color palette",
            ));
        }

        stream.read_exact(&mut buffer[0..256 * 3])
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;
    use crate::low_level::header;

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

        let mut row: Vec<u8> = vec![0; reader.width() as usize];
        for _ in 0..reader.height() {
            reader.next_row_paletted(&mut row[..]).unwrap();
        }

        let mut palette = [0; 256 * 3];
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

        let mut r: Vec<u8> = vec![0; reader.width() as usize];
        let mut g: Vec<u8> = vec![0; reader.width() as usize];
        let mut b: Vec<u8> = vec![0; reader.width() as usize];
        for _ in 0..reader.height() {
            reader
                .next_row_rgb_separate(&mut r[..], &mut g[..], &mut b[..])
                .unwrap();
        }

        let mut palette = [0; 0];
        assert_eq!(reader.read_palette(&mut palette).unwrap(), 0);
    }
}
