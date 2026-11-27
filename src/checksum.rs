use crate::error::ParseError;

/// Check the checksum of a line like `$GPRMC,...*6A`.
/// On success, returns the payload: the bytes between `$` and `*`.
pub fn verify_checksum(line: &[u8]) -> Result<&[u8], ParseError> {
    // Strip the leading '$'. Returns None if it isn't there.
    let rest = line.strip_prefix(b"$").ok_or(ParseError::MissingDollar)?;

    // Iterate over the rest until you find the position of the *
    let star = rest
        .iter()
        .position(|&b| b == b'*')
        .ok_or(ParseError::MissingChecksum)?;

    // Split into the payload and the tail, which starts with '*'.
    let (payload, tail) = rest.split_at(star);

    // The tail must be exactly: '*', hi digit, lo digit.
    let [b'*', hi, lo] = tail else {
        return Err(ParseError::BadChecksumHex);
    };

    // get hex value for each nibble
    let hi_val = hex_val(*hi).ok_or(ParseError::BadChecksumHex)?;
    let lo_val = hex_val(*lo).ok_or(ParseError::BadChecksumHex)?;

    // bitshift hi_val, XOR with lo_val to get single byte checksum
    let checksum = (hi_val << 4) | lo_val;
    let sum = payload.iter().fold(0, |acc, &b| acc ^ b);

    if sum != checksum {
        Err(ParseError::ChecksumMismatch)
    } else {
        Ok(payload)
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod checksum_tests {
    use super::*;

    const GGA: &[u8] = b"$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
    const RMC: &[u8] = b"$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6A";

    #[test]
    fn valid_gga_returns_payload() {
        let payload = verify_checksum(GGA).unwrap();
        assert_eq!(
            payload,
            &b"GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,"[..]
        );
    }

    #[test]
    fn valid_rmc_is_accepted() {
        assert!(verify_checksum(RMC).is_ok());
    }

    #[test]
    fn wrong_checksum_is_rejected() {
        let bad = b"$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6B";
        assert_eq!(verify_checksum(bad), Err(ParseError::ChecksumMismatch));
    }

    #[test]
    fn corrupted_payload_is_rejected() {
        let bad = b"$GPRMC,123519,A,4807.038,N,01131.000,E,022.5,084.4,230394,003.1,W*6A";
        assert_eq!(verify_checksum(bad), Err(ParseError::ChecksumMismatch));
    }

    #[test]
    fn missing_dollar() {
        assert_eq!(
            verify_checksum(b"GPRMC,1,2*00"),
            Err(ParseError::MissingDollar)
        );
    }

    #[test]
    fn missing_star() {
        assert_eq!(
            verify_checksum(b"$GPRMC,1,2"),
            Err(ParseError::MissingChecksum)
        );
    }

    #[test]
    fn non_hex_checksum() {
        assert_eq!(
            verify_checksum(b"$GPRMC,1,2*ZZ"),
            Err(ParseError::BadChecksumHex)
        );
    }

    #[test]
    fn truncated_checksum() {
        assert_eq!(
            verify_checksum(b"$GPRMC,1,2*4"),
            Err(ParseError::BadChecksumHex)
        );
    }

    #[test]
    fn short_inputs_do_not_panic() {
        for input in [&b""[..], b"$", b"*", b"$*", b"$*4", b"$*47", b"*47"] {
            let _ = verify_checksum(input); // just must not panic
        }
    }
}
