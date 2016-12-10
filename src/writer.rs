use std::io;
use std::io::Write;
use byteorder::WriteBytesExt;

use low_level::header;
use low_level::rle::Compressor;
use low_level::PALETTE_START;

/// Create 24-bit RGB PCX image.
pub struct WriterRgb<W: io::Write> {
    compressor : Compressor<W>,
}

/// Create paletted PCX image.
pub struct WriterPaletted<W: io::Write> {
    compressor : Compressor<W>,
}

impl<W: io::Write> WriterRgb<W> {
    /// Create new PCX writer.
    ///
    /// If you are not sure what to pass to `dpi` value just use something like `(100, 100)` or `(300, 300)`.
    pub fn new(mut stream : W, image_size : (u16, u16), dpi : (u16, u16)) -> io::Result<Self> {
        header::write(&mut stream, false, image_size, dpi)?;

        let lane_length = image_size.0 + (image_size.0 & 1); // width rounded up to even

        Ok(WriterRgb {
            compressor : Compressor::new(stream, lane_length),
        })
    }

    /// Write next row of pixels.
    ///
    /// Length of each of `r`, `g` and `b` must be equal to the width of the image passed to `new`.
    /// This function must be called number of times equal to the height of the image.
    pub fn write_row(&mut self, r : &[u8], g : &[u8], b : &[u8]) -> io::Result<()> {
        self.compressor.write(r)?;
        self.compressor.pad()?;
        self.compressor.write(g)?;
        self.compressor.pad()?;
        self.compressor.write(b)?;
        self.compressor.pad()
    }

    /// Flush all data and finish writing.
    ///
    /// If you simply drop `WriterRgb` it will also flush everything but this function is preferable because errors won't be ignored.
    pub fn finish(mut self) -> io::Result<()> {
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
    pub fn new(mut stream : W, image_size : (u16, u16), dpi : (u16, u16)) -> io::Result<Self> {
        header::write(&mut stream, true, image_size, dpi)?;

        let lane_length = image_size.0 + (image_size.0 & 1); // width rounded up to even

        Ok(WriterPaletted {
            compressor : Compressor::new(stream, lane_length),
        })
    }

    /// Write next row of pixels.
    ///
    /// Row length must be equal to the width of the image passed to `new`.
    /// This function must be called number of times equal to the height of the image.
    pub fn write_row(&mut self, row : &[u8]) -> io::Result<()> {
        self.compressor.write(row)?;
        self.compressor.pad()
    }

    /// Since palette is written to the end of PCX file this function must be called only after writing all the pixels.
    ///
    /// Palette length must be 256*3 = 768 bytes. Format is R, G, B, R, G, B, ...
    pub fn write_palette(self, palette : &[u8]) -> io::Result<()> {
        let mut stream = self.compressor.finish()?;
        stream.write_u8(PALETTE_START)?;
        stream.write(palette)?;

        Ok(())
    }
}
