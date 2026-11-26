//! The state of the panel, with no window in it.
//!
//! A program gives the viewer an attachment with [`Viewer::open`], draws [`Viewer::content`] and
//! passes on what the person asks for. Showing an attachment never writes a file and never starts a
//! program. A dangerous attachment shows only its warning until the person confirms, and a copy
//! leaves the program only through [`Viewer::request`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::doc::Preview;
use crate::error::ViewerError;
use crate::kind::{inspect, Classification, Danger, Kind, OfficeKind};
use crate::limits::ViewerLimits;
use crate::outside::{save_copy, Consent, Launcher, Outbox, SystemLauncher};
use crate::pdf::pdfium::PdfiumDocument;
use crate::pdf::session::{PdfSession, Reply};
use crate::pdf::view::{PdfView, RenderSpec};
use crate::pdf::PdfDocument;
use crate::picture::{self, Raster};
use crate::source::{read_all, read_head, Source};
use crate::texts::Text;
use crate::{markdown, office, plain};

/// Opens a PDF held in memory. The default is PDFium; tests give their own.
pub type PdfEngine = fn(Vec<u8>) -> Result<Box<dyn PdfDocument>, ViewerError>;

fn pdfium_engine(data: Vec<u8>) -> Result<Box<dyn PdfDocument>, ViewerError> {
    Ok(Box::new(PdfiumDocument::open(data)?))
}

/// What a person asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenOutside,
    SaveCopy,
    /// Show the preview of a dangerous attachment.
    Preview,
}

/// What the program has to do, or may want to know, after a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerEvent {
    /// Ask the person where to save, then call [`Viewer::save_to`].
    ChooseSavePath {
        suggested: String,
    },
    /// The temporary copy handed to the system.
    OpenedOutside(PathBuf),
    Saved(PathBuf),
    Failed(ViewerError),
}

/// The page drawn last.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawnPage {
    /// Changes with every new drawing, so the window knows when to renew its texture.
    pub ticket: u64,
    pub page: u32,
    pub raster: Raster,
}

/// A PDF in the panel.
pub struct PdfPane {
    session: PdfSession,
    /// `None` while the document is being opened.
    pub view: Option<PdfView>,
    pub drawn: Option<DrawnPage>,
    pub page_error: Option<ViewerError>,
    wanted: Option<(u32, RenderSpec)>,
    /// The number of the last page asked for.
    asked: u64,
}

impl PdfPane {
    /// Asks for the page of the view at this size, once.
    pub fn want(&mut self, spec: RenderSpec) {
        let Some(view) = &self.view else { return };
        let mut spec = spec;
        spec.zoom = view.zoom;
        let wanted = (view.page(), spec);
        if self.wanted != Some(wanted) {
            self.wanted = Some(wanted);
            self.page_error = None;
            self.asked = self.session.render(wanted.0, spec);
        }
    }

    /// Searches the whole document.
    pub fn search(&mut self, query: &str) {
        if let Some(view) = &mut self.view {
            let ticket = view.start_search(query);
            self.session.search(query, ticket);
        }
    }

    /// True while an answer is still expected.
    pub fn busy(&self) -> bool {
        match &self.view {
            None => true,
            Some(view) => {
                view.searching()
                    || (self.wanted.is_some()
                        && self.page_error.is_none()
                        && self.drawn.as_ref().map(|d| d.ticket) != Some(self.asked))
            }
        }
    }

    fn take(&mut self, reply: Reply) -> Option<ViewerError> {
        match reply {
            Reply::Opened { pages } => self.view = Some(PdfView::new(pages)),
            Reply::Failed(e) => return Some(e),
            Reply::Page {
                ticket,
                page,
                result,
            } => match result {
                Ok(raster) => {
                    self.drawn = Some(DrawnPage {
                        ticket,
                        page,
                        raster,
                    });
                }
                Err(e) => self.page_error = Some(e),
            },
            Reply::Hits { ticket, hits } => {
                if let Some(view) = &mut self.view {
                    view.add_hits(ticket, hits);
                }
            }
            Reply::Searched { ticket } => {
                if let Some(view) = &mut self.view {
                    view.finish_search(ticket);
                }
            }
        }
        None
    }
}

/// What the panel shows.
pub enum Content {
    Empty,
    /// A dangerous attachment: only its warning, and whether a preview can be asked for.
    Withheld {
        has_preview: bool,
    },
    /// No preview inside; the reason, when there is one.
    Outside(Option<ViewerError>),
    Pdf(Box<PdfPane>),
    Picture(Raster),
    /// The text and tables of an office document, a Markdown file or a table of values.
    Document {
        preview: Preview,
        text_only: bool,
    },
    Text {
        text: String,
        truncated: bool,
    },
}

