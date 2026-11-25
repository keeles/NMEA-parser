use crate::{ParseError, error::LineError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RMCStatus {
    A,
    #[default]
    V,
}

#[derive(Debug, Default)]
pub struct Date {
    day: u8,
    month: u8,
    year: u16,
}

impl Date {
    pub fn new(day: u8, month: u8, year: u16) -> Result<Date, ParseError> {
        if day == 0 || day > 31 || month == 0 || month > 12 {
            return Err(ParseError::InvalidDate);
        }

        Ok(Date { day, month, year })
    }
}

#[derive(Debug, Default)]
pub struct RMC {
    utc_ms: u32,
    status: RMCStatus,
    lat_e7: i32,
    lon_e7: i32,
    sog_mms: u32,
    angle_mdeg: u32,
    date: Date,
}

impl RMC {
    pub fn new() -> RMC {
        RMC {
            utc_ms: 0,
            status: RMCStatus::V,
            lat_e7: 0,
            lon_e7: 0,
            sog_mms: 0,
            angle_mdeg: 0,
            date: Date::default(),
        }
    }
}

pub fn build_rmc_struct(line: &[u8]) -> Result<RMC, LineError> {
    let mut rmc_data = RMC::default();

    for (col, field) in line.split(|&b| b == b',').enumerate() {
        if field.is_empty() {
            continue;
        }

        match col {
            0 => {
                rmc_data.utc_ms = parse_utc_ms(field)?;
            }
            1 => {
                //store status
                rmc_data.status = if field.first() == Some(&b'A') {
                    RMCStatus::A
                } else {
                    RMCStatus::V
                };
            }
            2 => {
                // store lat_e7
                rmc_data.lat_e7 = parse_coord(field)?;
            }
            3 => match field.first() {
                Some(b'N') => {}
                Some(b'S') => rmc_data.lat_e7 = -rmc_data.lat_e7,
                None => {} // empty when there's no fix
                _ => return Err(LineError::InvalidValue),
            },
            4 => {
                // store lon_e7
                rmc_data.lon_e7 = parse_coord(field)?;
            }
            5 => match field.first() {
                Some(b'E') => {}
                Some(b'W') => rmc_data.lon_e7 = -rmc_data.lon_e7,
                None => {}
                _ => return Err(LineError::InvalidValue),
            },
            6 => {
                // parse speed over ground
                rmc_data.sog_mms = parse_speed_mms(field)?;
            }
            7 => {
                let angle = parse_decimal_to_int(field, 3)?;
                if angle >= 360_000 {
                    return Err(LineError::InvalidValue);
                }

                rmc_data.angle_mdeg = parse_decimal_to_int(field, 3)?;
            }
            8 => {
                let (day, month, year) = parse_date(field)?;
                rmc_data.date = Date::new(day, month, year)?;
            }
            _ => continue,
        }
    }

    Ok(rmc_data)
}

/// Takes a byte array of length 6, returns a tuple of day, month, year
fn parse_date(bytes: &[u8]) -> Result<(u8, u8, u16), ParseError> {
    if bytes.len() != 6 {
        return Err(ParseError::InvalidDate);
    }

    let day = parse_u64(&bytes[0..2]).ok_or(ParseError::InvalidDate)?;
    let day = u8::try_from(day).map_err(|_| ParseError::InvalidDate)?;
    let month = parse_u64(&bytes[2..4]).ok_or(ParseError::InvalidDate)?;
    let month = u8::try_from(month).map_err(|_| ParseError::InvalidDate)?;
    let year = parse_u64(&bytes[4..6]).ok_or(ParseError::InvalidDate)?;
    let year = u16::try_from(year).map_err(|_| ParseError::InvalidDate)?;

    Ok((day, month, year))
}

/// Return utc time as u32
fn parse_utc_ms(bytes: &[u8]) -> Result<u32, ParseError> {
    let (hms, frac) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // some receivers omit the fraction
    };

    if hms.len() != 6 {
        return Err(ParseError::InvalidTime);
    }

    let hh = parse_u64(&hms[0..2]).ok_or(ParseError::InvalidTime)?;
    let mm = parse_u64(&hms[2..4]).ok_or(ParseError::InvalidTime)?;
    let ss = parse_u64(&hms[4..6]).ok_or(ParseError::InvalidTime)?;
    let ms = parse_decimal(frac, 3).ok_or(ParseError::InvalidTime)?;

    // ss == 60 is a legal leap second
    if hh > 23 || mm > 59 || ss > 60 {
        return Err(ParseError::InvalidTime);
    }

    Ok(((hh * 3600 + mm * 60 + ss) * 1000 + ms) as u32)
}

