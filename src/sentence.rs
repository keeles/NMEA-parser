use crate::{
    LineAssembler, error::LineError, gga::GGA, gsa::GSA, rmc::RMC, sentence, verify_checksum,
};

pub enum Sentence {
    RMC(RMC),
    GGA(GGA),
    GSA(GSA),
    Unknown,
}

pub fn parse_sentence(payload: &[u8], mut asm: LineAssembler) -> Result<Sentence, LineError> {
    for &b in payload {
        if let Some(line) = asm.push_byte(b) {
            let checksum = verify_checksum(line).map_err(|_| LineError::InvalidValue)?;
            if let Some(prefix) = checksum.split(|&b| b == b',').next() {
                if prefix.len() != 5 {
                    return Err(LineError::InvalidValue);
                }

                match prefix.get(2..) {
                    Some(b"RMC") => Ok(Sentence::RMC(RMC::default())),
                    Some(b"GGA") => Ok(Sentence::GGA(GGA::default())),
                    Some(b"GSA") => Ok(Sentence::GSA(GSA::default())),
                    None => Err(LineError::InvalidValue),
                    _ => Ok(Sentence::Unknown),
                };
            }
        }
    }

    Ok(Sentence::Unknown)
}
