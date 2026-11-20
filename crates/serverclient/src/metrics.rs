//! What the link has done, for a status page or a log line: how many requests, how many tries, how many were repeated, how long the server took to answer
//! and how many times the connection was lost. Nothing here holds a token, a body or an address.

use std::time::Duration;

/// How many of the latest latencies are kept for the percentiles (a ring: the oldest is overwritten).
pub const SAMPLES: usize = 128;

/// The counters and latencies of one client. A request is one call of the client (with all its tries); an attempt is one try on the transport.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkMetrics {
    requests: u64,
    attempts: u64,
    retries: u64,
    replies: u64,
    server_errors: u64,
    failures: u64,
    disconnects: u64,
    samples: Vec<u64>,
    next: usize,
    last_us: Option<u64>,
    min_us: Option<u64>,
    max_us: u64,
    total_us: u128,
}

fn micros(d: Duration) -> u64 {
    u64::try_from(d.as_micros()).unwrap_or(u64::MAX)
}

impl LinkMetrics {
    /// Calls of the client that went to the transport.
    pub fn requests(&self) -> u64 {
        self.requests
    }

    /// Tries on the transport, the first ones and the repeated ones.
    pub fn attempts(&self) -> u64 {
        self.attempts
    }

    /// Tries after the first of a request.
    pub fn retries(&self) -> u64 {
        self.retries
    }

    /// Tries that got an answer (of any status).
    pub fn replies(&self) -> u64 {
        self.replies
    }

    /// Answers with a status of 500 or more.
    pub fn server_errors(&self) -> u64 {
        self.server_errors
    }

    /// Requests that ended without an answer, after all their tries.
    pub fn failures(&self) -> u64 {
        self.failures
    }

    /// Times the link went from reachable (or unknown) to unreachable.
    pub fn disconnects(&self) -> u64 {
        self.disconnects
    }

    pub(crate) fn begin_request(&mut self) {
        self.requests += 1;
    }

    pub(crate) fn begin_attempt(&mut self, is_retry: bool) {
        self.attempts += 1;
        if is_retry {
            self.retries += 1;
        }
    }

    /// An attempt that got an answer after `latency`.
    pub(crate) fn record_reply(&mut self, latency: Duration, status: u16) {
        self.replies += 1;
        if status >= 500 {
            self.server_errors += 1;
        }
        let us = micros(latency);
        self.last_us = Some(us);
        self.min_us = Some(self.min_us.map_or(us, |m| m.min(us)));
        self.max_us = self.max_us.max(us);
        self.total_us += u128::from(us);
        if self.samples.len() < SAMPLES {
            self.samples.push(us);
        } else {
            self.samples[self.next] = us;
        }
        self.next = (self.next + 1) % SAMPLES;
    }

    pub(crate) fn record_failure(&mut self, newly_disconnected: bool) {
        self.failures += 1;
        if newly_disconnected {
            self.disconnects += 1;
        }
    }

    /// The latency of the last answer.
    pub fn last_latency(&self) -> Option<Duration> {
        self.last_us.map(Duration::from_micros)
    }

    pub fn min_latency(&self) -> Option<Duration> {
        self.min_us.map(Duration::from_micros)
    }

    pub fn max_latency(&self) -> Option<Duration> {
        (self.replies > 0).then(|| Duration::from_micros(self.max_us))
    }

    /// The mean of every answer so far (not only the latest ones).
    pub fn mean_latency(&self) -> Option<Duration> {
        if self.replies == 0 {
            return None;
        }
        let mean = self.total_us / u128::from(self.replies);
        Some(Duration::from_micros(
            u64::try_from(mean).unwrap_or(u64::MAX),
        ))
    }

