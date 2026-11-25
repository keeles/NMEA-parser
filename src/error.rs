#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    MissingDollar,
    MissingChecksum,
    BadChecksumHex,
    ChecksumMismatch,
    InvalidCoordinate,
    InvalidSpeed,
    InvalidCourse,
    InvalidTime,
    InvalidDate,
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