/// The panel of one attachment at a time.
pub struct Viewer {
    limits: ViewerLimits,
    outbox: Outbox,
    launcher: Box<dyn Launcher>,
    engine: PdfEngine,
    source: Option<Arc<dyn Source>>,
    class: Option<Classification>,
    content: Content,
    pending: Option<Action>,
    save_consent: Consent,
    notice: Option<Text>,
    generation: u64,
}

/// How a dangerous attachment can still be looked at, without running anything.
fn safe_preview(class: &Classification) -> Option<Kind> {
    match class.danger? {
        Danger::Script | Danger::ActiveContent => Some(Kind::Text),
        Danger::Macros => match class.kind {
            Kind::Office(k) if k != OfficeKind::Legacy => Some(class.kind),
            _ => None,
        },
        Danger::Executable | Danger::Archive => None,
    }
}

/// The words that explain an error.
pub fn error_text(error: &ViewerError) -> Text {
    match error {
        ViewerError::TooLarge { .. } => Text::TooLarge,
        ViewerError::Damaged(_) => Text::Damaged,
        ViewerError::Unsupported => Text::NoPreview,
        ViewerError::EngineMissing => Text::EngineMissing,
        ViewerError::EngineRejected(_) => Text::EngineRejected,
        ViewerError::PasswordProtected => Text::Password,
        ViewerError::Refused => Text::Refused,
        ViewerError::ConsentRequired | ViewerError::Io(_) => Text::Failed,
    }
}

impl Viewer {
    pub fn new(limits: ViewerLimits) -> Self {
        Viewer {
            limits,
            outbox: Outbox::new(),
            launcher: Box::new(SystemLauncher),
            engine: pdfium_engine,
            source: None,
            class: None,
            content: Content::Empty,
            pending: None,
            save_consent: Consent::NotAsked,
            notice: None,
            generation: 0,
        }
    }

    /// Another way of asking the system to open, and another place for the temporary copies.
    pub fn with_outside(mut self, launcher: Box<dyn Launcher>, outbox: Outbox) -> Self {
        self.launcher = launcher;
        self.outbox = outbox;
        self
    }

    pub fn with_pdf_engine(mut self, engine: PdfEngine) -> Self {
        self.engine = engine;
        self
    }

    pub fn limits(&self) -> &ViewerLimits {
        &self.limits
    }

