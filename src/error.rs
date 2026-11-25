#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    MissingDollar,
    MissingChecksum,
    BadChecksumHex,
    ChecksumMismatch,
    InvalidCoordinate,
    InvalidSpeed,
    InvalidTime,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LineError {
    TooManyCols,
    InvalidValue,
    InsufficientFields,
}

impl From<ParseError> for LineError {
    fn from(_: ParseError) -> Self {
        LineError::InvalidValue
    }
}
