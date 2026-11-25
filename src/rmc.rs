use crate::{MAX_LINE, ParseError, error::LineError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RMCStatus {
    A,
    #[default]
    V,
}

#[derive(Debug, Default)]
pub struct RMC {
    utc_ms: u32,
    status: RMCStatus,
    lat: i32,
    lon: i32,
    sog: u32,
    angle: u32,
    date: [u32; 6],
    mag_var: u32,
}

impl RMC {
    pub fn new() -> RMC {
        RMC {
            utc_ms: 0,
            status: RMCStatus::V,
            lat: 0,
            lon: 0,
            sog: 0,
            angle: 0,
            date: [0; 6],
            mag_var: 0,
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
                // store lat
                rmc_data.lat = parse_coord(field)?;
            }
            3 => match field.first() {
                Some(b'N') => {}
                Some(b'S') => rmc_data.lat = -rmc_data.lat,
                None => {} // empty when there's no fix
                _ => return Err(LineError::InvalidValue),
            },
            4 => {
                // store lon
                rmc_data.lon = parse_coord(field)?;
            }
            5 => match field.first() {
                Some(b'E') => {}
                Some(b'W') => rmc_data.lon = -rmc_data.lon,
                None => {}
                _ => return Err(LineError::InvalidValue),
            },
            6 => {
                // parse speed over ground
                rmc_data.sog = parse_speed_mms(field)?;
            }
            7 => {
                // TODO: parse course
            }
            8 => {
                // TODO: parse date
            }
            9 => {
                // TODO: parse magnetic variation
            }
            _ => continue,
        }
    }

    Ok(rmc_data)
}

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

fn parse_u64(bytes: &[u8]) -> Option<u64> {
    bytes.iter().try_fold(0u64, |acc, &b| {
        if !b.is_ascii_digit() {
            return None;
        }
        acc.checked_mul(10)?.checked_add((b - b'0') as u64)
    })
}

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
        // Needs the checked-arithmetic fix below; panics in debug builds without it.
        assert!(matches!(
            parse_speed_mms(b"99999999999999.0"),
            Err(ParseError::InvalidSpeed)
        ));
    }

    // ---------- build_rmc_struct ----------
    // The time field is left empty in these: a non-empty one currently hits
    // to_u32_array, which panics (see the ignored test at the bottom).

    #[test]
    fn rmc_full_sentence_north_east() {
        let rmc =
            build_rmc_struct(b",A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W").unwrap();
        assert_eq!(rmc.status, RMCStatus::A);
        assert_eq!(rmc.lat, 481_173_000);
        assert_eq!(rmc.lon, 115_166_667);
        assert_eq!(rmc.sog, 11_524);
    }

    #[test]
    fn rmc_south_west_are_negative() {
        let rmc = build_rmc_struct(b",A,3351.000,S,15112.000,W,000.0").unwrap();
        assert_eq!(rmc.lat, -338_500_000);
        assert_eq!(rmc.lon, -1_512_000_000);
        assert_eq!(rmc.sog, 0);
    }

    #[test]
    fn rmc_south_east_mixed() {
        let rmc = build_rmc_struct(b",A,3351.000,S,15112.000,E,0.0").unwrap();
        assert_eq!(rmc.lat, -338_500_000);
        assert_eq!(rmc.lon, 1_512_000_000);
    }

    #[test]
    fn rmc_no_fix_leaves_defaults() {
        let rmc = build_rmc_struct(b",V,,,,,,,,,").unwrap();
        assert_eq!(rmc.status, RMCStatus::V);
        assert_eq!(rmc.lat, 0);
        assert_eq!(rmc.lon, 0);
        assert_eq!(rmc.sog, 0);
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
    fn rmc_with_real_time_field() {
        let r = build_rmc_struct(b"123519.00,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W");
        assert!(r.is_ok());
    }
}
