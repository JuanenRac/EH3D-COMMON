//! The state of the panel of a PDF: which page, how large, and what a search found. No window and
//! no library in it.

use super::pdfium::MAX_RENDER_SIDE;
use super::Hit;

/// The steps of the zoom, in percent of the real size on a screen of 96 points per inch.
pub const ZOOM_STEPS: &[u32] = &[25, 50, 75, 100, 125, 150, 200, 300, 400];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zoom {
    /// The page as wide as the panel.
    FitWidth,
    Percent(u32),
}

/// What decides the size a page is drawn at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSpec {
    pub zoom: Zoom,
    /// The width of the panel in pixels of the screen.
    pub area_width: u32,
    /// Pixels of the screen per point of the interface.
    pub pixels_per_point: f32,
    /// The longest side the window can show as one picture.
    pub max_side: u32,
}

fn sane(value: f32, otherwise: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        otherwise
    }
}

/// The size in pixels of a page of `points` (width, height), never more than `max_pixels` in all
/// nor the side of `spec` (itself at most [`MAX_RENDER_SIDE`]), whatever the document says its size is.
pub fn render_size(points: (f32, f32), spec: RenderSpec, max_pixels: u64) -> (u32, u32) {
    let width = sane(points.0, 612.0).clamp(1.0, 100_000.0);
    let height = sane(points.1, 792.0).clamp(1.0, 100_000.0);
    let mut scale = match spec.zoom {
        Zoom::FitWidth => spec.area_width.max(64) as f32 / width,
        Zoom::Percent(percent) => {
            sane(spec.pixels_per_point, 1.0) * (96.0 / 72.0) * percent.clamp(10, 800) as f32 / 100.0
        }
    };
    let longest = spec.max_side.clamp(64, MAX_RENDER_SIDE);
    let side = longest as f32;
    scale = scale.min(side / width).min(side / height);
    let pixels = f64::from(width * scale) * f64::from(height * scale);
    if pixels > max_pixels as f64 {
        scale *= (max_pixels as f64 / pixels).sqrt() as f32;
    }
    let whole = |v: f32| (v.floor() as u32).clamp(1, longest);
    (whole(width * scale), whole(height * scale))
}

/// The state of the panel of one document.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfView {
    pages: u32,
    page: u32,
    pub zoom: Zoom,
    query: String,
    hits: Vec<Hit>,
    current: Option<usize>,
    searching: bool,
    search: u64,
}

impl PdfView {
    pub fn new(pages: u32) -> Self {
        PdfView {
            pages,
            page: 0,
            zoom: Zoom::FitWidth,
            query: String::new(),
            hits: Vec::new(),
            current: None,
            searching: false,
            search: 0,
        }
    }

    pub fn pages(&self) -> u32 {
        self.pages
    }

    /// The page shown, from 0.
    pub fn page(&self) -> u32 {
        self.page
    }

    /// Goes to a page; past the end is the last one.
    pub fn go(&mut self, page: u32) {
        self.page = page.min(self.pages.saturating_sub(1));
    }

    pub fn next_page(&mut self) {
        self.go(self.page.saturating_add(1));
    }

    pub fn previous_page(&mut self) {
        self.go(self.page.saturating_sub(1));
    }

    pub fn zoom_in(&mut self) {
        let now = match self.zoom {
            Zoom::Percent(p) => p,
            Zoom::FitWidth => 100,
        };
        let next = ZOOM_STEPS.iter().find(|s| **s > now);
        self.zoom = Zoom::Percent(next.copied().unwrap_or(400));
    }

