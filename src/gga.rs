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
    ortho_h_m_x100: u32,
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
                gga_data.ortho_h_m_x100 = parse_decimal_to_uint(field, 2)?;
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
