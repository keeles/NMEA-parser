use crate::{
    error::LineError,
    parse::{parse_coord, parse_decimal_to_int, parse_decimal_to_uint, parse_u32, parse_utc_ms},
};

#[derive(Debug, Default)]
pub struct GGA {
    utc_ms: u32,
    lat_e7: i32,
    lon_e7: i32,
    gps_qual: u32,
    sv_in_use: u32,
    hdop_x100: u32,
    ortho_h_m_x100: i32,
    geoid_sep_m_x100: i32,
    age_of_dif_sep: u32,
    ref_stat_id: u32,
}

impl GGA {
    pub fn new() -> GGA {
        GGA {
            utc_ms: 0,
            lat_e7: 0,
            lon_e7: 0,
            gps_qual: 3,
            sv_in_use: 0,
            hdop_x100: 0,
            ortho_h_m_x100: 0,
            geoid_sep_m_x100: 0,
            age_of_dif_sep: 0,
            ref_stat_id: 0,
        }
    }
}

pub fn build_gga_struct(line: &[u8]) -> Result<GGA, LineError> {
    let mut gga_data = GGA::default();

    for (col, field) in line.split(|&b| b == b',').enumerate() {
        if field.is_empty() {
            continue;
        }

        match col {
            0 => {
                gga_data.utc_ms = parse_utc_ms(field)?;
            }
            1 => {
                gga_data.lat_e7 = parse_coord(field)?;
            }
            2 => match field.first() {
                Some(b'N') => {}
                Some(b'S') => gga_data.lat_e7 = -gga_data.lat_e7,
                None => {} // empty when there's no fix
                _ => return Err(LineError::InvalidValue),
            },
            3 => {
                gga_data.lon_e7 = parse_coord(field)?;
            }
            4 => match field.first() {
                Some(b'E') => {}
                Some(b'W') => gga_data.lon_e7 = -gga_data.lon_e7,
                None => {}
                _ => return Err(LineError::InvalidValue),
            },
            5 => {
                gga_data.gps_qual = parse_u32(field).ok_or(LineError::InvalidValue)?;
            }
            6 => {
                gga_data.sv_in_use = parse_u32(field).ok_or(LineError::InvalidValue)?;
            }
            7 => {
                gga_data.hdop_x100 = parse_decimal_to_uint(field, 2)?;
            }
            8 => {
                gga_data.ortho_h_m_x100 = parse_decimal_to_int(field, 2)?;
            }
            9 => {}
            10 => {
                gga_data.geoid_sep_m_x100 = parse_decimal_to_int(field, 2)?;
            }
            11 => {}
            12 => {
                // TODO: age of diff gps data record
            }
            13 => {
                // TODO: ref station id
            }
            _ => continue,
        }
    }

    Ok(gga_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fields after the sentence ID, i.e. what follows "$GPGGA,"
    const GOOD: &[u8] = b"123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,";

    fn near(actual: i32, expected: i32) {
        assert!(
            (actual - expected).abs() <= 1,
            "expected ~{expected}, got {actual}"
        );
    }

    #[test]
    fn parses_canonical_sentence() {
        let g = build_gga_struct(GOOD).unwrap();
        assert_eq!(g.utc_ms, 45_319_000);
        near(g.lat_e7, 481_173_000);
        near(g.lon_e7, 115_166_667);
        assert_eq!(g.gps_qual, 1);
        assert_eq!(g.sv_in_use, 8);
        assert_eq!(g.hdop_x100, 90);
        assert_eq!(g.ortho_h_m_x100, 54_540);
        assert_eq!(g.geoid_sep_m_x100, 4_690);
    }

    #[test]
    fn southern_and_western_hemispheres_are_negative() {
        let g =
            build_gga_struct(b"123519,4807.038,S,01131.000,W,1,08,0.9,545.4,M,46.9,M,,").unwrap();
        near(g.lat_e7, -481_173_000);
        near(g.lon_e7, -115_166_667);
    }

    #[test]
    fn fractional_seconds_in_time() {
        let g = build_gga_struct(b"123519.500,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,")
            .unwrap();
        assert_eq!(g.utc_ms, 45_319_500);
    }

    #[test]
    fn no_fix_sentence_parses_with_zeroed_fields() {
        let g = build_gga_struct(b"123519,,,,,0,00,,,M,,M,,").unwrap();
        assert_eq!(g.utc_ms, 45_319_000);
        assert_eq!(g.gps_qual, 0);
        assert_eq!(g.sv_in_use, 0);
        // These asserts will change if you later move lat/lon to Option:
        assert_eq!(g.lat_e7, 0);
        assert_eq!(g.lon_e7, 0);
    }

    #[test]
    fn fully_empty_fields_do_not_error() {
        assert!(build_gga_struct(b",,,,,,,,,,,,,").is_ok());
    }

    #[test]
    fn negative_geoid_separation() {
        let g =
            build_gga_struct(b"123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,-34.0,M,,").unwrap();
        assert_eq!(g.geoid_sep_m_x100, -3_400);
    }

    #[test]
    fn poor_signal_hdop() {
        let g =
            build_gga_struct(b"123519,4807.038,N,01131.000,E,1,03,99.99,545.4,M,46.9,M,,").unwrap();
        assert_eq!(g.hdop_x100, 9_999);
    }

    #[test]
    fn invalid_hemisphere_is_an_error() {
        let r = build_gga_struct(b"123519,4807.038,X,01131.000,E,1,08,0.9,545.4,M,46.9,M,,");
        assert!(matches!(r, Err(LineError::InvalidValue)));
    }

    #[test]
    fn non_numeric_quality_is_an_error() {
        let r = build_gga_struct(b"123519,4807.038,N,01131.000,E,x,08,0.9,545.4,M,46.9,M,,");
        assert!(matches!(r, Err(LineError::InvalidValue)));
    }

    #[test]
    fn non_numeric_satellite_count_is_an_error() {
        let r = build_gga_struct(b"123519,4807.038,N,01131.000,E,1,xx,0.9,545.4,M,46.9,M,,");
        assert!(matches!(r, Err(LineError::InvalidValue)));
    }

    #[test]
    fn extra_trailing_fields_are_ignored() {
        let line = [GOOD, &b",extra,fields"[..]].concat();
        assert!(build_gga_struct(&line).is_ok());
    }

    #[test]
    fn new_matches_default() {
        // Fails until GGA::new() and GGA::default() agree (see problem 3).
        assert_eq!(GGA::new().gps_qual, GGA::default().gps_qual);
    }

    // ---- robustness: these only assert "does not panic" ----

    #[test]
    fn truncated_input_never_panics() {
        for end in 0..=GOOD.len() {
            let _ = build_gga_struct(&GOOD[..end]);
        }
    }

    #[test]
    fn garbage_never_panics() {
        for input in [
            &b""[..],
            b",",
            b",,,,",
            b"\xff\xfe,\x00,,",
            b"-,-,-,-,-,-,-,-,-,-,-,-",
            b"1234567890123456789012345678901234567890",
        ] {
            let _ = build_gga_struct(input);
        }
    }

    #[test]
    fn overflowing_numbers_do_not_panic() {
        // Debug builds panic on integer overflow, so this catches unchecked math.
        let big: &[u8] = b"99999999999999999999";
        let empty: &[u8] = b"";
        for col in 0..14 {
            let mut fields: Vec<&[u8]> = vec![empty; 14];
            fields[col] = big;
            let line = fields.join(&b',');
            let _ = build_gga_struct(&line);
        }
    }

    #[test]
    fn negative_altitude_near_sea_level() {
        let g =
            build_gga_struct(b"123519,4807.038,N,01131.000,E,1,08,0.9,-2.1,M,46.9,M,,").unwrap();
        assert_eq!(g.ortho_h_m_x100, -210);
    }
}
