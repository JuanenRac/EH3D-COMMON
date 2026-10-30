//! The line transport of a real serial port (or a USB serial adapter), for GRBL and the controllers that talk lines of text.
//!
//! The bytes that come in are cut into lines by [`LineBuffer`], which has no port in it and is tested on its own; [`SerialTransport`] is the
//! port itself and needs the `serial` feature.

use std::collections::VecDeque;

/// Cuts a stream of bytes into lines: a line ends at `\n`, a `\r` before it is dropped, an empty line is not a line, and the half of a line
/// that has not ended yet waits for the rest. A line that grows past the limit without ending is dropped, so noise on a wrong baud rate
/// cannot fill the memory.
#[derive(Debug, Default)]
pub struct LineBuffer {
    partial: Vec<u8>,
    lines: VecDeque<String>,
}

/// The longest line that is kept (a status report of GRBL is far shorter).
pub const MAX_LINE: usize = 512;

impl LineBuffer {
    pub fn push(&mut self, bytes: &[u8]) {
        for &b in bytes {
            match b {
                b'\n' => {
                    if self.partial.last() == Some(&b'\r') {
                        self.partial.pop();
                    }
                    if !self.partial.is_empty() {
                        let line = String::from_utf8_lossy(&self.partial).into_owned();
                        self.lines.push_back(line);
                    }
                    self.partial.clear();
                }
                _ => {
                    if self.partial.len() < MAX_LINE {
                        self.partial.push(b);
                    } else {
                        // too long to be a line of the controller: noise
                        self.partial.clear();
                    }
                }
            }
        }
    }

    pub fn pop(&mut self) -> Option<String> {
        self.lines.pop_front()
    }
}

#[cfg(feature = "serial")]
pub use port::SerialTransport;

#[cfg(feature = "serial")]
mod port {
    use std::io::{self, Read, Write};
    use std::time::{Duration, Instant};

    use super::LineBuffer;
    use crate::grbl::LineTransport;

    /// A serial port opened for a controller: 8N1 without flow control, the line settings of GRBL.
    pub struct SerialTransport {
        port: Box<dyn serialport::SerialPort>,
        buffer: LineBuffer,
    }

    impl SerialTransport {
        /// Opens `name` (`COM3`, `/dev/ttyUSB0`) at `baud` (115200 for GRBL 1.1).
        pub fn open(name: &str, baud: u32) -> io::Result<SerialTransport> {
            let port = serialport::new(name, baud)
                .timeout(Duration::from_millis(20))
                .open()
                .map_err(|e| io::Error::other(e.to_string()))?;
            Ok(SerialTransport {
                port,
                buffer: LineBuffer::default(),
            })
        }

        /// The serial ports of this computer, by name.
        pub fn available() -> Vec<String> {
            serialport::available_ports()
                .map(|p| p.into_iter().map(|i| i.port_name).collect())
                .unwrap_or_default()
        }
    }

    impl LineTransport for SerialTransport {
        fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.port.write_all(bytes)?;
            self.port.flush()
        }

        fn read_line(&mut self, timeout: Duration) -> io::Result<Option<String>> {
            let end = Instant::now() + timeout;
            let mut chunk = [0u8; 256];
            loop {
                if let Some(line) = self.buffer.pop() {
                    return Ok(Some(line));
                }
                match self.port.read(&mut chunk) {
                    Ok(n) if n > 0 => self.buffer.push(&chunk[..n]),
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::TimedOut => {}
                    Err(e) => return Err(e),
                }
                if Instant::now() >= end {
                    return Ok(self.buffer.pop());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_cut_at_the_line_end_and_a_half_line_waits() {
        let mut b = LineBuffer::default();
        b.push(b"ok\r\n<Idle|MPos:0.000,0.0");
        assert_eq!(b.pop().as_deref(), Some("ok"));
        assert_eq!(b.pop(), None);
        b.push(b"00,0.000|FS:0,0>\r\n\r\n\nerror:20\n");
        assert_eq!(
            b.pop().as_deref(),
            Some("<Idle|MPos:0.000,0.000,0.000|FS:0,0>")
        );
        assert_eq!(b.pop().as_deref(), Some("error:20"));
        assert_eq!(b.pop(), None);
    }

    #[test]
    fn noise_without_a_line_end_cannot_fill_the_memory() {
        let mut b = LineBuffer::default();
        for _ in 0..100 {
            b.push(&[0xFFu8; 100]);
        }
        assert!(b.partial.len() <= MAX_LINE);
        b.push(b"ok\n");
        // the noise was dropped; what follows is read from the start
        assert!(b.pop().is_some());
    }

    #[test]
    fn bytes_that_are_not_text_do_not_stop_the_reading() {
        let mut b = LineBuffer::default();
        b.push(&[b'o', 0xC3, b'\n']);
        assert!(b.pop().is_some());
    }
}
