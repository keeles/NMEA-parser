use crate::ParseError;

#[derive(Debug, Default)]
pub struct Date {
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

impl Date {
    pub fn new(day: u8, month: u8, year: u16) -> Result<Date, ParseError> {
        if day == 0 || day > 31 || month == 0 || month > 12 {
            return Err(ParseError::InvalidDate);
        }

        Ok(Date { day, month, year })
    }
}
