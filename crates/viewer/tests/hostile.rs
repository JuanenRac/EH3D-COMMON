//! Documents made to use up the memory or the time of the viewer, and damaged ones. Every file is made here.

mod support;

use eh3d_viewer::kind::OfficeKind;
use eh3d_viewer::office::preview;
use eh3d_viewer::{markdown, Block, Preview, ViewerLimits};
use support::zip_of;

/// The bytes of text a preview holds.
fn held(preview: &Preview) -> usize {
    preview
        .blocks
        .iter()
        .map(|block| match block {
            Block::Heading(_, t)
            | Block::Paragraph(t)
            | Block::Item(_, t)
            | Block::Code(t)
            | Block::Quote(t) => t.capacity(),
            Block::Table(rows) => rows.iter().flatten().map(String::capacity).sum(),
            Block::Rule => 0,
        })
        .sum()
}

const MOST: usize = 16 * 1024 * 1024;

#[test]
fn one_long_value_shown_in_many_cells_of_a_workbook_is_bounded() {
    let workbook = br#"<workbook><sheets><sheet name="a" r:id="a"/></sheets></workbook>"#;
    let rels =
        br#"<Relationships><Relationship Id="a" Target="worksheets/sheet1.xml"/></Relationships>"#;
    let strings = format!("<sst><si><t>{}</t></si></sst>", "v".repeat(90_000));
    let row = format!("<row>{}</row>", r#"<c t="s"><v>0</v></c>"#.repeat(200));
    let sheet = format!(
        "<worksheet><sheetData>{}</sheetData></worksheet>",
        row.repeat(400)
    );
    let got = preview(
        OfficeKind::Xlsx,
        &zip_of(&[
            ("xl/workbook.xml", workbook),
            ("xl/_rels/workbook.xml.rels", rels),
            ("xl/sharedStrings.xml", strings.as_bytes()),
            ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
        ]),
        &ViewerLimits::default(),
    )
    .unwrap();
    assert!(got.truncated);
    assert!(held(&got) <= MOST, "{}", held(&got));
    assert!(held(&got) > 1024 * 1024, "the beginning is shown");
}

#[test]
fn one_long_value_repeated_in_a_sheet_of_libreoffice_is_bounded() {
    let cell = format!(
        r#"<table:table-cell table:number-columns-repeated="1000000"><text:p>{}</text:p></table:table-cell>"#,
        "w".repeat(2_000_000)
    );
    let row = format!(
        r#"<table:table-row table:number-rows-repeated="1000000">{cell}</table:table-row>"#
    );
    let content = format!(
        r#"<office:document-content><table:table table:name="a">{}</table:table></office:document-content>"#,
        row.repeat(4)
    );
    let got = preview(
        OfficeKind::Ods,
        &zip_of(&[("content.xml", content.as_bytes())]),
        &ViewerLimits::default(),
    )
    .unwrap();
    assert!(got.truncated);
    assert!(held(&got) <= MOST, "{}", held(&got));
}

#[test]
fn a_long_run_of_text_is_cut_at_a_character() {
    let xml = format!(
        "<w:document><w:body><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:body></w:document>",
        "é日".repeat(300_000)
    );
    let got = preview(
        OfficeKind::Docx,
        &zip_of(&[("word/document.xml", xml.as_bytes())]),
        &ViewerLimits::default(),
    )
    .unwrap();
    assert!(got.truncated);
    match &got.blocks[..] {
        [Block::Paragraph(text)] => assert_eq!(text.chars().count(), 20_000),
        other => panic!("{} blocks", other.len()),
    }
}

#[test]
fn a_markdown_file_of_open_brackets_ends() {
    let limits = ViewerLimits::default();
    for piece in ["[", "![", "[a](", "[](", "a`", "[é日"] {
        let text = piece.repeat(300_000 / piece.len());
        let got = markdown::preview(text.as_bytes(), &limits);
        assert!(got.truncated, "{piece}");
    }
    // and links are still written out
    let got = markdown::preview(b"[a](b) ![c](d) [[e](f)", &limits);
    assert_eq!(got.blocks, [Block::Paragraph("a (b) [c] [e (f)".into())]);
}

#[test]
fn damaged_office_documents_never_stop_the_viewer() {
    let files: &[(OfficeKind, &str)] = &[
        (OfficeKind::Docx, "word/document.xml"),
        (OfficeKind::Xlsx, "xl/workbook.xml"),
        (OfficeKind::Pptx, "ppt/presentation.xml"),
        (OfficeKind::Odt, "content.xml"),
    ];
    let limits = ViewerLimits::default();
    let mut state = 0x5EED_0003u64;
    let mut below = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    for (kind, part) in files {
        let xml = br#"<a><w:p><w:r><w:t>x &amp; y</w:t></w:r></w:p><table:table-cell/></a>"#;
        let whole = zip_of(&[(part, xml)]);
        for cut in 0..whole.len() {
            assert!(preview(*kind, &whole[..cut], &limits).is_err(), "cut {cut}");
        }
        for _ in 0..1_000 {
            let mut bytes = whole.clone();
            for _ in 0..1 + below(6) {
                let at = below(bytes.len());
                bytes[at] = below(256) as u8;
            }
            let _ = preview(*kind, &bytes, &limits);
        }
    }
    let rtf = br#"{\rtf1\ansi{\fonttbl{\f0 A;}}\u8364?\'e9\bin5 abcde{\*\x y}\par end}"#;
    for cut in 0..rtf.len() {
        let _ = preview(OfficeKind::Rtf, &rtf[..cut], &limits);
    }
    for _ in 0..3_000 {
        let mut bytes = rtf.to_vec();
        for _ in 0..1 + below(6) {
            let at = below(bytes.len());
            bytes[at] = below(256) as u8;
        }
        let _ = preview(OfficeKind::Rtf, &bytes, &limits);
    }
}
