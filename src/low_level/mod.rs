//! Low-level handling of PCX. You generally don't need to use this module.
pub mod header;
pub mod rle;

pub use self::header::Header;

/// Magic byte which is used as a first byte in all PCX files.
pub const MAGIC_BYTE: u8 = 0xA;

/// Byte marking the start of the 256-color palette.
pub const PALETTE_START: u8 = 0xC;
