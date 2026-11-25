use nmea::{LineAssembler, verify_checksum};

pub fn main() {
    let raw_data: &[u8] = include_bytes!("../tests/data/sample.nmea");

    let mut asm = LineAssembler::new();

    for raw_line in raw_data.split_inclusive(|&b| b == b'\n') {
        for &b in raw_line {
            if let Some(line) = asm.push_byte(b) {
                let checksum = verify_checksum(line).unwrap();
                let s: String = String::from_utf8_lossy(checksum).into_owned();
                println!("{s}");
            }
        }
    }
}
