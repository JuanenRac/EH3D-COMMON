//! The state of the panel: what is shown, what waits for a person and what leaves the program.

mod support;

use std::sync::Arc;

use eh3d_viewer::outside::Outbox;
use eh3d_viewer::pdf::pins::{program_folder, Pins};
use eh3d_viewer::pdf::view::{RenderSpec, Zoom};
use eh3d_viewer::pdf::{Hit, PdfDocument};
use eh3d_viewer::picture::Raster;
use eh3d_viewer::texts::Text;
use eh3d_viewer::viewer::Action;
use eh3d_viewer::{Content, MemorySource, Viewer, ViewerError, ViewerEvent, ViewerLimits};
use support::{until, zip_of, Folder, Recorder};

fn setup(what: &str) -> (Viewer, Recorder, Folder) {
    let base = Folder::new(what);
    let recorder = Recorder::default();
    let viewer = Viewer::new(ViewerLimits::default())
        .with_outside(Box::new(recorder.clone()), Outbox::under(base.path()));
    (viewer, recorder, base)
}

fn open(viewer: &mut Viewer, name: &str, data: &[u8]) {
    viewer.open(Arc::new(MemorySource::new(name, data.to_vec())));
}

#[test]
fn showing_an_attachment_writes_nothing_and_starts_nothing() {
    let (mut viewer, recorder, base) = setup("show");
    open(&mut viewer, "a.txt", b"text");
    assert!(matches!(viewer.content(), Content::Text { text, .. } if text == "text"));
    open(&mut viewer, "a.md", b"# T");
    assert!(matches!(
        viewer.content(),
        Content::Document {
            text_only: false,
            ..
        }
    ));
    open(&mut viewer, "a.mp4", b"film");
    assert!(matches!(viewer.content(), Content::Outside(None)));
    open(&mut viewer, "a.xls", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1");
    assert!(matches!(
        viewer.content(),
        Content::Outside(Some(ViewerError::Unsupported))
    ));
    open(&mut viewer, "a.png", b"broken");
    assert!(matches!(
        viewer.content(),
        Content::Outside(Some(ViewerError::Damaged(_)))
    ));
    open(&mut viewer, "setup.exe", b"MZ\0");
    assert!(matches!(
        viewer.content(),
        Content::Withheld { has_preview: false }
    ));
    assert!(recorder.opened().is_empty());
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 0);
    viewer.close();
    assert!(matches!(viewer.content(), Content::Empty));
    assert_eq!(viewer.request(Action::OpenOutside), None);
}

#[test]
fn a_program_is_never_opened_and_is_saved_only_after_agreement() {
    let (mut viewer, recorder, base) = setup("program");
    open(&mut viewer, "setup.exe", b"MZ\0program");
    assert_eq!(viewer.warning(), Some(Text::WarnExecutable));
    assert_eq!(
        viewer.request(Action::OpenOutside),
        Some(ViewerEvent::Failed(ViewerError::Refused))
    );
    assert_eq!(viewer.notice(), Some(Text::Refused));
    assert_eq!(viewer.pending(), None);
    assert_eq!(viewer.confirm(), None);
    assert_eq!(viewer.request(Action::Preview), None);

    // without having asked, a path alone saves nothing
    let to = base.path().join("setup.exe");
    assert_eq!(
        viewer.save_to(&to),
        ViewerEvent::Failed(ViewerError::ConsentRequired)
    );
    assert_eq!(viewer.request(Action::SaveCopy), None);
    assert_eq!(viewer.pending(), Some(Action::SaveCopy));
    assert_eq!(
        viewer.confirm(),
        Some(ViewerEvent::ChooseSavePath {
            suggested: "setup.exe".into()
        })
    );
    assert_eq!(viewer.save_to(&to), ViewerEvent::Saved(to.clone()));
    assert_eq!(std::fs::read(&to).unwrap(), b"MZ\0program");
    // the agreement was for one copy
    assert_eq!(
        viewer.save_to(&base.path().join("again.exe")),
        ViewerEvent::Failed(ViewerError::ConsentRequired)
    );
    assert!(recorder.opened().is_empty());
}

