use std::io;
use std::io::Write;
use byteorder::WriteBytesExt;

use user_error;
use low_level::header;
use low_level::rle::Compressor;
use low_level::PALETTE_START;

/// Create 24-bit RGB PCX image.
pub struct WriterRgb<W: io::Write> {
    compressor: Compressor<W>,
    num_rows_left: u16,
    width: u16,
}

/// Create paletted PCX image.
pub struct WriterPaletted<W: io::Write> {
    compressor: Compressor<W>,
    num_rows_left: u16,
    width: u16,
}

impl<W: io::Write> WriterRgb<W> {
    /// Create new PCX writer.
    ///
    /// If you are not sure what to pass to `dpi` value just use something like `(100, 100)` or `(300, 300)`.
    pub fn new(mut stream: W, image_size: (u16, u16), dpi: (u16, u16)) -> io::Result<Self> {
        header::write(&mut stream, false, image_size, dpi)?;

        let lane_length = image_size.0 + (image_size.0 & 1); // width rounded up to even

        Ok(WriterRgb {
            compressor: Compressor::new(stream, lane_length),
            width: image_size.0,
            num_rows_left: image_size.1,
        })
    }

    /// Write next row of pixels from separate buffers for R, G and B channels.
    ///
    /// Length of each of `r`, `g` and `b` must be equal to the width of the image passed to `new`.
    /// This function must be called number of times equal to the height of the image.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right.
    pub fn write_row_from_separate(&mut self, r: &[u8], g: &[u8], b: &[u8]) -> io::Result<()> {
        if self.num_rows_left == 0 {
            return user_error("pcx::WriterRgb::write_row_from_separate: all rows were already written");
        }

        let width = self.width as usize;
        if r.len() != width || g.len() != width || b.len() != width {
            return user_error("pcx::WriterRgb::write_row_from_separate: buffer lengths must be equal to the width of the image");
        }

        self.compressor.write(r)?;
        self.compressor.pad()?;
        self.compressor.write(g)?;
        self.compressor.pad()?;
        self.compressor.write(b)?;
        self.compressor.pad()?;

        self.num_rows_left -= 1;
        Ok(())
    }

    /// Write next row of pixels from buffer which contain RGB values interleaved (i.e. R, G, B, R, G, B, ...).
    ///
    /// Length of the `rgb` buffer must be equal to the width of the image passed to `new` multiplied by 3.
    /// This function must be called number of times equal to the height of the image.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right.
    pub fn write_row_from_interleaved(&mut self, rgb: &[u8]) -> io::Result<()> {
        if self.num_rows_left == 0 {
            return user_error("pcx::WriterRgb::write_row_from_interleaved: all rows were already written");
        }

        if rgb.len() != (self.width as usize) * 3 {
            return user_error("pcx::WriterRgb::write_row_from_interleaved: buffer length must be equal to the width of the image multiplied by 3");
        }

        for color in 0..3 {
            for x in 0..(self.width as usize) {
                self.compressor.write_u8(rgb[x * 3 + color])?;
            }
            self.compressor.pad()?;
        }

        self.num_rows_left -= 1;
        Ok(())
    }

    /// Flush all data and finish writing.
    ///
    /// If you simply drop `WriterRgb` it will also flush everything but this function is preferable because errors won't be ignored.
    pub fn finish(mut self) -> io::Result<()> {
        if self.num_rows_left != 0 {
            return user_error("pcx::WriterRgb::finish: not all rows written");
        }

        self.compressor.flush()
    }
}

impl<W: io::Write> Drop for WriterRgb<W> {
    fn drop(&mut self) {
        let _r = self.compressor.flush();
    }
}

impl<W: io::Write> WriterPaletted<W> {
    /// Create new PCX writer.
    ///
    /// If you are not sure what to pass to `dpi` value just use something like `(100, 100)` or `(300, 300)`.
    pub fn new(mut stream: W, image_size: (u16, u16), dpi: (u16, u16)) -> io::Result<Self> {
        header::write(&mut stream, true, image_size, dpi)?;

        let lane_length = image_size.0 + (image_size.0 & 1); // width rounded up to even

        Ok(WriterPaletted {
            compressor: Compressor::new(stream, lane_length),
            width: image_size.0,
            num_rows_left: image_size.1,
        })
    }

    /// Write next row of pixels.
    ///
    /// Row length must be equal to the width of the image passed to `new`.
    /// This function must be called number of times equal to the height of the image.
    ///
    /// Order of rows is from top to bottom, order of pixels is from left to right.
    pub fn write_row(&mut self, row: &[u8]) -> io::Result<()> {
        if self.num_rows_left == 0 {
            return user_error("pcx::WriterPaletted::write_row: all rows were already written");
        }

        if row.len() != self.width as usize {
            return user_error("pcx::WriterPaletted::write_row: buffer length must be equal to the width of the image");
        }

        self.compressor.write(row)?;
        self.compressor.pad()?;

        self.num_rows_left -= 1;
        Ok(())
    }

    /// Since palette is written to the end of PCX file this function must be called only after writing all the pixels.
    ///
    /// Palette length must be not larger than 256*3 = 768 bytes and be divisible by 3. Format is R, G, B, R, G, B, ...
    pub fn write_palette(self, palette: &[u8]) -> io::Result<()> {
        if self.num_rows_left != 0 {
            return user_error("pcx::WriterPaletted::write_palette: not all rows written");
        }

        if palette.len() > 256 * 3 || palette.len() % 3 != 0 {
            return user_error("pcx::WriterPaletted::write_palette: incorrect palette length");
        }

        let mut stream = self.compressor.finish()?;
        stream.write_u8(PALETTE_START)?;
        stream.write(palette)?;
        for _ in 0..(256 * 3 - palette.len()) {
            stream.write_u8(0)?;
        }

        Ok(())
    }
}
