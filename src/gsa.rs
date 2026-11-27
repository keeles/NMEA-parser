use crate::{
    error::LineError,
    parse::{parse_decimal_to_uint, parse_u32},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GSAMode {
    #[default]
    M,
    A,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FixType {
    #[default]
    NA,
    TwoDim,
    ThreeDim,
}

#[derive(Debug, Default)]
pub struct GSA {
    mode: GSAMode,
    fix_type: FixType,
    prn_num: u32,
    pdop_x100: u32,
    hdop_x100: u32,
    vdop_x100: u32,
}

impl GSA {
    pub fn new() -> GSA {
        GSA {
            mode: GSAMode::M,
            fix_type: FixType::NA,
            prn_num: 0,
            pdop_x100: 0,
            hdop_x100: 0,
            vdop_x100: 0,
        }
    }
}

pub fn build_gsa_struct(line: &[u8]) -> Result<GSA, LineError> {
    let mut gsa_data = GSA::default();

    for (col, field) in line.split(|&b| b == b',').enumerate() {
        if field.is_empty() {
            continue;
        }

        match col {
            0 => {
                gsa_data.mode = if field.first() == Some(&b'A') {
                    GSAMode::A
                } else {
                    GSAMode::M
                };
            }
            1 => {
                gsa_data.fix_type = if field.first() == Some(&b'2') {
                    FixType::TwoDim
                } else if field.first() == Some(&b'3') {
                    FixType::ThreeDim
                } else {
                    FixType::NA
                }
            }
            2 => {
                gsa_data.prn_num = parse_u32(field).ok_or(LineError::InvalidValue)?;
            }
            3 => {
                gsa_data.pdop_x100 = parse_decimal_to_uint(field, 2)?;
            }
            4 => {
                gsa_data.hdop_x100 = parse_decimal_to_uint(field, 2)?;
            }
            5 => {
                gsa_data.vdop_x100 = parse_decimal_to_uint(field, 2)?;
            }
            _ => continue,
        }
    }

    Ok(gsa_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fields after the sentence ID, i.e. what follows "$GPGSA,"
    const GOOD: &[u8] = b"A,3,04,05,,09,12,,,24,,,,,2.5,1.3,2.1";

    /// Builds "mode,fix,<12 satellite fields>,pdop,hdop,vdop".
    fn gsa(mode: &str, fix: &str, prns: &[&str], pdop: &str, hdop: &str, vdop: &str) -> Vec<u8> {
        let mut fields: Vec<&str> = vec![mode, fix];
        for i in 0..12 {
            fields.push(prns.get(i).copied().unwrap_or(""));
        }
        fields.extend([pdop, hdop, vdop]);
        fields.join(",").into_bytes()
    }

    #[test]
    fn parses_canonical_sentence() {
        // Fails until DOP columns are 14/15/16 (see problem 1).
        let g = build_gsa_struct(GOOD).unwrap();
        assert_eq!(g.mode, GSAMode::A);
        assert_eq!(g.fix_type, FixType::ThreeDim);
        assert_eq!(g.pdop_x100, 250);
        assert_eq!(g.hdop_x100, 130);
        assert_eq!(g.vdop_x100, 210);
    }

    #[test]
    fn manual_mode_and_2d_fix() {
        let line = gsa("M", "2", &["04", "05", "06"], "3.2", "2.0", "2.5");
        let g = build_gsa_struct(&line).unwrap();
        assert_eq!(g.mode, GSAMode::M);
        assert_eq!(g.fix_type, FixType::TwoDim);
        assert_eq!(g.pdop_x100, 320);
        assert_eq!(g.hdop_x100, 200);
        assert_eq!(g.vdop_x100, 250);
    }

    #[test]
    fn no_fix_reports_na_and_99_99() {
        let line = gsa("A", "1", &[], "99.99", "99.99", "99.99");
        let g = build_gsa_struct(&line).unwrap();
        assert_eq!(g.fix_type, FixType::NA);
        assert_eq!(g.pdop_x100, 9_999);
        assert_eq!(g.hdop_x100, 9_999);
        assert_eq!(g.vdop_x100, 9_999);
    }

    #[test]
    fn empty_dops_do_not_error() {
        let line = gsa("A", "1", &[], "", "", "");
        let g = build_gsa_struct(&line).unwrap();
        assert_eq!(g.pdop_x100, 0);
        assert_eq!(g.hdop_x100, 0);
        assert_eq!(g.vdop_x100, 0);
    }

    #[test]
    fn twelve_satellites_do_not_leak_into_dops() {
        let prns = [
            "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12",
        ];
        let line = gsa("A", "3", &prns, "1.0", "0.8", "0.6");
        let g = build_gsa_struct(&line).unwrap();
        assert_eq!(g.pdop_x100, 100);
        assert_eq!(g.hdop_x100, 80);
        assert_eq!(g.vdop_x100, 60);
    }

    #[test]
    fn trailing_system_id_is_ignored() {
        let mut line = gsa("A", "3", &["04", "05"], "2.5", "1.3", "2.1");
        line.extend_from_slice(b",1");
        let g = build_gsa_struct(&line).unwrap();
        assert_eq!(g.vdop_x100, 210);
    }

    #[test]
    fn unknown_fix_type_maps_to_na() {
        let line = gsa("A", "9", &[], "1.0", "1.0", "1.0");
        let g = build_gsa_struct(&line).unwrap();
        assert_eq!(g.fix_type, FixType::NA);
    }

    #[test]
    fn invalid_dop_is_an_error() {
        let line = gsa("A", "3", &["04"], "x.y", "1.3", "2.1");
        assert!(build_gsa_struct(&line).is_err());
    }

    // ---- robustness: these only assert "does not panic" ----

    #[test]
    fn truncated_input_never_panics() {
        for end in 0..=GOOD.len() {
            let _ = build_gsa_struct(&GOOD[..end]);
        }
    }

    #[test]
    fn garbage_never_panics() {
        for input in [
            &b""[..],
            b",",
            b",,,,",
            b"\xff\xfe,\x00,,",
            b"-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-",
        ] {
            let _ = build_gsa_struct(input);
        }
    }

    #[test]
    fn overflowing_numbers_do_not_panic() {
        let big: &[u8] = b"99999999999999999999";
        let empty: &[u8] = b"";
        for col in 0..18 {
            let mut fields: Vec<&[u8]> = vec![empty; 18];
            fields[col] = big;
            let line = fields.join(&b',');
            let _ = build_gsa_struct(&line);
        }
    }
}