    /// Changes with every attachment opened or closed.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn classification(&self) -> Option<&Classification> {
        self.class.as_ref()
    }

    pub fn content(&self) -> &Content {
        &self.content
    }

    pub fn pdf_mut(&mut self) -> Option<&mut PdfPane> {
        match &mut self.content {
            Content::Pdf(pane) => Some(pane),
            _ => None,
        }
    }

    /// The request that waits for the person to confirm.
    pub fn pending(&self) -> Option<Action> {
        self.pending
    }

    /// The last message of an action.
    pub fn notice(&self) -> Option<Text> {
        self.notice
    }

    /// The warning of the attachment shown, if it has one.
    pub fn warning(&self) -> Option<Text> {
        let class = self.class.as_ref()?;
        Some(match class.danger {
            Some(Danger::Executable) => Text::WarnExecutable,
            Some(Danger::Script) => Text::WarnScript,
            Some(Danger::Macros) => Text::WarnMacros,
            Some(Danger::Archive) => Text::WarnArchive,
            Some(Danger::ActiveContent) => Text::WarnActive,
            None if !class.known => Text::WarnUnknown,
            None => return None,
        })
    }

    pub fn close(&mut self) {
        self.source = None;
        self.class = None;
        self.content = Content::Empty;
        self.pending = None;
        self.save_consent = Consent::NotAsked;
        self.notice = None;
        self.generation += 1;
    }

    /// Shows an attachment.
    pub fn open(&mut self, source: Arc<dyn Source>) {
        self.close();
        let class = inspect(source.as_ref(), &self.limits);
        self.content = if source.len() > self.limits.max_attachment_bytes {
            Content::Outside(Some(ViewerError::TooLarge {
                limit: self.limits.max_attachment_bytes,
            }))
        } else if class.danger.is_some() {
            Content::Withheld {
                has_preview: safe_preview(&class).is_some(),
            }
        } else {
            self.load(source.as_ref(), class.kind, &class.extension)
        };
        self.class = Some(class);
        self.source = Some(source);
    }

    fn load(&self, source: &dyn Source, kind: Kind, extension: &str) -> Content {
        self.try_load(source, kind, extension)
            .unwrap_or_else(|e| Content::Outside(Some(e)))
    }

    fn try_load(
        &self,
        source: &dyn Source,
        kind: Kind,
        extension: &str,
    ) -> Result<Content, ViewerError> {
        let limits = &self.limits;
        // a text is cut, not refused: one byte more than shown says there was more
        let start = || {
            let count = limits.capped(limits.max_text_bytes).saturating_add(1);
            read_head(source, usize::try_from(count).unwrap_or(usize::MAX))
        };
        Ok(match kind {
            Kind::Pdf => {
                let data = read_all(source, limits.capped(limits.max_pdf_bytes))?;
                let engine = self.engine;
                let session = PdfSession::start(move || engine(data), limits.clone())?;
                Content::Pdf(Box::new(PdfPane {
                    session,
                    view: None,
                    drawn: None,
                    page_error: None,
                    wanted: None,
                    asked: 0,
                }))
            }
            Kind::Picture => {
                let data = read_all(source, limits.capped(limits.max_image_file_bytes))?;
                Content::Picture(picture::decode(&data, limits)?)
            }
            Kind::Markdown => Content::Document {
                preview: markdown::preview(&start()?, limits),
                text_only: false,
            },
            Kind::Table => Content::Document {
                preview: plain::table(&start()?, extension == "tsv", limits),
                text_only: false,
            },
            Kind::Text => {
                let (text, truncated) = plain::text(&start()?, limits);
                Content::Text { text, truncated }
            }
            Kind::Office(OfficeKind::Legacy) => Content::Outside(Some(ViewerError::Unsupported)),
            Kind::Office(office_kind) => {
                let data = read_all(source, limits.capped(limits.max_office_file_bytes))?;
                Content::Document {
                    preview: office::preview(office_kind, &data, limits)?,
                    text_only: true,
                }
            }
            Kind::Media | Kind::Cad | Kind::Other => Content::Outside(None),
        })
    }

    /// Reads what the PDF thread has answered. Call it on every frame; true when something changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut failed = None;
        if let Content::Pdf(pane) = &mut self.content {
            while let Some(reply) = pane.session.poll() {
                changed = true;
                if let Some(e) = pane.take(reply) {
                    failed = Some(e);
                }
            }
        }
        if let Some(e) = failed {
            self.content = Content::Outside(Some(e));
        }
        changed
    }

    /// What the person asked for. A dangerous or unknown attachment first waits for [`Viewer::confirm`].
    pub fn request(&mut self, action: Action) -> Option<ViewerEvent> {
        self.pending = None;
        self.notice = None;
        self.save_consent = Consent::NotAsked;
        let class = self.class.as_ref()?;
        if action == Action::OpenOutside && !class.may_launch() {
            self.notice = Some(Text::Refused);
            return Some(ViewerEvent::Failed(ViewerError::Refused));
        }
        if action == Action::Preview && safe_preview(class).is_none() {
            return None;
        }
        if class.needs_consent() {
            self.pending = Some(action);
            return None;
        }
        self.perform(action, Consent::NotAsked)
    }

    /// The person agreed to the warning.
    pub fn confirm(&mut self) -> Option<ViewerEvent> {
        let action = self.pending.take()?;
        self.perform(action, Consent::Given)
    }

    pub fn cancel(&mut self) {
        self.pending = None;
    }

    fn failed(&mut self, error: ViewerError) -> ViewerEvent {
        self.notice = Some(error_text(&error));
        ViewerEvent::Failed(error)
    }

    fn perform(&mut self, action: Action, consent: Consent) -> Option<ViewerEvent> {
        let source = Arc::clone(self.source.as_ref()?);
        let class = self.class.as_ref()?;
        match action {
            Action::OpenOutside => {
                let opened = self.outbox.open(
                    source.as_ref(),
                    consent,
                    &self.limits,
                    self.launcher.as_mut(),
                );
                Some(match opened {
                    Ok(path) => {
                        self.notice = Some(Text::HandedOver);
                        ViewerEvent::OpenedOutside(path)
                    }
                    Err(e) => self.failed(e),
                })
            }
            Action::SaveCopy => {
                self.save_consent = consent;
                Some(ViewerEvent::ChooseSavePath {
                    suggested: class.name.clone(),
                })
            }
            Action::Preview => {
                let kind = safe_preview(class)?;
                let extension = class.extension.clone();
                self.content = self.load(source.as_ref(), kind, &extension);
                None
            }
        }
    }

    /// Saves the copy where the person chose, after [`ViewerEvent::ChooseSavePath`].
    pub fn save_to(&mut self, path: &Path) -> ViewerEvent {
        let consent = std::mem::replace(&mut self.save_consent, Consent::NotAsked);
        let Some(source) = self.source.clone() else {
            return self.failed(ViewerError::Unsupported);
        };
        match save_copy(source.as_ref(), path, consent, &self.limits) {
            Ok(()) => {
                self.notice = Some(Text::Saved);
                ViewerEvent::Saved(path.to_path_buf())
            }
            Err(e) => self.failed(e),
        }
    }
}
