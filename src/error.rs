use std::io;

#[derive(Debug)]
pub enum DecodingError {
    NotPcx,
    UnknownVersion(u8),
    UnknownEncoding(u8),
    InvalidBitsPerPlane(u8),
    InvalidNumberOfPlanes(u8),
    InvalidData,
    IoError(io::Error),
}

impl From<io::Error> for DecodingError {
    fn from(err: io::Error) -> DecodingError {
        DecodingError::IoError(err)
    }
}
