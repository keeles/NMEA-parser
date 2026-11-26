use crate::ParseError;

/// Takes a byte array of length 6, returns a tuple of day, month, year
pub fn parse_date(bytes: &[u8]) -> Result<(u8, u8, u16), ParseError> {
    fn inner(bytes: &[u8]) -> Option<(u8, u8, u16)> {
        if bytes.len() != 6 {
            return None;
        }
        let day = u8::try_from(parse_u64(&bytes[0..2])?).ok()?;
        let month = u8::try_from(parse_u64(&bytes[2..4])?).ok()?;
        let year = u16::try_from(parse_u64(&bytes[4..6])?).ok()?;
        Some((day, month, year))
    }
    inner(bytes).ok_or(ParseError::InvalidDate)
}

/// Return utc time as u32
pub fn parse_utc_ms(bytes: &[u8]) -> Result<u32, ParseError> {
    let (hms, frac) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // some receivers omit the fraction
    };

    fn inner(hms: &[u8], frac: &[u8]) -> Option<u32> {
        if hms.len() != 6 {
            return None;
        }

        let hh = parse_u64(&hms[0..2])?;
        let mm = parse_u64(&hms[2..4])?;
        let ss = parse_u64(&hms[4..6])?;
        let ms = parse_decimal(frac, 3)?;

        if hh > 23 || mm > 59 || ss > 60 {
            return None;
        }

        Some(((hh * 3600 + mm * 60 + ss) * 1000 + ms) as u32)
    }

    inner(hms, frac).ok_or(ParseError::InvalidTime)
}

/// Takes a byte string of arbitrary length, splits at '.'
/// returns a u32 representing a floating point number of the provided precision
pub fn parse_decimal_to_uint(bytes: &[u8], precision: u32) -> Result<u32, ParseError> {
    let (int_part, frac_part) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // tolerate "5" with no decimal point
    };

    fn inner(int_part: &[u8], frac_part: &[u8], precision: u32) -> Option<u32> {
        let int = parse_u64(int_part)?;
        let frac = parse_decimal(frac_part, precision)?;

        let total = int
            .checked_mul(10u64.pow(precision))
            .and_then(|v| v.checked_add(frac))?;

        u32::try_from(total).ok()
    }

    inner(int_part, frac_part, precision).ok_or(ParseError::InvalidSpeed)
}

/// Takes a byte string of arbitrary length, splits at '.'
/// returns a u32 representing a floating point number of the provided precision
pub fn parse_decimal_to_int(bytes: &[u8], precision: u32) -> Result<i32, ParseError> {
    let (int_part, frac_part) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // tolerate "5" with no decimal point
    };

    fn inner(int_part: &[u8], frac_part: &[u8], precision: u32) -> Option<i32> {
        let int = parse_i32(int_part)?;
        let frac = parse_decimal(frac_part, precision)?;

        let total = int
            .checked_mul(10i32.pow(precision))
            .and_then(|v| v.checked_add(frac as i32))?;

        i32::try_from(total).ok()
    }

    inner(int_part, frac_part, precision).ok_or(ParseError::InvalidSpeed)
}

/// Takes a byte array of speed in knots, splits at '.'
/// returns speed as mm/s with decimal precision of 3
pub fn parse_speed_mms(bytes: &[u8]) -> Result<u32, ParseError> {
    let (int_part, frac_part) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // tolerate "5" with no decimal point
    };

    fn inner(int_part: &[u8], frac_part: &[u8], precision: u32) -> Option<u32> {
        let int = parse_u64(int_part)?;
        let frac = parse_decimal(frac_part, precision)?;

        let milli_knots = int.checked_mul(1000).and_then(|v| v.checked_add(frac))?;

        let mms = milli_knots
            .checked_mul(1852)
            .and_then(|v| v.checked_add(1800))?;
        let mms = mms / 3600;

        u32::try_from(mms).ok()
    }

    inner(int_part, frac_part, 3).ok_or(ParseError::InvalidSpeed)
}

/// parses byte array into lat_e7 or lon_e7 coordinate
/// decimal precision of 7
pub fn parse_coord(bytes: &[u8]) -> Result<i32, ParseError> {
    fn inner(bytes: &[u8]) -> Option<i32> {
        let dot_idx = bytes.iter().position(|&b| b == b'.')?;
        if dot_idx < 3 {
            return None;
        }

        let int_part = &bytes[..dot_idx];
        let deg = parse_u64(&int_part[..int_part.len() - 2])?;
        let mins = parse_u64(&int_part[int_part.len() - 2..])?;

        if mins >= 60 || deg > 180 {
            return None;
        }

        let frac = parse_decimal(&bytes[dot_idx + 1..], 7)?;

        // minutes scaled by 10^7, round (+30), divide by 60 -> fractional degrees
        let total_mins_e7 = mins * 10_000_000 + frac;
        let total_deg_e7 = deg * 10_000_000 + (total_mins_e7 + 30) / 60;

        i32::try_from(total_deg_e7).ok()
    }

    inner(bytes).ok_or(ParseError::InvalidCoordinate)
}

/// Takes byte array and returns u64 representation
pub fn parse_u64(bytes: &[u8]) -> Option<u64> {
    bytes.iter().try_fold(0u64, |acc, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        acc.checked_mul(10)?.checked_add((b - b'0') as u64)
    })
}

/// Takes byte array and returns i32 representation
pub fn parse_i32(bytes: &[u8]) -> Option<i32> {
    bytes.iter().try_fold(0i32, |acc, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        acc.checked_mul(10)?.checked_add((b - b'0') as i32)
    })
}

/// Takes byte array and returns u64 representation
pub fn parse_u32(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0u32, |acc, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        acc.checked_mul(10)?.checked_add((b - b'0') as u32)
    })
}

/// Takes byte array and returns u64 representation of floating point of provided decimal precision
pub fn parse_decimal(bytes: &[u8], precision: u32) -> Option<u64> {
    let mut acc = 0u64;
    let mut count = 0u32;

    for &b in bytes.iter().take(precision as usize) {
        if !b.is_ascii_digit() {
            return None;
        }
        acc = acc * 10 + (b - b'0') as u64;
        count += 1;
    }

    // Also validate any remaining digits beyond 7 aren't garbage characters
    if !bytes
        .iter()
        .skip(precision as usize)
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }

    // Pad right with zeros if fewer than 7 digits (e.g., "11437" -> 1143700)
    Some(acc * 10u64.pow(precision as u32 - count))
}
