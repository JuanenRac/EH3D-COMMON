//! A line transport for the tests: what the controller is going to answer is queued, what the program writes is kept.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use crate::grbl::LineTransport;

#[derive(Debug, Default)]
pub struct MockTransport {
    pub written: Vec<u8>,
    pub replies: VecDeque<String>,
    /// The time each read was allowed to wait.
    pub timeouts: Vec<Duration>,
    /// Replies that arrive later: each one with the moment, on the virtual clock, at which it becomes readable. They come after the immediate replies, in the order given.
    pub delayed: VecDeque<(Duration, String)>,
    /// The virtual clock: it moves only when a read waits, by what the read waited (no real time passes), so a test of a timeout takes no time.
    pub elapsed: Duration,
}

impl MockTransport {
    pub fn with_replies(lines: &[&str]) -> MockTransport {
        MockTransport {
            written: Vec::new(),
            replies: lines.iter().map(|l| l.to_string()).collect(),
            timeouts: Vec::new(),
            delayed: VecDeque::new(),
            elapsed: Duration::ZERO,
        }
    }

    /// A transport whose replies arrive at the given moments of the virtual clock (in order): a read that waits less than the moment still to come gets nothing.
    pub fn with_delayed(replies: &[(Duration, &str)]) -> MockTransport {
        MockTransport {
            delayed: replies
                .iter()
                .map(|(at, line)| (*at, line.to_string()))
                .collect(),
            ..MockTransport::default()
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

    fn read_line(&mut self, timeout: Duration) -> io::Result<Option<String>> {
        self.timeouts.push(timeout);
        if let Some(line) = self.replies.pop_front() {
            return Ok(Some(line));
        }
        if let Some((at, _)) = self.delayed.front() {
            if *at <= self.elapsed + timeout {
                if let Some((at, line)) = self.delayed.pop_front() {
                    self.elapsed = self.elapsed.max(at);
                    return Ok(Some(line));
                }
            }
        }
        // nothing came in the time given: the wait is spent
        self.elapsed += timeout;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grbl::GrblController;
    use crate::{Controller, Error, State};

    #[test]
    fn a_delayed_reply_is_readable_only_when_the_virtual_clock_reaches_it() {
        let mut t = MockTransport::with_delayed(&[(Duration::from_millis(500), "ok")]);
        assert_eq!(t.read_line(Duration::from_millis(200)).unwrap(), None);
        assert_eq!(t.read_line(Duration::from_millis(200)).unwrap(), None);
        assert_eq!(t.elapsed, Duration::from_millis(400));
        assert_eq!(
            t.read_line(Duration::from_millis(200)).unwrap(),
            Some("ok".to_string())
        );
        assert_eq!(t.elapsed, Duration::from_millis(500));
        // the immediate replies come first and cost no time
        let mut t = MockTransport::with_replies(&["a"]);
        t.delayed
            .push_back((Duration::from_secs(1), "b".to_string()));
        assert_eq!(t.read_line(Duration::ZERO).unwrap(), Some("a".to_string()));
        assert_eq!(t.elapsed, Duration::ZERO);
        assert_eq!(
            t.read_line(Duration::from_secs(2)).unwrap(),
            Some("b".to_string())
        );
    }

    #[test]
    fn the_controller_times_out_on_a_reply_that_is_too_late_and_takes_one_that_is_in_time() {
        let report = "<Idle|MPos:0.000,0.000,0.000|FS:0,0>";
        let late = MockTransport::with_delayed(&[(Duration::from_secs(5), report)]);
        let mut c = GrblController::new(late).with_timeout(Duration::from_secs(1));
        assert_eq!(c.status(), Err(Error::Timeout));
        let in_time = MockTransport::with_delayed(&[(Duration::from_millis(200), report)]);
        let mut c = GrblController::new(in_time).with_timeout(Duration::from_secs(1));
        assert_eq!(c.status().unwrap().state, State::Idle);
    }
}
