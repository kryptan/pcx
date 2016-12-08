use std::io;
use byteorder::{LittleEndian, ReadBytesExt};

use DecodingError;

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

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
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

impl Header {
    pub fn load<R : io::Read>(stream : &mut R) -> Result<Self, DecodingError> {
        let magic = stream.read_u8()?;
        if magic != 0xA {
            return Err(DecodingError::NotPcx);
        }

        let version = match stream.read_u8()? {
            0 => Version::V0,
            2 => Version::V2,
            3 => Version::V3,
            4 => Version::V4,
            5 => Version::V5,
            v => return Err(DecodingError::UnknownVersion(v)),
        };

        let encoding = stream.read_u8()?;
        if encoding != 0 && encoding != 1 {
            return Err(DecodingError::UnknownEncoding(encoding));
        }

        let bit_depth = stream.read_u8()?;
        if ![1u8, 2, 4, 8].contains(&bit_depth) {
            return Err(DecodingError::InvalidBitsPerPlane(bit_depth));
        }

        let x_start = stream.read_u16::<LittleEndian>()?;
        let y_start = stream.read_u16::<LittleEndian>()?;
        let x_end = stream.read_u16::<LittleEndian>()?;
        let y_end = stream.read_u16::<LittleEndian>()?;

        if x_end < x_start || y_end < y_start {
            return Err(DecodingError::InvalidData);
        }

        let x_dpi = stream.read_u16::<LittleEndian>()?;
        let y_dpi = stream.read_u16::<LittleEndian>()?;

        let mut palette = [[0; 3]; 16];
        for i in 0..16 {
            stream.read_exact(&mut palette[i])?;
        }

        let _reserved_0 = stream.read_u8()?;

        let number_of_color_planes = stream.read_u8()?;
        if ![1u8, 3, 4].contains(&number_of_color_planes) {
            return Err(DecodingError::InvalidNumberOfPlanes(number_of_color_planes));
        }

        let lane_length = stream.read_u16::<LittleEndian>()?;
        let palette_kind = stream.read_u16::<LittleEndian>()?;

        let mut _reserved_1 = [0; 58];
        stream.read_exact(&mut _reserved_1)?;

        let header = Header{
            version : version,
            is_compressed : encoding == 1,
            bit_depth : bit_depth,
            size : (x_end + 1 - x_start, y_end + 1 - y_start),
            start : (x_start, y_start),
            dpi : (x_dpi, y_dpi),
            palette : palette,
            number_of_color_planes : number_of_color_planes,
            lane_length : lane_length,
        };

        match header.palette_length() {
            None | Some(1 ... 16) | Some(256) => {},
            _ => return Err(DecodingError::InvalidData),
        }

        Ok(header)
    }

    /// Length of each lane without padding.
    pub fn lane_proper_length(&self) -> u16 {
        ((self.number_of_color_planes as u32)*(self.size.0 as u32)*(self.bit_depth as u32)/8) as u16
    }

    /// Number of padding bytes in each lane.
    pub fn lane_padding(&self) -> u16 {
        self.lane_length - self.lane_proper_length()
    }

    pub fn palette_length(&self) -> Option<u16> {
        match (self.number_of_color_planes, self.bit_depth) {
            (3, 8) => None,
            (1, 8) => Some(256),
            (number_of_color_planes, bit_depth) => Some((1 << bit_depth as u16)*(number_of_color_planes as u16)),
        }
    }
}
