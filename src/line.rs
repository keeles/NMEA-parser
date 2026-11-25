/// NMEA 0183 caps a sentence at 82 bytes including `$` and CR LF.
pub const MAX_LINE: usize = 82;

/// Collects bytes from a stream and hands back complete lines.
pub struct LineAssembler {
    buf: [u8; MAX_LINE],
    len: usize,
    collecting: bool,
}

impl LineAssembler {
    pub const fn new() -> Self {
        let buf: [u8; MAX_LINE] = [0; MAX_LINE];
        let len = 0;
        LineAssembler {
            buf,
            len,
            collecting: false,
        }
    }

    pub fn push_byte(&mut self, byte: u8) -> Option<&[u8]> {
        match byte {
            b'$' => {
                self.len = 0;
                self.buf[self.len] = byte;
                self.collecting = true;
                self.len += 1;
            }
            b'\n' => {
                if self.collecting {
                    self.collecting = false;
                    return Some(&self.buf[..self.len]);
                }
            }
            b'\r' => {
                // ignore it
            }
            _ => {
                if self.collecting {
                    if self.len < MAX_LINE {
                        self.buf[self.len] = byte;
                        self.len += 1;
                    } else {
                        self.collecting = false;
                    }
                }
            }
        }

        None // most calls return None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Push every byte; collect the lines that come out as Strings.
    fn feed(asm: &mut LineAssembler, bytes: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        for &b in bytes {
            if let Some(line) = asm.push_byte(b) {
                out.push(String::from_utf8_lossy(line).into_owned());
            }
        }
        out
    }

    #[test]
    fn complete_line_with_crlf() {
        let mut asm = LineAssembler::new();
        let lines = feed(&mut asm, b"$GPRMC,1,2*00\r\n");
        assert_eq!(lines, vec!["$GPRMC,1,2*00"]);
    }

    #[test]
    fn complete_line_with_lf_only() {
        let mut asm = LineAssembler::new();
        let lines = feed(&mut asm, b"$GPRMC,1,2*00\n");
        assert_eq!(lines, vec!["$GPRMC,1,2*00"]);
    }

    #[test]
    fn nothing_returned_until_line_ends() {
        let mut asm = LineAssembler::new();
        assert!(feed(&mut asm, b"$GPRMC,1,2*00").is_empty());
        assert_eq!(feed(&mut asm, b"\r\n"), vec!["$GPRMC,1,2*00"]);
    }

    #[test]
    fn two_lines_back_to_back() {
        let mut asm = LineAssembler::new();
        let lines = feed(&mut asm, b"$AAA,1*00\r\n$BBB,2*00\r\n");
        assert_eq!(lines, vec!["$AAA,1*00", "$BBB,2*00"]);
    }

    #[test]
    fn chunking_does_not_matter() {
        let data = b"$AAA,1*00\r\n$BBB,2*00\r\n";
        for chunk_size in 1..=data.len() {
            let mut asm = LineAssembler::new();
            let mut lines = Vec::new();
            for chunk in data.chunks(chunk_size) {
                lines.extend(feed(&mut asm, chunk));
            }
            assert_eq!(
                lines,
                vec!["$AAA,1*00", "$BBB,2*00"],
                "chunk size {chunk_size}"
            );
        }
    }

    #[test]
    fn garbage_before_dollar_is_ignored() {
        let mut asm = LineAssembler::new();
        let lines = feed(&mut asm, b"xyz\x00\xff,12$AAA,1*00\r\n");
        assert_eq!(lines, vec!["$AAA,1*00"]);
    }

    #[test]
    fn filler_newlines_produce_no_empty_lines() {
        let mut asm = LineAssembler::new();
        let lines = feed(&mut asm, b"\n\n\r\n$AAA,1*00\r\n\n\n");
        assert_eq!(lines, vec!["$AAA,1*00"]);
    }

    #[test]
    fn new_dollar_restarts_a_cut_off_line() {
        let mut asm = LineAssembler::new();
        let lines = feed(&mut asm, b"$AAA,1,2$BBB,3*00\r\n");
        assert_eq!(lines, vec!["$BBB,3*00"]);
    }

    #[test]
    fn overlong_line_is_dropped_and_assembler_recovers() {
        let mut asm = LineAssembler::new();
        let mut data = vec![b'$'];
        data.extend(std::iter::repeat(b'A').take(MAX_LINE * 2));
        data.extend_from_slice(b"\r\n$BBB,3*00\r\n");
        let lines = feed(&mut asm, &data);
        assert_eq!(lines, vec!["$BBB,3*00"]);
    }
}
