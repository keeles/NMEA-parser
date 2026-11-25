use nmea::{LineAssembler, verify_checksum};

const RMC: &[u8] = b"$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6A";

#[test]
fn assembler_output_passes_checksum() {
    let mut asm = LineAssembler::new();
    let mut stream = Vec::from(RMC);
    stream.extend_from_slice(b"\r\n");
    let mut results = Vec::new();
    for &b in &stream {
        if let Some(line) = asm.push_byte(b) {
            results.push(verify_checksum(line).is_ok());
        }
    }
    assert_eq!(results, vec![true]);
}