/// Takes a byte string of arbitrary length, splits at '.'
/// returns an int representing a floating point number of the provided precision
fn parse_decimal_to_int(bytes: &[u8], precision: u32) -> Result<u32, ParseError> {
    let (int_part, frac_part) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // tolerate "5" with no decimal point
    };

    let int = parse_u64(int_part).ok_or(ParseError::InvalidSpeed)?;
    let int = u32::try_from(int).map_err(|_| ParseError::InvalidSpeed)?;
    let frac = parse_decimal(frac_part, precision).ok_or(ParseError::InvalidSpeed)?;
    let frac = u32::try_from(frac).map_err(|_| ParseError::InvalidSpeed)?;

    let total = int
        .checked_mul(10u32.pow(precision))
        .and_then(|v| v.checked_add(frac))
        .ok_or(ParseError::InvalidSpeed)?;
    Ok(total)
}

/// Takes a byte array of speed in knots, splits at '.'
/// returns speed as mm/s with decimal precision of 3
fn parse_speed_mms(bytes: &[u8]) -> Result<u32, ParseError> {
    let (int_part, frac_part) = match bytes.iter().position(|&b| b == b'.') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[][..]), // tolerate "5" with no decimal point
    };

    let int = parse_u64(int_part).ok_or(ParseError::InvalidSpeed)?;
    let frac = parse_decimal(frac_part, 3).ok_or(ParseError::InvalidSpeed)?;

    let milli_knots = int
        .checked_mul(1000)
        .and_then(|v| v.checked_add(frac))
        .ok_or(ParseError::InvalidSpeed)?;

    let mms = milli_knots
        .checked_mul(1852)
        .and_then(|v| v.checked_add(1800))
        .ok_or(ParseError::InvalidSpeed)?
        / 3600;

    u32::try_from(mms).map_err(|_| ParseError::InvalidSpeed)
}

/// parses byte array into lat_e7 or lon_e7 coordinate
/// decimal precision of 7
fn parse_coord(bytes: &[u8]) -> Result<i32, ParseError> {
    let dot_idx = bytes
        .iter()
        .position(|&b| b == b'.')
        .ok_or(ParseError::InvalidCoordinate)?;

    if dot_idx < 3 {
        return Err(ParseError::InvalidCoordinate);
    }

    let int_part = &bytes[..dot_idx];
    let deg = parse_u64(&int_part[..int_part.len() - 2]).ok_or(ParseError::InvalidCoordinate)?;
    let mins = parse_u64(&int_part[int_part.len() - 2..]).ok_or(ParseError::InvalidCoordinate)?;

    if mins >= 60 || deg > 180 {
        return Err(ParseError::InvalidCoordinate);
    }

    let frac = parse_decimal(&bytes[dot_idx + 1..], 7).ok_or(ParseError::InvalidCoordinate)?;

    // Combine minutes (scaled by 10^7), round (+30), and divide by 60 to get fractional degrees
    let total_mins_e7 = mins * 10_000_000 + frac;
    let total_deg_e7 = deg * 10_000_000 + (total_mins_e7 + 30) / 60;

    i32::try_from(total_deg_e7).map_err(|_| ParseError::InvalidCoordinate)
}

/// Takes byte array and returns u64 representation
fn parse_u64(bytes: &[u8]) -> Option<u64> {
    bytes.iter().try_fold(0u64, |acc, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        acc.checked_mul(10)?.checked_add((b - b'0') as u64)
    })
}

