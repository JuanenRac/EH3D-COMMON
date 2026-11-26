//! The real library on a synthetic document. It runs when the pinned library has been put beside the
//! test program (`tools/fetch_pdfium.py`) and `EH3D_PDFIUM_REQUIRED` is set, as the CI does; without
//! the variable it does nothing.

mod support;

use std::sync::Arc;

use eh3d_viewer::pdf::pdfium::PdfiumDocument;
use eh3d_viewer::pdf::view::{RenderSpec, Zoom};
use eh3d_viewer::pdf::PdfDocument;
use eh3d_viewer::{Content, MemorySource, Viewer, ViewerError, ViewerLimits};
use support::{pdf, until};

fn required() -> bool {
    std::env::var_os("EH3D_PDFIUM_REQUIRED").is_some()
}

#[test]
fn pages_are_measured_drawn_and_searched() {
    if !required() {
        return;
    }
    let document = PdfiumDocument::open(pdf(&["Hello datasheet", "second page"])).unwrap();
    assert_eq!(document.page_count(), 2);
    assert_eq!(document.page_size(0).unwrap(), (200.0, 100.0));
    let page = document.render(0, 400, 200).unwrap();
    assert_eq!(page.rgba.len(), 400 * 200 * 4);
    assert_eq!(&page.rgba[..4], &[255, 255, 255, 255], "white paper");
    let ink = page.rgba.chunks(4).filter(|p| p[0] < 128).count();
    assert!(ink > 50, "the text is drawn: {ink} dark pixels");

    // upper and lower case are the same
    let hits = document.find(0, "DATASHEET", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].page, 0);
    let area = hits[0].areas[0];
    // the text starts 20 of 200 points from the left and stands on a line half way up
    assert!(
        area.left > 0.1 && area.right < 0.9 && area.left < area.right,
        "{area:?}"
    );
    assert!(
        area.top > 0.2 && area.bottom < 0.7 && area.top < area.bottom,
        "{area:?}"
    );
    assert!(document.find(1, "datasheet", 10).unwrap().is_empty());
    assert_eq!(document.find(1, "second", 10).unwrap().len(), 1);
    assert!(document.find(0, "", 10).unwrap().is_empty());
    assert_eq!(document.find(0, "e", 1).unwrap().len(), 1);

    assert!(matches!(
        document.render(2, 10, 10),
        Err(ViewerError::Damaged(_))
    ));
    assert!(matches!(
        document.render(0, 100_000, 10),
        Err(ViewerError::TooLarge { .. })
    ));
}

#[test]
fn damaged_documents_are_refused_and_never_stop_the_program() {
    if !required() {
        return;
    }
    assert!(matches!(
        PdfiumDocument::open(b"not a pdf at all".to_vec()),
        Err(ViewerError::Damaged(_))
    ));
    assert!(PdfiumDocument::open(Vec::new()).is_err());
    let sound = pdf(&["text"]);
    // every tenth cut, and bytes changed with a fixed seed
    for end in (0..sound.len()).step_by(10) {
        if let Ok(document) = PdfiumDocument::open(sound[..end].to_vec()) {
            let _ = document.render(0, 50, 50);
        }
    }
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..200 {
        let mut data = sound.clone();
        for _ in 0..4 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let at = (seed % data.len() as u64) as usize;
            data[at] = (seed >> 40) as u8;
        }
        if let Ok(document) = PdfiumDocument::open(data) {
            let _ = document.render(0, 50, 50);
            let _ = document.find(0, "text", 5);
        }
    }
}

#[test]
fn the_viewer_shows_a_page_with_the_library_beside_the_program() {
    if !required() {
        return;
    }
    let mut viewer = Viewer::new(ViewerLimits::default());
    viewer.open(Arc::new(MemorySource::new("a.pdf", pdf(&["one", "two"]))));
    until("the document opens", || {
        viewer.poll();
        assert!(
            matches!(viewer.content(), Content::Pdf(_)),
            "the library was not accepted"
        );
        viewer.pdf_mut().is_some_and(|p| p.view.is_some())
    });
    viewer.pdf_mut().unwrap().want(RenderSpec {
        zoom: Zoom::FitWidth,
        area_width: 300,
        pixels_per_point: 1.0,
        max_side: 2048,
    });
    until("a page is drawn", || {
        viewer.poll();
        viewer.pdf_mut().is_some_and(|p| p.drawn.is_some())
    });
    let drawn = viewer.pdf_mut().unwrap().drawn.clone().unwrap();
    assert_eq!((drawn.raster.width, drawn.raster.height), (300, 150));
    viewer.pdf_mut().unwrap().search("two");
    until("the match is found", || {
        viewer.poll();
        let view = viewer.pdf_mut().unwrap().view.as_ref().unwrap();
        !view.searching() && view.hits().len() == 1 && view.page() == 1
    });
}

#[test]
fn a_damaged_document_is_refused_or_read_but_never_stops_the_program() {
    if !required() {
        return;
    }
    let whole = pdf(&["Hello datasheet", "second page"]);
    let try_it = |bytes: Vec<u8>| {
        let Ok(document) = PdfiumDocument::open(bytes) else {
            return;
        };
        for page in 0..document.page_count().min(4) {
            let _ = document.page_size(page);
            let _ = document.render(page, 40, 20);
            let _ = document.find(page, "e", 4);
        }
    };
    for cut in (0..whole.len()).step_by(7) {
        try_it(whole[..cut].to_vec());
    }
    let mut state = 0x5EED_0005u64;
    let mut below = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    for _ in 0..300 {
        let mut bytes = whole.clone();
        for _ in 0..1 + below(8) {
            let at = below(bytes.len());
            bytes[at] = below(256) as u8;
        }
        try_it(bytes);
    }
}