    pub fn zoom_out(&mut self) {
        let now = match self.zoom {
            Zoom::Percent(p) => p,
            Zoom::FitWidth => 100,
        };
        let next = ZOOM_STEPS.iter().rev().find(|s| **s < now);
        self.zoom = Zoom::Percent(next.copied().unwrap_or(25));
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Starts a new search and gives its number; an empty text clears the search.
    pub fn start_search(&mut self, query: &str) -> u64 {
        self.query = query.to_string();
        self.hits.clear();
        self.current = None;
        self.searching = !query.is_empty();
        self.search += 1;
        self.search
    }

    /// The matches of one page of search number `search`; an older search is ignored.
    pub fn add_hits(&mut self, search: u64, hits: Vec<Hit>) {
        if search != self.search || hits.is_empty() {
            return;
        }
        let first = self.hits.is_empty();
        self.hits.extend(hits);
        if first {
            self.show(0);
        }
    }

    pub fn finish_search(&mut self, search: u64) {
        if search == self.search {
            self.searching = false;
        }
    }

    pub fn searching(&self) -> bool {
        self.searching
    }

    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    /// The number of the match shown, from 0.
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    fn show(&mut self, index: usize) {
        if let Some(hit) = self.hits.get(index) {
            let page = hit.page;
            self.current = Some(index);
            self.go(page);
        }
    }

    pub fn next_hit(&mut self) {
        if !self.hits.is_empty() {
            let next = self.current.map_or(0, |c| (c + 1) % self.hits.len());
            self.show(next);
        }
    }

    pub fn previous_hit(&mut self) {
        let count = self.hits.len();
        if count > 0 {
            let previous = self.current.map_or(0, |c| (c + count - 1) % count);
            self.show(previous);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(page: u32) -> Hit {
        Hit {
            page,
            areas: Vec::new(),
        }
    }

    #[test]
    fn pages_and_zoom_stay_within_their_ends() {
        let mut view = PdfView::new(3);
        view.previous_page();
        assert_eq!(view.page(), 0);
        view.go(99);
        view.next_page();
        assert_eq!(view.page(), 2);
        assert_eq!(view.zoom, Zoom::FitWidth);
        view.zoom_in();
        assert_eq!(view.zoom, Zoom::Percent(125));
        for _ in 0..20 {
            view.zoom_in();
        }
        assert_eq!(view.zoom, Zoom::Percent(400));
        for _ in 0..20 {
            view.zoom_out();
        }
        assert_eq!(view.zoom, Zoom::Percent(25));
    }

    #[test]
    fn a_search_moves_between_matches_and_old_answers_are_ignored() {
        let mut view = PdfView::new(5);
        let old = view.start_search("a");
        let search = view.start_search("ab");
        view.add_hits(old, vec![hit(4)]);
        assert!(view.hits().is_empty() && view.searching());
        view.add_hits(search, vec![hit(1), hit(3)]);
        assert_eq!((view.page(), view.current()), (1, Some(0)));
        view.next_hit();
        assert_eq!(view.page(), 3);
        view.next_hit();
        assert_eq!(view.page(), 1);
        view.previous_hit();
        assert_eq!(view.page(), 3);
        view.finish_search(old);
        assert!(view.searching());
        view.finish_search(search);
        assert!(!view.searching());
        view.start_search("");
        assert!(!view.searching() && view.hits().is_empty());
        view.next_hit();
        assert_eq!(view.current(), None);
    }

    #[test]
    fn the_size_drawn_is_bounded_whatever_the_document_says() {
        let spec = RenderSpec {
            zoom: Zoom::FitWidth,
            area_width: 600,
            pixels_per_point: 1.0,
            max_side: MAX_RENDER_SIDE,
        };
        let narrow = RenderSpec {
            max_side: 400,
            ..spec
        };
        assert_eq!(render_size((300.0, 400.0), narrow, 16_000_000), (300, 400));
        assert_eq!(render_size((300.0, 400.0), spec, 16_000_000), (600, 800));
        let big = RenderSpec {
            zoom: Zoom::Percent(400),
            ..spec
        };
        for points in [
            (1e9, 1e9),
            (f32::NAN, -1.0),
            (1.0, 100_000.0),
            (14_400.0, 14_400.0),
        ] {
            let (w, h) = render_size(points, big, 1_000_000);
            assert!(w >= 1 && h >= 1 && w <= MAX_RENDER_SIDE && h <= MAX_RENDER_SIDE);
            // a very thin page keeps one pixel on its narrow side
            assert!(
                u64::from(w) * u64::from(h) <= 1_000_000,
                "{points:?} {w}x{h}"
            );
        }
    }
}