/// Takes byte array and returns u64 representation of floating point of provided decimal precision
fn parse_decimal(bytes: &[u8], precision: u32) -> Option<u64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- parse_u64 ----------

    #[test]
    fn u64_valid() {
        assert_eq!(parse_u64(b"0"), Some(0));
        assert_eq!(parse_u64(b"007"), Some(7));
        assert_eq!(parse_u64(b"1234567890"), Some(1_234_567_890));
        assert_eq!(parse_u64(b"18446744073709551615"), Some(u64::MAX));
    }

    #[test]
    fn u64_empty_is_zero() {
        // Documents current behaviour: callers must check is_empty themselves.
        assert_eq!(parse_u64(b""), Some(0));
    }

    #[test]
    fn u64_rejects_garbage_and_overflow() {
        assert_eq!(parse_u64(b"12a"), None);
        assert_eq!(parse_u64(b"-1"), None);
        assert_eq!(parse_u64(b" 1"), None);
        assert_eq!(parse_u64(b"18446744073709551616"), None); // u64::MAX + 1
    }

    // ---------- parse_decimal ----------

    #[test]
    fn decimal_pads_right() {
        assert_eq!(parse_decimal(b"5", 3), Some(500));
        assert_eq!(parse_decimal(b"05", 3), Some(50));
        assert_eq!(parse_decimal(b"", 3), Some(0));
        assert_eq!(parse_decimal(b"11437", 7), Some(1_143_700));
        assert_eq!(parse_decimal(b"038", 7), Some(380_000));
    }

    #[test]
    fn decimal_truncates_extra_digits() {
        assert_eq!(parse_decimal(b"123456", 3), Some(123));
        assert_eq!(parse_decimal(b"9999999999", 7), Some(9_999_999));
    }

    #[test]
    fn decimal_rejects_garbage() {
        assert_eq!(parse_decimal(b"12x", 3), None);
        assert_eq!(parse_decimal(b"123x", 3), None); // garbage past the precision
        assert_eq!(parse_decimal(b"1.2", 3), None);
    }

    // ---------- parse_coord ----------

    #[test]
    fn coord_known_values() {
        let cases: &[(&[u8], i32)] = &[
            (b"4807.038", 481_173_000),  // 48.1173
            (b"01131.000", 115_166_667), // 11 deg 31' = 11.5166667 (rounded)
            (b"4916.45", 492_741_667),   // 49 deg 16.45'
            (b"0000.0000", 0),
            (b"3351.000", 338_500_000),     // exact: 33.85
            (b"15112.000", 1_512_000_000),  // exact: 151.2
            (b"18000.0000", 1_800_000_000), // 180 deg, fits in i32
        ];
        for &(input, expected) in cases {
            assert_eq!(
                parse_coord(input).unwrap(),
                expected,
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn coord_rejects_bad_input() {
        let bad: &[&[u8]] = &[
            b"4807",      // no decimal point
            b"12.5",      // dot too early for dd mm
            b"48a7.038",  // non-digit
            b"4807.03x",  // non-digit in fraction
            b"4860.000",  // minutes >= 60
            b"18100.000", // degrees > 180
            b"",
        ];
        for &input in bad {
            assert!(
                matches!(parse_coord(input), Err(ParseError::InvalidCoordinate)),
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    // ---------- parse_speed_mms ----------

    #[test]
    fn speed_known_values() {
        let cases: &[(&[u8], u32)] = &[
            (b"0.0", 0),
            (b"0.000", 0),
            (b"0.001", 1),     // 0.514 mm/s rounds up
            (b"1.00", 514),    // 514.44
            (b"0.54", 278),    // 277.8
            (b"22.4", 11_524), // 11523.56
            (b"5", 2_572),     // no decimal point
            (b".5", 257),      // no integer part
            (b"10.000", 5_144),
            (b"100.0", 51_444),
            (b"022.4", 11_524), // leading zeros, as receivers emit them
        ];
        for &(input, expected) in cases {
            assert_eq!(
                parse_speed_mms(input).unwrap(),
                expected,
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn speed_rejects_bad_input() {
        let bad: &[&[u8]] = &[b"abc", b"1.2x", b"1.2.3", b"-1.0", b"1,5"];
        for &input in bad {
            assert!(
                matches!(parse_speed_mms(input), Err(ParseError::InvalidSpeed)),
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn speed_too_large_for_u32() {
        // fits in u64 internally, but the result exceeds u32::MAX mm/s
        assert!(matches!(
            parse_speed_mms(b"10000000000.0"),
            Err(ParseError::InvalidSpeed)
        ));
    }

    #[test]
    fn speed_u64_overflow_is_an_error_not_a_panic() {
        assert!(matches!(
            parse_speed_mms(b"99999999999999.0"),
            Err(ParseError::InvalidSpeed)
        ));
    }

    #[test]
    fn rmc_full_sentence_north_east() {
        let rmc =
            build_rmc_struct(b",A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W").unwrap();
        assert_eq!(rmc.status, RMCStatus::A);
        assert_eq!(rmc.lat_e7, 481_173_000);
        assert_eq!(rmc.lon_e7, 115_166_667);
        assert_eq!(rmc.sog_mms, 11_524);
    }

    #[test]
    fn rmc_south_west_are_negative() {
        let rmc = build_rmc_struct(b",A,3351.000,S,15112.000,W,000.0").unwrap();
        assert_eq!(rmc.lat_e7, -338_500_000);
        assert_eq!(rmc.lon_e7, -1_512_000_000);
        assert_eq!(rmc.sog_mms, 0);
    }

    #[test]
    fn rmc_south_east_mixed() {
        let rmc = build_rmc_struct(b",A,3351.000,S,15112.000,E,0.0").unwrap();
        assert_eq!(rmc.lat_e7, -338_500_000);
        assert_eq!(rmc.lon_e7, 1_512_000_000);
    }

    #[test]
    fn rmc_no_fix_leaves_defaults() {
        let rmc = build_rmc_struct(b",V,,,,,,,,,").unwrap();
        assert_eq!(rmc.status, RMCStatus::V);
        assert_eq!(rmc.lat_e7, 0);
        assert_eq!(rmc.lon_e7, 0);
        assert_eq!(rmc.sog_mms, 0);
    }

    #[test]
    fn rmc_unknown_status_is_void() {
        let rmc = build_rmc_struct(b",X,").unwrap();
        assert_eq!(rmc.status, RMCStatus::V);
    }

    #[test]
    fn rmc_invalid_hemisphere_is_error() {
        let r = build_rmc_struct(b",A,4807.038,Q,01131.000,E,0.0");
        assert!(matches!(r, Err(LineError::InvalidValue)));
        let r = build_rmc_struct(b",A,4807.038,N,01131.000,Q,0.0");
        assert!(matches!(r, Err(LineError::InvalidValue)));
    }

    #[test]
    fn rmc_bad_coord_or_speed_is_error() {
        assert!(build_rmc_struct(b",A,48a7.038,N,01131.000,E,0.0").is_err());
        assert!(build_rmc_struct(b",A,4807.038,N,01131.000,E,fast").is_err());
    }

    #[test]
    fn utc_known_values() {
        let cases: &[(&[u8], u32)] = &[
            (b"000000", 0),
            (b"000000.00", 0),
            (b"123519.00", 45_319_000), // 12:35:19
            (b"123519", 45_319_000),    // no fraction
            (b"123519.5", 45_319_500),
            (b"123519.123", 45_319_123),
            (b"123519.1234", 45_319_123), // extra digits truncated
            (b"235959.999", 86_399_999),
            (b"235960", 86_400_000), // leap second
        ];
        for &(input, expected) in cases {
            assert_eq!(
                parse_utc_ms(input).unwrap(),
                expected,
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn utc_rejects_bad_input() {
        let bad: &[&[u8]] = &[
            b"12351",     // too short
            b"1235190",   // too long before the dot
            b"243000",    // hour 24
            b"126000",    // minute 60
            b"123561",    // second 61
            b"12ab19.00", // non-digit
            b"123519.0x", // garbage in fraction
            b"",
        ];
        for &input in bad {
            assert!(
                matches!(parse_utc_ms(input), Err(ParseError::InvalidTime)),
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    // ---------- parse_date ----------

    #[test]
    fn date_known_values() {
        // Year is the raw two-digit value; update these when you add a century pivot.
        let cases: &[(&[u8], (u8, u8, u16))] = &[
            (b"230394", (23, 3, 94)),
            (b"010100", (1, 1, 0)),
            (b"311299", (31, 12, 99)),
        ];
        for &(input, expected) in cases {
            assert_eq!(
                parse_date(input).unwrap(),
                expected,
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn date_rejects_malformed() {
        let bad: &[&[u8]] = &[b"23039", b"2303944", b"", b"23a394", b"23-394"];
        for &input in bad {
            assert!(
                matches!(parse_date(input), Err(ParseError::InvalidDate)),
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    // ---------- Date::new ----------

    #[test]
    fn date_new_accepts_valid() {
        let d = Date::new(31, 12, 99).unwrap();
        assert_eq!((d.day, d.month, d.year), (31, 12, 99));
        assert!(Date::new(1, 1, 0).is_ok());
    }

    #[test]
    fn date_new_rejects_out_of_range() {
        for (day, month) in [(0, 1), (32, 1), (1, 0), (1, 13), (0, 0)] {
            assert!(
                matches!(Date::new(day, month, 24), Err(ParseError::InvalidDate)),
                "day {day} month {month}"
            );
        }
    }

    // ---------- parse_decimal_to_int ----------

    #[test]
    fn fixed_known_values() {
        let cases: &[(&[u8], u32, u32)] = &[
            (b"0.0", 3, 0),
            (b"084.4", 3, 84_400),
            (b"359.999", 3, 359_999),
            (b"5", 3, 5_000),
            (b".5", 3, 500),
            (b"1.23456", 3, 1_234), // extra digits truncated
            (b"12.3", 2, 1_230),
            (b"12.34", 2, 1_234),
        ];
        for &(input, precision, expected) in cases {
            assert_eq!(
                parse_decimal_to_int(input, precision).unwrap(),
                expected,
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn fixed_rejects_bad_input() {
        // Only is_err(): the helper currently reports InvalidSpeed even for course.
        let bad: &[&[u8]] = &[b"abc", b"1.2x", b"1.2.3", b"-1.0", b"1,5"];
        for &input in bad {
            assert!(
                parse_decimal_to_int(input, 3).is_err(),
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn fixed_int_part_too_big_for_u32() {
        assert!(parse_decimal_to_int(b"4294967296.0", 3).is_err());
    }

    // ---------- build_rmc_struct ----------

    #[test]
    fn rmc_full_sentence_every_field() {
        let rmc =
            build_rmc_struct(b"123519.00,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W")
                .unwrap();
        assert_eq!(rmc.utc_ms, 45_319_000);
        assert_eq!(rmc.status, RMCStatus::A);
        assert_eq!(rmc.lat_e7, 481_173_000);
        assert_eq!(rmc.lon_e7, 115_166_667);
        assert_eq!(rmc.sog_mms, 11_524);
        assert_eq!(rmc.angle_mdeg, 84_400);
        assert_eq!((rmc.date.day, rmc.date.month, rmc.date.year), (23, 3, 94));
    }

    #[test]
    fn rmc_empty_course_does_not_shift_columns() {
        let rmc = build_rmc_struct(b",A,4807.038,N,01131.000,E,022.4,,230394").unwrap();
        assert_eq!(rmc.angle_mdeg, 0);
        assert_eq!(rmc.sog_mms, 11_524);
        assert_eq!((rmc.date.day, rmc.date.month), (23, 3));
    }

    #[test]
    fn rmc_course_alone() {
        let rmc = build_rmc_struct(b",A,,,,,,084.4").unwrap();
        assert_eq!(rmc.angle_mdeg, 84_400);
    }

    #[test]
    fn rmc_date_alone() {
        let rmc = build_rmc_struct(b",,,,,,,,230394").unwrap();
        assert_eq!((rmc.date.day, rmc.date.month, rmc.date.year), (23, 3, 94));
    }

    #[test]
    fn rmc_no_date_leaves_default_date() {
        let rmc = build_rmc_struct(b",A,4807.038,N,01131.000,E,022.4,084.4").unwrap();
        assert_eq!((rmc.date.day, rmc.date.month, rmc.date.year), (0, 0, 0));
    }

    #[test]
    fn rmc_bad_date_is_error() {
        let bad: &[&[u8]] = &[
            b",,,,,,,,000194", // day 0
            b",,,,,,,,230094", // month 0
            b",,,,,,,,231394", // month 13
            b",,,,,,,,320194", // day 32
            b",,,,,,,,2303",   // wrong length
            b",,,,,,,,23a394", // non-digit
        ];
        for &input in bad {
            assert!(
                build_rmc_struct(input).is_err(),
                "input {:?}",
                core::str::from_utf8(input)
            );
        }
    }

    #[test]
    fn rmc_bad_time_is_error() {
        assert!(build_rmc_struct(b"246000,A").is_err());
        assert!(build_rmc_struct(b"12ab19.00,A").is_err());
    }

    #[test]
    fn fixed_overflow_is_an_error_not_a_panic() {
        assert!(parse_decimal_to_int(b"99999999.0", 3).is_err());
    }

    #[test]
    fn rmc_course_out_of_range_is_error() {
        assert!(build_rmc_struct(b",A,,,,,,360.5").is_err());
        assert!(build_rmc_struct(b",A,,,,,,999.9").is_err());
    }
}
