//! PDF pages inside the program.
//!
//! [`pins`] says which build of PDFium is accepted and finds it in the folder of the program,
//! [`pdfium`] calls it, [`session`] keeps it on a thread of its own so a heavy page does not stop the
//! window, and [`view`] is the state of the panel: page, zoom and search.

pub mod pdfium;
pub mod pins;
pub mod session;
pub mod view;

use crate::error::ViewerError;
use crate::picture::Raster;

/// A rectangle on a page as fractions from 0 to 1 of its width and height, from the top left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// One match of a search: the areas it covers on its page.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub page: u32,
    pub areas: Vec<Area>,
}

/// An open document. [`pdfium`] gives the real one; tests give their own.
pub trait PdfDocument {
    fn page_count(&self) -> u32;
    /// Width and height of a page, in points (1/72 of an inch).
    fn page_size(&self, page: u32) -> Result<(f32, f32), ViewerError>;
    /// The page drawn on white at the size asked, in pixels.
    fn render(&self, page: u32, width: u32, height: u32) -> Result<Raster, ViewerError>;
    /// The matches of a text on one page, at most `max`, ignoring upper and lower case.
    fn find(&self, page: u32, query: &str, max: usize) -> Result<Vec<Hit>, ViewerError>;
}