    /// The latency under which `percent` of the latest answers fell (nearest rank): `percentile(50)` is the median, `percentile(95)` the slow tail.
    /// `percent` above 100 is 100.
    pub fn percentile(&self, percent: u32) -> Option<Duration> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let percent = percent.min(100) as usize;
        let rank = (percent * sorted.len())
            .div_ceil(100)
            .clamp(1, sorted.len());
        Some(Duration::from_micros(sorted[rank - 1]))
    }

    /// How many of the requests that went out ended without an answer, from 0.0 to 1.0.
    pub fn failure_rate(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            self.failures as f64 / self.requests as f64
        }
    }

    /// Forgets everything (a new session of work).
    pub fn reset(&mut self) {
        *self = LinkMetrics::default();
    }

    /// `key = value` lines for a log or a page of diagnostics (times in microseconds, `-` where there is none yet).
    pub fn to_text(&self) -> String {
        let us = |d: Option<Duration>| d.map_or("-".to_string(), |d| d.as_micros().to_string());
        format!(
            "requests = {}\nattempts = {}\nretries = {}\nreplies = {}\nserver_errors = {}\nfailures = {}\ndisconnects = {}\nlatency_last_us = {}\nlatency_min_us = {}\nlatency_mean_us = {}\nlatency_p50_us = {}\nlatency_p95_us = {}\nlatency_max_us = {}\n",
            self.requests,
            self.attempts,
            self.retries,
            self.replies,
            self.server_errors,
            self.failures,
            self.disconnects,
            us(self.last_latency()),
            us(self.min_latency()),
            us(self.mean_latency()),
            us(self.percentile(50)),
            us(self.percentile(95)),
            us(self.max_latency()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_new_record_is_empty() {
        let m = LinkMetrics::default();
        assert_eq!(m.requests(), 0);
        assert_eq!(m.mean_latency(), None);
        assert_eq!(m.percentile(50), None);
        assert_eq!(m.max_latency(), None);
        assert_eq!(m.failure_rate(), 0.0);
        assert!(m.to_text().contains("latency_p95_us = -"));
    }

    #[test]
    fn latencies_give_min_max_mean_and_percentiles() {
        let mut m = LinkMetrics::default();
        for n in 1..=100u64 {
            m.begin_request();
            m.begin_attempt(false);
            m.record_reply(ms(n), 200);
        }
        assert_eq!(m.replies(), 100);
        assert_eq!(m.min_latency(), Some(ms(1)));
        assert_eq!(m.max_latency(), Some(ms(100)));
        assert_eq!(m.last_latency(), Some(ms(100)));
        assert_eq!(m.mean_latency(), Some(Duration::from_micros(50_500)));
        assert_eq!(m.percentile(50), Some(ms(50)));
        assert_eq!(m.percentile(95), Some(ms(95)));
        assert_eq!(m.percentile(100), Some(ms(100)));
        assert_eq!(m.percentile(900), Some(ms(100)));
        assert_eq!(m.percentile(0), Some(ms(1)));
    }

    #[test]
    fn the_ring_keeps_only_the_latest_samples_but_the_mean_counts_all() {
        let mut m = LinkMetrics::default();
        for _ in 0..SAMPLES {
            m.record_reply(ms(1000), 200);
        }
        for _ in 0..SAMPLES {
            m.record_reply(ms(1), 200);
        }
        assert_eq!(m.percentile(100), Some(ms(1)));
        assert_eq!(m.max_latency(), Some(ms(1000)));
        assert_eq!(m.mean_latency(), Some(Duration::from_micros(500_500)));
    }

    #[test]
    fn retries_failures_and_disconnects_are_counted() {
        let mut m = LinkMetrics::default();
        m.begin_request();
        m.begin_attempt(false);
        m.begin_attempt(true);
        m.begin_attempt(true);
        m.record_failure(true);
        m.begin_request();
        m.begin_attempt(false);
        m.record_reply(ms(5), 503);
        assert_eq!(m.requests(), 2);
        assert_eq!(m.attempts(), 4);
        assert_eq!(m.retries(), 2);
        assert_eq!(m.failures(), 1);
        assert_eq!(m.disconnects(), 1);
        assert_eq!(m.server_errors(), 1);
        assert_eq!(m.failure_rate(), 0.5);
        m.reset();
        assert_eq!(m, LinkMetrics::default());
    }
}
