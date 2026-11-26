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
