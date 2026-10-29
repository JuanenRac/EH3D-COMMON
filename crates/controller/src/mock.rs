//! A line transport for the tests: what the controller is going to answer is queued, what the program writes is kept.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use crate::grbl::LineTransport;

#[derive(Debug, Default)]
pub struct MockTransport {
    pub written: Vec<u8>,
    pub replies: VecDeque<String>,
}

impl MockTransport {
    pub fn with_replies(lines: &[&str]) -> MockTransport {
        MockTransport {
            written: Vec::new(),
            replies: lines.iter().map(|l| l.to_string()).collect(),
        }
    }

    /// What was written, as text (the control bytes such as the soft reset show as their characters).
    pub fn text(&self) -> String {
        self.written.iter().map(|&b| b as char).collect()
    }
}

impl LineTransport for MockTransport {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.written.extend_from_slice(bytes);
        Ok(())
    }

    fn read_line(&mut self, _timeout: Duration) -> io::Result<Option<String>> {
        Ok(self.replies.pop_front())
    }
}
