//! A document on a thread of its own. The window sends what it wants and reads the answers when they
//! are there; it never waits for the library. A newer request of a page or of a search makes the
//! older ones be dropped, and a search gives way to a page between two pages.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use super::view::{render_size, RenderSpec};
use super::{Hit, PdfDocument};
use crate::error::ViewerError;
use crate::limits::ViewerLimits;
use crate::picture::Raster;

#[derive(Debug)]
enum Request {
    Render {
        page: u32,
        spec: RenderSpec,
        ticket: u64,
    },
    Search {
        query: String,
        ticket: u64,
    },
}

/// What the thread answers.
#[derive(Debug, PartialEq)]
pub enum Reply {
    Opened {
        pages: u32,
    },
    /// The document could not be opened; nothing else follows.
    Failed(ViewerError),
    Page {
        ticket: u64,
        page: u32,
        result: Result<Raster, ViewerError>,
    },
    /// The matches of one page.
    Hits {
        ticket: u64,
        hits: Vec<Hit>,
    },
    Searched {
        ticket: u64,
    },
}

struct Search {
    query: String,
    ticket: u64,
    page: u32,
    found: u64,
}

fn run(
    document: &dyn PdfDocument,
    requests: &Receiver<Request>,
    replies: &Sender<Reply>,
    newest: &AtomicU64,
    limits: &ViewerLimits,
) {
    let mut search: Option<Search> = None;
    loop {
        let request = if search.is_some() {
            match requests.try_recv() {
                Ok(request) => Some(request),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            }
        } else {
            match requests.recv() {
                Ok(request) => Some(request),
                Err(_) => return,
            }
        };
        let sent = match request {
            Some(Request::Render { page, spec, ticket }) => {
                if ticket < newest.load(Ordering::SeqCst) {
                    continue;
                }
                let result = document.page_size(page).and_then(|points| {
                    let (w, h) = render_size(points, spec, limits.max_render_pixels);
                    document.render(page, w, h)
                });
                replies.send(Reply::Page {
                    ticket,
                    page,
                    result,
                })
            }
            Some(Request::Search { query, ticket }) => {
                search = (!query.is_empty()).then_some(Search {
                    query,
                    ticket,
                    page: 0,
                    found: 0,
                });
                Ok(())
            }
            None => {
                let Some(now) = search.as_mut() else { continue };
                let room = limits.max_search_hits.saturating_sub(now.found);
                if now.page >= document.page_count() || room == 0 {
                    let ticket = now.ticket;
                    search = None;
                    replies.send(Reply::Searched { ticket })
                } else {
                    let max = usize::try_from(room).unwrap_or(usize::MAX);
                    // a page that cannot be read has no matches
                    let hits = document.find(now.page, &now.query, max).unwrap_or_default();
                    now.page += 1;
                    now.found += hits.len() as u64;
                    if hits.is_empty() {
                        Ok(())
                    } else {
                        replies.send(Reply::Hits {
                            ticket: now.ticket,
                            hits,
                        })
                    }
                }
            }
        };
        if sent.is_err() {
            return;
        }
    }
}

/// The side of the window.
pub struct PdfSession {
    requests: Sender<Request>,
    replies: Receiver<Reply>,
    newest: Arc<AtomicU64>,
    tickets: u64,
}

impl PdfSession {
    /// Opens a document on a new thread. `open` runs there, so the document never changes thread.
    pub fn start<F>(open: F, limits: ViewerLimits) -> Result<Self, ViewerError>
    where
        F: FnOnce() -> Result<Box<dyn PdfDocument>, ViewerError> + Send + 'static,
    {
        let (requests, inbox) = channel();
        let (outbox, replies) = channel();
        let newest = Arc::new(AtomicU64::new(0));
        let seen = Arc::clone(&newest);
        std::thread::Builder::new()
            .name("eh3d-pdf".into())
            .spawn(move || match open() {
                Ok(document) => {
                    let pages = document.page_count();
                    if outbox.send(Reply::Opened { pages }).is_ok() {
                        run(document.as_ref(), &inbox, &outbox, &seen, &limits);
                    }
                }
                Err(e) => {
                    let _ = outbox.send(Reply::Failed(e));
                }
            })?;
        Ok(PdfSession {
            requests,
            replies,
            newest,
            tickets: 0,
        })
    }

    /// Asks for a page and gives the number of the request.
    pub fn render(&mut self, page: u32, spec: RenderSpec) -> u64 {
        self.tickets += 1;
        self.newest.store(self.tickets, Ordering::SeqCst);
        // a thread that has ended already said why
        let _ = self.requests.send(Request::Render {
            page,
            spec,
            ticket: self.tickets,
        });
        self.tickets
    }

    /// Searches every page for `query`; the answers carry `ticket`.
    pub fn search(&mut self, query: &str, ticket: u64) {
        let _ = self.requests.send(Request::Search {
            query: query.to_string(),
            ticket,
        });
    }

    /// An answer, if one is ready.
    pub fn poll(&self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }

    /// Waits at most `timeout` for an answer; `None` when none came or the thread has ended.
    pub fn wait(&self, timeout: Duration) -> Option<Reply> {
        match self.replies.recv_timeout(timeout) {
            Ok(reply) => Some(reply),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    }
}
