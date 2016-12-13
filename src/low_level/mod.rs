//! Low-level handling of PCX. You generally don't need to use this module.
pub mod rle;
pub mod header;

pub use self::header::Header;

/// Magic byte which is used as first byte in all PCX files.
pub const MAGIC_BYTE: u8 = 0xA;

/// Byte marking start of 256-color palette.
pub const PALETTE_START: u8 = 0xC;