#[test]
fn a_document_with_macros_waits_for_the_person_at_every_step() {
    let (mut viewer, recorder, _base) = setup("macros");
    let data = zip_of(&[
        ("word/document.xml", b"<w:p><w:t>inside</w:t></w:p>"),
        ("word/vbaProject.bin", b"code"),
    ]);
    open(&mut viewer, "report.docx", &data);
    assert_eq!(viewer.warning(), Some(Text::WarnMacros));
    assert!(matches!(
        viewer.content(),
        Content::Withheld { has_preview: true }
    ));
    assert_eq!(viewer.request(Action::Preview), None);
    assert!(matches!(viewer.content(), Content::Withheld { .. }));
    viewer.cancel();
    assert_eq!(viewer.confirm(), None);
    assert!(matches!(viewer.content(), Content::Withheld { .. }));
    viewer.request(Action::Preview);
    viewer.confirm();
    assert!(matches!(
        viewer.content(),
        Content::Document {
            text_only: true,
            ..
        }
    ));
    assert_eq!(viewer.request(Action::OpenOutside), None);
    assert!(recorder.opened().is_empty());
    let event = viewer.confirm();
    let opened = recorder.opened();
    assert_eq!(opened.len(), 1);
    assert_eq!(event, Some(ViewerEvent::OpenedOutside(opened[0].clone())));
    assert_eq!(std::fs::read(&opened[0]).unwrap(), data);
    assert_eq!(viewer.notice(), Some(Text::HandedOver));
}

#[test]
fn a_script_can_be_read_after_agreement_and_never_launched() {
    let (mut viewer, recorder, _base) = setup("script");
    open(&mut viewer, "notes.txt", b"#!/bin/sh\necho hi\n");
    assert_eq!(viewer.warning(), Some(Text::WarnScript));
    viewer.request(Action::Preview);
    viewer.confirm();
    assert!(matches!(viewer.content(), Content::Text { text, .. } if text.contains("echo hi")));
    assert_eq!(
        viewer.request(Action::OpenOutside),
        Some(ViewerEvent::Failed(ViewerError::Refused))
    );
    assert!(recorder.opened().is_empty());
}

#[test]
fn a_harmless_file_opens_outside_at_once_and_an_unknown_one_asks() {
    let (mut viewer, recorder, _base) = setup("harmless");
    open(&mut viewer, "film.mp4", b"film");
    assert_eq!(viewer.warning(), None);
    assert!(matches!(
        viewer.request(Action::OpenOutside),
        Some(ViewerEvent::OpenedOutside(_))
    ));
    open(&mut viewer, "data.xyz", b"??");
    assert_eq!(viewer.warning(), Some(Text::WarnUnknown));
    assert_eq!(viewer.request(Action::OpenOutside), None);
    assert_eq!(recorder.opened().len(), 1);
    viewer.confirm();
    assert_eq!(recorder.opened().len(), 2);
}

#[test]
fn an_attachment_over_the_limit_is_not_shown_nor_copied() {
    let base = Folder::new("limit");
    let recorder = Recorder::default();
    let limits = ViewerLimits {
        max_attachment_bytes: 8,
        ..ViewerLimits::default()
    };
    let mut viewer =
        Viewer::new(limits).with_outside(Box::new(recorder.clone()), Outbox::under(base.path()));
    open(&mut viewer, "a.txt", b"more than eight bytes");
    assert!(matches!(
        viewer.content(),
        Content::Outside(Some(ViewerError::TooLarge { limit: 8 }))
    ));
    assert_eq!(
        viewer.request(Action::OpenOutside),
        Some(ViewerEvent::Failed(ViewerError::TooLarge { limit: 8 }))
    );
    assert!(recorder.opened().is_empty());
}

/// Three pages; the word "pin" is on the last two.
struct Fake;

