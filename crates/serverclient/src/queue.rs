//! The queue of requests that could not be sent because the server was not reachable.
//!
//! Only a request that can be sent again without harm waits here: a `PUT` (it sets a state) and a `DELETE` (it removes one). A `POST` (a job, a login) is
//! never kept, because sending it later could do the work twice, and a `GET` has nothing to replay. The queue is bounded in the number of requests, in the
//! size of each body, in the size of all of them and in age, and what it cannot take it refuses with the reason. Two requests for the same path are one:
//! the latest wins. Nothing is written to disk and no token is kept: the token travels with the request when it is finally sent.

use crate::transport::check_request_part;
use std::collections::VecDeque;

/// The limits of a queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueLimits {
    /// The most requests waiting at once.
    pub max_items: usize,
    /// The most bytes of the body of one request.
    pub max_body_bytes: usize,
    /// The most bytes of all the bodies together.
    pub max_total_bytes: usize,
    /// The seconds after which a waiting request is dropped instead of sent (an order that old is no longer what the person meant).
    pub max_age_s: u64,
}

impl Default for QueueLimits {
    fn default() -> QueueLimits {
        QueueLimits {
            max_items: 64,
            max_body_bytes: 64 * 1024,
            max_total_bytes: 1024 * 1024,
            max_age_s: 3600,
        }
    }
}

/// Why a request was not kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejected {
    /// Only `PUT` and `DELETE` can wait; the method is named.
    NotRepeatable(String),
    /// The path is not one that can be sent (it must start with `/` and have no space or control character).
    BadPath,
    /// The queue holds `max_items` requests already.
    Full,
    /// The body of this request is larger than `max_body_bytes`.
    BodyTooLarge,
    /// With this body the bodies together would pass `max_total_bytes`.
    TotalTooLarge,
}

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rejected::NotRepeatable(m) => write!(f, "a {m} cannot wait: it could be done twice"),
            Rejected::BadPath => f.write_str("the path cannot be sent"),
            Rejected::Full => f.write_str("the queue is full"),
            Rejected::BodyTooLarge => f.write_str("the body is too large for the queue"),
            Rejected::TotalTooLarge => f.write_str("the queue would hold too many bytes"),
        }
    }
}

impl std::error::Error for Rejected {}

/// A request that waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queued {
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    /// The second (of the clock of the client) at which it was queued.
    pub queued_at: u64,
}

impl Queued {
    fn bytes(&self) -> usize {
        self.body.as_ref().map_or(0, String::len)
    }
}

/// Can a request with this method wait in the queue?
pub fn is_queueable(method: &str) -> bool {
    matches!(method, "PUT" | "DELETE")
}

/// The waiting requests, oldest first.
#[derive(Debug, Clone, Default)]
pub struct OfflineQueue {
    limits: QueueLimits,
    items: VecDeque<Queued>,
    total_bytes: usize,
    refused: u64,
    expired: u64,
    replaced: u64,
}

impl OfflineQueue {
    pub fn new(limits: QueueLimits) -> OfflineQueue {
        OfflineQueue {
            limits,
            ..OfflineQueue::default()
        }
    }

    pub fn limits(&self) -> QueueLimits {
        self.limits
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The bytes of all the bodies waiting.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// How many requests were refused since the queue was made.
    pub fn refused(&self) -> u64 {
        self.refused
    }

    /// How many were dropped for being too old.
    pub fn expired(&self) -> u64 {
        self.expired
    }

    /// How many were replaced by a later request for the same path.
    pub fn replaced(&self) -> u64 {
        self.replaced
    }

    /// The waiting requests, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Queued> {
        self.items.iter()
    }

    /// Keeps a request, or says why not. A request for a path that already waits takes its place (the latest wins, and it goes to the end).
    pub fn push(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
        now_s: u64,
    ) -> Result<(), Rejected> {
        let verdict = self.check(method, path, body);
        if let Err(e) = verdict {
            self.refused += 1;
            return Err(e);
        }
        let item = Queued {
            method: method.to_string(),
            path: path.to_string(),
            body: body.map(str::to_string),
            queued_at: now_s,
        };
        self.items.push_back(item);
        self.total_bytes += body.map_or(0, str::len);
        Ok(())
    }

    fn check(&mut self, method: &str, path: &str, body: Option<&str>) -> Result<(), Rejected> {
        if !is_queueable(method) {
            return Err(Rejected::NotRepeatable(method.to_string()));
        }
        if !path.starts_with('/') || check_request_part("the path", path).is_err() {
            return Err(Rejected::BadPath);
        }
        let size = body.map_or(0, str::len);
        if size > self.limits.max_body_bytes {
            return Err(Rejected::BodyTooLarge);
        }
        // the request that would be replaced does not count against the limits
        let same = self.items.iter().position(|q| q.path == path);
        let (freed_items, freed_bytes) = match same {
            Some(i) => (1, self.items[i].bytes()),
            None => (0, 0),
        };
        if self.items.len() - freed_items >= self.limits.max_items {
            return Err(Rejected::Full);
        }
        if self.total_bytes - freed_bytes + size > self.limits.max_total_bytes {
            return Err(Rejected::TotalTooLarge);
        }
        if let Some(i) = same {
            if let Some(old) = self.items.remove(i) {
                self.total_bytes -= old.bytes();
                self.replaced += 1;
            }
        }
        Ok(())
    }

