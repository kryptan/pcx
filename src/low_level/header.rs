//! PCX file header.
use std::io;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use low_level::MAGIC_BYTE;

/*
typedef struct _PcxHeader
{
    BYTE	Identifier;        /* PCX Id Number (Always 0x0A) */
    BYTE	Version;           /* Version Number */
    BYTE	Encoding;          /* Encoding Format */
    BYTE	BitsPerPixel;      /* Bits per Pixel */
    WORD	XStart;            /* Left of image */
    WORD	YStart;            /* Top of Image */
    WORD	XEnd;              /* Right of Image */
    WORD	YEnd;              /* Bottom of image */
    WORD	HorzRes;           /* Horizontal Resolution */
    WORD	VertRes;           /* Vertical Resolution */
    BYTE	Palette[48];       /* 16-Color EGA Palette */
    BYTE	Reserved1;         /* Reserved (Always 0) */
    BYTE	NumBitPlanes;      /* Number of Bit Planes */
    WORD	BytesPerLine;      /* Bytes per Scan-line */
    WORD	PaletteType;       /* Palette Type */
    WORD	HorzScreenSize;    /* Horizontal Screen Size */
    WORD	VertScreenSize;    /* Vertical Screen Size */
    BYTE	Reserved2[54];     /* Reserved (Always 0) */
} PCXHEAD;
*/

/// File format version.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Version {
    V0 = 0,
    V2 = 2,
    V3 = 3,
    V4 = 4,
    V5 = 5,
}

/// Parsed header of PCX file.
#[derive(Copy, Clone, Debug)]
pub struct Header {
    /// Version of the file format.
    pub version : Version,

    /// Whether data in the file is RLE-compressed or not. Non-compressed files are non-standard but are supported by this library.
    pub is_compressed : bool,

    /// Bits per pixel per color. Either 1, 2, 4 or 8.
    pub bit_depth : u8,

    /// Width and height of the image.
    pub size : (u16, u16),

    /// Offset indicating where to render this image. This is usually set to `(0, 0)` and can be ignored.
    pub start : (u16, u16),

    /// Dots per inch.
    pub dpi : (u16, u16),

    /// Color palette.
    pub palette : [[u8; 3]; 16],

    /// Number of color channels in the image.
    pub number_of_color_planes : u8,

    /// Lane length including padding bytes.
    pub lane_length : u16,
}

fn error<T>(msg : &str) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, msg))
}

fn lane_proper_length(width : u16, bit_depth : u8) -> u16 {
    (((width as u32)*(bit_depth as u32) - 1)/8 + 1) as u16
}

impl Header {
    pub fn load<R : io::Read>(stream : &mut R) -> io::Result<Self> {
        let magic = stream.read_u8()?;
        if magic != MAGIC_BYTE {
            return error("not a PCX file");
        }

        let version = match stream.read_u8()? {
            0 => Version::V0,
            2 => Version::V2,
            3 => Version::V3,
            4 => Version::V4,
            5 => Version::V5,
            _ => return error("unknown PCX version"),
        };

        let encoding = stream.read_u8()?;
        if encoding != 0 && encoding != 1 {
            return error("unknown PCX encoding");
        }

        let bit_depth = stream.read_u8()?;

        let x_start = stream.read_u16::<LittleEndian>()?;
        let y_start = stream.read_u16::<LittleEndian>()?;
        let x_end = stream.read_u16::<LittleEndian>()?;
        let y_end = stream.read_u16::<LittleEndian>()?;

        if x_end < x_start || y_end < y_start {
            return error("PCX: invalid dimensions");
        }

        let x_dpi = stream.read_u16::<LittleEndian>()?;
        let y_dpi = stream.read_u16::<LittleEndian>()?;

        let mut palette = [[0; 3]; 16];
        for i in 0..16 {
            stream.read_exact(&mut palette[i])?;
        }

        let _reserved_0 = stream.read_u8()?;
        let number_of_color_planes = stream.read_u8()?;
        let lane_length = stream.read_u16::<LittleEndian>()?;
        let _palette_kind = stream.read_u16::<LittleEndian>()?;

        let mut _reserved_1 = [0; 58];
        stream.read_exact(&mut _reserved_1)?;

        // Must be one of the supported format.
        match (number_of_color_planes, bit_depth) {
            (3, 8) | // 24-bit RGB
            (1, 1) | // monochrome
            (1, 2) | // 4-color palette
            (1, 4) | // 16-color palette
            (1, 8) | // 256-color palette
            (2, 1) |
            (3, 1) |
            (4, 1) => {},
            _ => return error("PCX: invalid or unsupported color format"),
        }

        if lane_length < lane_proper_length(x_end - x_start, bit_depth) {
            return error("PCX: invalid lane length");
        }

        Ok(Header{
            version : version,
            is_compressed : encoding == 1,
            bit_depth : bit_depth,
            size : (x_end + 1 - x_start, y_end + 1 - y_start),
            start : (x_start, y_start),
            dpi : (x_dpi, y_dpi),
            palette : palette,
            number_of_color_planes : number_of_color_planes,
            lane_length : lane_length,
        })
    }

    /// Length of each lane without padding.
    pub fn lane_proper_length(&self) -> u16 {
        lane_proper_length(self.size.0, self.bit_depth)
    }

    /// Number of padding bytes in each lane.
    pub fn lane_padding(&self) -> u16 {
        self.lane_length - self.lane_proper_length()
    }

    pub fn palette_length(&self) -> Option<u16> {
        match (self.number_of_color_planes, self.bit_depth) {
            (3, 8) => None,
            (number_of_color_planes, bit_depth) => Some((1 << bit_depth as u16)*(number_of_color_planes as u16)),
        }
    }
}

/// Write header to the stream.
pub fn write<W: io::Write>(stream : &mut W, paletted : bool, size : (u16, u16), dpi : (u16, u16)) -> io::Result<()> {
    if size.0 == 0xFFFF { // we'll need to round width up to even number which is not possible for 0xFFFF due to overflow
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "cannot save PCX with width equal to 0xFFFF"))
    }

    // Write header.
    stream.write_u8(MAGIC_BYTE)?;
    stream.write_u8(Version::V5 as u8)?;
    stream.write_u8(1)?; // encoding = compressed
    stream.write_u8(8)?; // bit depth
    stream.write_u16::<LittleEndian>(0)?; // x_start
    stream.write_u16::<LittleEndian>(0)?; // y_start
    stream.write_u16::<LittleEndian>(size.0)?;
    stream.write_u16::<LittleEndian>(size.1)?;
    stream.write_u16::<LittleEndian>(dpi.0)?;
    stream.write_u16::<LittleEndian>(dpi.1)?;

    // Write 16-color palette (not used as we will use 256-color palette instead).
    for _ in 0..16 {
        stream.write(&[0, 0, 0])?;
    }

    let lane_length = size.0 + (size.0 & 1); // width rounded up to even

    stream.write_u8(0)?; // reserved
    stream.write_u8(if paletted { 1 } else { 3 })?; // number of color planes
    stream.write_u16::<LittleEndian>(lane_length)?;
    stream.write_u16::<LittleEndian>(1)?; // palette kind (not used)

    // Unused values in header.
    for _ in 0..58 {
        stream.write(&[0])?;
    }

    Ok(())
}