impl PdfDocument for Fake {
    fn page_count(&self) -> u32 {
        3
    }
    fn page_size(&self, _: u32) -> Result<(f32, f32), ViewerError> {
        Ok((100.0, 200.0))
    }
    fn render(&self, page: u32, width: u32, height: u32) -> Result<Raster, ViewerError> {
        Ok(Raster {
            width,
            height,
            rgba: vec![page as u8; width as usize * height as usize * 4],
        })
    }
    fn find(&self, page: u32, query: &str, max: usize) -> Result<Vec<Hit>, ViewerError> {
        let found = query == "pin" && page > 0 && max > 0;
        Ok(found
            .then(|| Hit {
                page,
                areas: Vec::new(),
            })
            .into_iter()
            .collect())
    }
}

fn fake(data: Vec<u8>) -> Result<Box<dyn PdfDocument>, ViewerError> {
    if data.starts_with(b"%PDF locked") {
        return Err(ViewerError::PasswordProtected);
    }
    Ok(Box::new(Fake))
}

#[test]
fn the_pages_zoom_and_search_of_a_pdf() {
    let mut viewer = Viewer::new(ViewerLimits::default()).with_pdf_engine(fake);
    open(&mut viewer, "a.pdf", b"%PDF");
    until("the document opens", || {
        viewer.poll();
        viewer.pdf_mut().is_some_and(|p| p.view.is_some())
    });
    let spec = RenderSpec {
        zoom: Zoom::FitWidth,
        area_width: 100,
        pixels_per_point: 1.0,
        max_side: 2048,
    };
    let settle = |viewer: &mut Viewer| {
        viewer.pdf_mut().unwrap().want(spec);
        until("the page is drawn", || {
            viewer.poll();
            !viewer.pdf_mut().unwrap().busy()
        });
    };
    settle(&mut viewer);
    let first = viewer.pdf_mut().unwrap().drawn.clone().unwrap();
    assert_eq!(
        (first.page, first.raster.width, first.raster.height),
        (0, 100, 200)
    );

    viewer.pdf_mut().unwrap().search("pin");
    until("the search ends", || {
        viewer.poll();
        !viewer.pdf_mut().unwrap().busy()
    });
    let view = viewer.pdf_mut().unwrap().view.clone().unwrap();
    assert_eq!((view.hits().len(), view.page()), (2, 1));
    settle(&mut viewer);
    assert_eq!(viewer.pdf_mut().unwrap().drawn.as_ref().unwrap().page, 1);

    // another zoom draws the same page again
    let before = viewer.pdf_mut().unwrap().drawn.as_ref().unwrap().ticket;
    viewer.pdf_mut().unwrap().view.as_mut().unwrap().zoom = Zoom::Percent(75);
    settle(&mut viewer);
    let after = viewer.pdf_mut().unwrap().drawn.clone().unwrap();
    assert!(after.ticket > before);
    assert_eq!((after.page, after.raster.width), (1, 100));
}

#[test]
fn a_pdf_that_cannot_be_opened_says_why() {
    let mut viewer = Viewer::new(ViewerLimits::default()).with_pdf_engine(fake);
    open(&mut viewer, "a.pdf", b"%PDF locked");
    until("the refusal arrives", || {
        viewer.poll();
        matches!(
            viewer.content(),
            Content::Outside(Some(ViewerError::PasswordProtected))
        )
    });
}

#[test]
fn without_the_library_beside_the_program_the_panel_says_so() {
    let pins = Pins::built_in().unwrap();
    let library = program_folder()
        .unwrap()
        .join(&pins.here().expect("pinned").library);
    if library.exists() {
        // the job of the CI that puts it there tests the real engine instead
        return;
    }
    let (mut viewer, recorder, _base) = setup("noengine");
    open(&mut viewer, "a.pdf", &support::pdf(&["x"]));
    until("the answer arrives", || {
        viewer.poll();
        matches!(
            viewer.content(),
            Content::Outside(Some(ViewerError::EngineMissing))
        )
    });
    // it can still be opened outside
    assert!(matches!(
        viewer.request(Action::OpenOutside),
        Some(ViewerEvent::OpenedOutside(_))
    ));
    assert_eq!(recorder.opened().len(), 1);
}