    /// Drops what has waited longer than `max_age_s`; gives how many.
    pub fn expire(&mut self, now_s: u64) -> usize {
        let max_age = self.limits.max_age_s;
        let before = self.items.len();
        let mut kept = VecDeque::with_capacity(before);
        for q in self.items.drain(..) {
            if now_s.saturating_sub(q.queued_at) > max_age {
                self.total_bytes -= q.bytes();
                self.expired += 1;
            } else {
                kept.push_back(q);
            }
        }
        self.items = kept;
        before - self.items.len()
    }

    /// Takes the oldest request out.
    pub fn pop_front(&mut self) -> Option<Queued> {
        let q = self.items.pop_front()?;
        self.total_bytes -= q.bytes();
        Some(q)
    }

    /// Puts a request back at the front (one that could not be sent yet). It does not count against the limits again.
    pub fn push_front(&mut self, q: Queued) {
        self.total_bytes += q.bytes();
        self.items.push_front(q);
    }

    /// Forgets every waiting request.
    pub fn clear(&mut self) {
        self.items.clear();
        self.total_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> OfflineQueue {
        OfflineQueue::new(QueueLimits {
            max_items: 3,
            max_body_bytes: 10,
            max_total_bytes: 20,
            max_age_s: 100,
        })
    }

    #[test]
    fn only_put_and_delete_can_wait() {
        let mut q = small();
        assert_eq!(q.push("PUT", "/a", Some("1"), 0), Ok(()));
        assert_eq!(q.push("DELETE", "/b", None, 0), Ok(()));
        for m in ["POST", "GET", "HEAD", "OPTIONS", "PATCH", "put"] {
            assert_eq!(
                q.push(m, "/c", None, 0),
                Err(Rejected::NotRepeatable(m.to_string())),
                "{m}"
            );
        }
        assert_eq!(q.len(), 2);
        assert_eq!(q.refused(), 6);
    }

    #[test]
    fn a_path_that_cannot_be_sent_is_refused() {
        let mut q = small();
        for p in ["", "a", "/a b", "/a\r\nHost: x", "/\u{e9}"] {
            assert_eq!(q.push("PUT", p, None, 0), Err(Rejected::BadPath), "{p:?}");
        }
        assert!(q.is_empty());
    }

    #[test]
    fn the_number_of_requests_is_bounded() {
        let mut q = small();
        for p in ["/a", "/b", "/c"] {
            q.push("PUT", p, None, 0).unwrap();
        }
        assert_eq!(q.push("PUT", "/d", None, 0), Err(Rejected::Full));
        assert_eq!(q.len(), 3);
        // a request for a path that waits is a replacement, not one more
        assert_eq!(q.push("PUT", "/a", Some("new"), 1), Ok(()));
        assert_eq!(q.len(), 3);
        assert_eq!(q.replaced(), 1);
    }

    #[test]
    fn the_size_of_a_body_and_of_all_of_them_is_bounded() {
        let mut q = small();
        assert_eq!(
            q.push("PUT", "/a", Some("12345678901"), 0),
            Err(Rejected::BodyTooLarge)
        );
        q.push("PUT", "/a", Some("1234567890"), 0).unwrap();
        q.push("PUT", "/b", Some("1234567890"), 0).unwrap();
        assert_eq!(q.total_bytes(), 20);
        assert_eq!(
            q.push("PUT", "/c", Some("1"), 0),
            Err(Rejected::TotalTooLarge)
        );
        // replacing a body frees its bytes first
        assert_eq!(q.push("PUT", "/a", Some("12"), 0), Ok(()));
        assert_eq!(q.total_bytes(), 12);
    }

    #[test]
    fn the_latest_request_for_a_path_wins_and_goes_to_the_end() {
        let mut q = small();
        q.push("PUT", "/a", Some("1"), 0).unwrap();
        q.push("PUT", "/b", Some("2"), 0).unwrap();
        q.push("DELETE", "/a", None, 1).unwrap();
        let order: Vec<(&str, &str)> = q
            .iter()
            .map(|i| (i.method.as_str(), i.path.as_str()))
            .collect();
        assert_eq!(order, vec![("PUT", "/b"), ("DELETE", "/a")]);
        assert_eq!(q.total_bytes(), 1);
    }

    #[test]
    fn what_waited_too_long_is_dropped() {
        let mut q = small();
        q.push("PUT", "/old", Some("12"), 0).unwrap();
        q.push("PUT", "/new", Some("34"), 90).unwrap();
        assert_eq!(q.expire(100), 0);
        assert_eq!(q.expire(101), 1);
        assert_eq!(q.len(), 1);
        assert_eq!(q.total_bytes(), 2);
        assert_eq!(q.expired(), 1);
        assert_eq!(q.pop_front().map(|i| i.path), Some("/new".to_string()));
        assert_eq!(q.total_bytes(), 0);
    }

    #[test]
    fn a_request_put_back_keeps_its_place() {
        let mut q = small();
        q.push("PUT", "/a", Some("1"), 0).unwrap();
        q.push("PUT", "/b", Some("2"), 0).unwrap();
        let first = q.pop_front().unwrap();
        q.push_front(first);
        assert_eq!(q.total_bytes(), 2);
        assert_eq!(q.iter().next().map(|i| i.path.as_str()), Some("/a"));
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.total_bytes(), 0);
    }
}
