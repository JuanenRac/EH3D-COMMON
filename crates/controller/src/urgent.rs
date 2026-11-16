//! The bytes that cannot wait. A controller that is busy waiting for the `ok` of a line cannot be given another order by the thread that
//! sent the line, yet a stop, a hold or a reset must reach the machine at once. An [`UrgentChannel`] is the way in: any thread keeps a
//! clone of it, puts an [`Urgent`] byte in, and the controller writes it to the machine at the next moment it looks (while it waits for an
//! `ok` it looks every few milliseconds), without waiting for the answer to the line it is waiting for and without queueing behind lines.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

/// What can be sent out of turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgent {
    /// The status report (`?`).
    Status,
    /// The feed hold (`!`).
    Hold,
    /// Goes on after a hold (`~`).
    Resume,
    /// Stops the job: holds, waits until the machine stands still and then resets (a reset while it still moves loses its position).
    StopJob,
    /// The soft reset at once (`0x18`): the emergency stop.
    Reset,
}

impl Urgent {
    /// The bytes that go out. `StopJob` is the hold and the reset; a controller that can wait between the two does.
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Urgent::Status => b"?",
            Urgent::Hold => b"!",
            Urgent::Resume => b"~",
            Urgent::StopJob => b"!\x18",
            Urgent::Reset => b"\x18",
        }
    }

    /// Whether it ends the job that was running (the line that waits for its answer will not get one).
    pub fn ends_job(self) -> bool {
        matches!(self, Urgent::StopJob | Urgent::Reset)
    }
}

/// The queue of urgent bytes, shared between the thread that gives them and the controller that sends them. Cloning it shares the queue.
#[derive(Debug, Clone, Default)]
pub struct UrgentChannel {
    queue: Arc<Mutex<VecDeque<Urgent>>>,
}

impl UrgentChannel {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<Urgent>> {
        // a thread that panicked cannot make a stop impossible: the queue is plain data
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Puts a byte in; it never waits for the controller.
    pub fn send(&self, urgent: Urgent) {
        self.lock().push_back(urgent);
    }

    /// Takes everything that is waiting, in the order it came.
    pub fn take(&self) -> Vec<Urgent> {
        self.lock().drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    /// How many handles of this channel exist (the controller's own counts one): more than one means somebody may be sending.
    pub fn handles(&self) -> usize {
        Arc::strong_count(&self.queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bytes_come_out_in_the_order_they_went_in_and_only_once() {
        let a = UrgentChannel::new();
        let b = a.clone();
        assert!(a.is_empty());
        b.send(Urgent::Hold);
        b.send(Urgent::Status);
        assert_eq!(a.take(), vec![Urgent::Hold, Urgent::Status]);
        assert!(a.take().is_empty());
        assert_eq!(a.handles(), 2);
        drop(b);
        assert_eq!(a.handles(), 1);
    }

    #[test]
    fn each_urgent_is_its_byte_of_the_protocol() {
        assert_eq!(Urgent::Reset.bytes(), &[0x18]);
        assert_eq!(Urgent::Hold.bytes(), b"!");
        assert_eq!(Urgent::Status.bytes(), b"?");
        assert_eq!(Urgent::Resume.bytes(), b"~");
        assert!(Urgent::Reset.ends_job() && Urgent::StopJob.ends_job());
        assert!(!Urgent::Hold.ends_job() && !Urgent::Status.ends_job());
    }

    #[test]
    fn the_channel_can_be_fed_from_another_thread() {
        let a = UrgentChannel::new();
        let b = a.clone();
        std::thread::spawn(move || b.send(Urgent::Reset))
            .join()
            .unwrap();
        assert_eq!(a.take(), vec![Urgent::Reset]);
    }
}
