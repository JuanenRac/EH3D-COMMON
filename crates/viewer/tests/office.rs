//! The previews of office documents, from synthetic files, the hostile ones included.

mod support;

use eh3d_viewer::kind::OfficeKind;
use eh3d_viewer::office::preview;
use eh3d_viewer::{Block, Preview, ViewerError, ViewerLimits};
use support::zip_of;

fn read(kind: OfficeKind, entries: &[(&str, &[u8])]) -> Result<Preview, ViewerError> {
    preview(kind, &zip_of(entries), &ViewerLimits::default())
}

fn rows(cells: &[&[&str]]) -> Block {
    Block::Table(
        cells
            .iter()
            .map(|r| r.iter().map(|c| c.to_string()).collect())
            .collect(),
    )
}

fn paragraph(text: &str) -> Block {
    Block::Paragraph(text.into())
}

fn heading(level: u8, text: &str) -> Block {
    Block::Heading(level, text.into())
}

#[test]
fn a_word_document_gives_headings_text_lists_and_tables() {
    let xml = br#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Ratings</w:t></w:r></w:p>
<w:p><w:r><w:t xml:space="preserve">Up to </w:t></w:r><w:r><w:t>5 V &amp; 1 A</w:t><w:tab/><w:t>max</w:t></w:r></w:p>
<w:p><w:pPr><w:numPr/></w:pPr><w:r><w:t>first</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>pin</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>name</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p/><w:p><w:r><w:t>VCC</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:instrText>not shown</w:instrText></w:r></w:p>
</w:body></w:document>"#;
    let got = read(OfficeKind::Docx, &[("word/document.xml", xml)]).unwrap();
    assert_eq!(
        got.blocks,
        vec![
            heading(2, "Ratings"),
            paragraph("Up to 5 V & 1 A\tmax"),
            Block::Item(0, "first".into()),
            rows(&[&["pin", "name"], &["1", "VCC"]]),
        ]
    );
    assert!(!got.truncated);
}

#[test]
fn a_workbook_gives_one_table_per_sheet_in_the_order_of_the_book() {
    let workbook = br#"<workbook><sheets><sheet name="Parts" sheetId="1" r:id="b"/><sheet name="Two" sheetId="2" r:id="a"/></sheets></workbook>"#;
    let rels = br#"<Relationships><Relationship Id="a" Target="worksheets/sheet1.xml"/><Relationship Id="b" Target="/xl/worksheets/sheet2.xml"/></Relationships>"#;
    let strings = br#"<sst><si><t>ref</t></si><si><r><t>val</t></r><r><t>ue</t></r><rPh><t>x</t></rPh></si></sst>"#;
    let first = br#"<worksheet><sheetData><row><c r="A1" t="s"><v>0</v></c><c r="C1" t="s"><v>1</v></c></row>
<row><c r="A2" t="inlineStr"><is><t>R1</t></is></c><c r="B2" t="b"><v>1</v></c><c r="C2"><f>1+1</f><v>4700</v></c></row></sheetData></worksheet>"#;
    let second = br#"<worksheet><sheetData><row><c t="str"><v>only</v></c><c t="s"><v>99</v></c></row></sheetData></worksheet>"#;
    let got = read(
        OfficeKind::Xlsx,
        &[
            ("xl/workbook.xml", workbook),
            ("xl/_rels/workbook.xml.rels", rels),
            ("xl/sharedStrings.xml", strings),
            ("xl/worksheets/sheet2.xml", first),
            ("xl/worksheets/sheet1.xml", second),
        ],
    )
    .unwrap();
    assert_eq!(
        got.blocks,
        vec![
            heading(2, "Parts"),
            rows(&[&["ref", "", "value"], &["R1", "TRUE", "4700"]]),
            heading(2, "Two"),
            rows(&[&["only", ""]]),
        ]
    );
}

#[test]
fn a_presentation_gives_its_slides_in_their_order() {
    let presentation = br#"<p:presentation><p:sldIdLst><p:sldId id="1" r:id="r2"/><p:sldId id="2" r:id="r1"/></p:sldIdLst></p:presentation>"#;
    let rels = br#"<Relationships><Relationship Id="r1" Target="slides/slide1.xml"/><Relationship Id="r2" Target="slides/slide2.xml"/></Relationships>"#;
    let one = br#"<p:sld><a:p><a:r><a:t>second shown</a:t></a:r></a:p></p:sld>"#;
    let two = br#"<p:sld><a:p><a:r><a:t>first shown</a:t></a:r></a:p></p:sld>"#;
    let got = read(
        OfficeKind::Pptx,
        &[
            ("ppt/presentation.xml", presentation),
            ("ppt/_rels/presentation.xml.rels", rels),
            ("ppt/slides/slide1.xml", one),
            ("ppt/slides/slide2.xml", two),
        ],
    )
    .unwrap();
    assert_eq!(
        got.blocks,
        vec![
            heading(2, "1"),
            paragraph("first shown"),
            heading(2, "2"),
            paragraph("second shown"),
        ]
    );
}

#[test]
fn the_documents_of_libreoffice() {
    let text = br#"<office:document-content><office:body><office:text>
<text:h text:outline-level="3">Title</text:h>
<text:p>a<text:s text:c="3"/><text:span>b</text:span><text:line-break/>c</text:p>
</office:text></office:body></office:document-content>"#;
    let got = read(OfficeKind::Odt, &[("content.xml", text)]).unwrap();
    assert_eq!(got.blocks, vec![heading(3, "Title"), paragraph("a   b\nc")]);

    let sheet = br#"<o><table:table table:name="S">
<table:table-row><table:table-cell><text:p>x</text:p></table:table-cell><table:table-cell table:number-columns-repeated="2"/><table:table-cell table:number-columns-repeated="2"><text:p>y</text:p></table:table-cell><table:table-cell table:number-columns-repeated="16384"/></table:table-row>
<table:table-row table:number-rows-repeated="1048576"><table:table-cell table:number-columns-repeated="16384"/></table:table-row>
</table:table></o>"#;
    let got = read(OfficeKind::Ods, &[("content.xml", sheet)]).unwrap();
    assert_eq!(
        got.blocks,
        vec![heading(2, "S"), rows(&[&["x", "", "", "y", "y"]])]
    );

    let slides = br#"<o><draw:page><draw:frame><text:p>one</text:p></draw:frame></draw:page><draw:page><text:p>two</text:p></draw:page></o>"#;
    let got = read(OfficeKind::Odp, &[("content.xml", slides)]).unwrap();
    assert_eq!(
        got.blocks,
        vec![
            heading(2, "1"),
            paragraph("one"),
            heading(2, "2"),
            paragraph("two")
        ]
    );
}

#[test]
fn huge_repetitions_are_bounded() {
    let sheet = br#"<o><table:table><table:table-row table:number-rows-repeated="999999999"><table:table-cell table:number-columns-repeated="999999999"><text:p>z<text:s text:c="999999999"/></text:p></table:table-cell></table:table-row></table:table></o>"#;
    let got = read(OfficeKind::Ods, &[("content.xml", sheet)]).unwrap();
    assert!(got.truncated);
    match &got.blocks[..] {
        [Block::Table(table)] => {
            assert_eq!((table.len(), table[0].len()), (64, 256));
            assert_eq!(table[0][0].len(), 201);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn rich_text_format_is_read_as_text() {
    let rtf = br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\*\generator x;}\f0 Caf\'e9 \{ok\}\par \u937?\tab end{\pict 0102}}";
    let got = preview(OfficeKind::Rtf, rtf, &ViewerLimits::default()).unwrap();
    assert_eq!(
        got.blocks,
        vec![paragraph("Café {ok}"), paragraph("Ω\tend")]
    );
    assert!(preview(OfficeKind::Rtf, b"plain", &ViewerLimits::default()).is_err());
    let deep = [b"{\\rtf1".as_slice(), "{".repeat(10_000).as_bytes()].concat();
    assert!(preview(OfficeKind::Rtf, &deep, &ViewerLimits::default()).is_err());
    // every cut of a document, and a binary run longer than the file
    for end in 5..rtf.len() {
        let _ = preview(OfficeKind::Rtf, &rtf[..end], &ViewerLimits::default());
    }
    let long = br"{\rtf1 \bin999999999 x}";
    assert!(preview(OfficeKind::Rtf, long, &ViewerLimits::default()).is_ok());
}

#[test]
fn a_part_that_unpacks_to_too_much_is_stopped() {
    let zeros = vec![b' '; 3_000_000];
    let bomb = zip_of(&[("content.xml", &zeros)]);
    assert!(bomb.len() < 20_000, "the sample is a small archive");
    let limits = ViewerLimits {
        max_office_part_bytes: 1_000_000,
        ..ViewerLimits::default()
    };
    assert_eq!(
        preview(OfficeKind::Odt, &bomb, &limits),
        Err(ViewerError::TooLarge { limit: 1_000_000 })
    );
}

#[test]
fn the_parts_together_have_a_limit_too() {
    let filler = vec![b' '; 600_000];
    let presentation = br#"<p><sldId r:id="a"/><sldId r:id="b"/></p>"#;
    let rels = br#"<R><Relationship Id="a" Target="slides/slide1.xml"/><Relationship Id="b" Target="slides/slide2.xml"/></R>"#;
    let data = zip_of(&[
        ("ppt/presentation.xml", presentation),
        ("ppt/_rels/presentation.xml.rels", rels),
        ("ppt/slides/slide1.xml", &filler),
        ("ppt/slides/slide2.xml", &filler),
    ]);
    let limits = ViewerLimits {
        max_office_total_bytes: 1_000_000,
        ..ViewerLimits::default()
    };
    assert!(matches!(
        preview(OfficeKind::Pptx, &data, &limits),
        Err(ViewerError::TooLarge { .. })
    ));
}

#[test]
fn too_many_entries_document_types_and_broken_archives_are_refused() {
    let limits = ViewerLimits {
        max_office_entries: 2,
        ..ViewerLimits::default()
    };
    let many = zip_of(&[("a", b""), ("b", b""), ("content.xml", b"<a/>")]);
    assert!(matches!(
        preview(OfficeKind::Odt, &many, &limits),
        Err(ViewerError::TooLarge { .. })
    ));
    let entity =
        br#"<!DOCTYPE d [<!ENTITY e SYSTEM "file:///etc/passwd">]><d><text:p>&e;</text:p></d>"#;
    assert!(matches!(
        read(OfficeKind::Odt, &[("content.xml", entity)]),
        Err(ViewerError::Damaged(_))
    ));
    assert!(matches!(
        read(OfficeKind::Docx, &[("other.xml", b"<a/>")]),
        Err(ViewerError::Damaged(_))
    ));
    assert!(matches!(
        preview(
            OfficeKind::Docx,
            b"PK\x03\x04junk",
            &ViewerLimits::default()
        ),
        Err(ViewerError::Damaged(_))
    ));
    assert_eq!(
        preview(OfficeKind::Legacy, b"\xd0\xcf", &ViewerLimits::default()),
        Err(ViewerError::Unsupported)
    );
}

#[test]
fn a_relationship_that_points_outside_is_not_followed() {
    let workbook = br#"<workbook><sheet name="S" r:id="a"/></workbook>"#;
    let rels = br#"<Relationships><Relationship Id="a" Target="http://example.org/sheet.xml" TargetMode="External"/></Relationships>"#;
    let got = read(
        OfficeKind::Xlsx,
        &[
            ("xl/workbook.xml", workbook),
            ("xl/_rels/workbook.xml.rels", rels),
        ],
    )
    .unwrap();
    assert_eq!(got.blocks, vec![heading(2, "S")]);
}

#[test]
fn damaged_documents_never_stop_the_reader() {
    let xml = br#"<w:document><w:tbl><w:tr><w:tc><w:p><w:t>a</w:t></w:p></w:tc></w:tr></w:tbl></w:document>"#;
    let sound = zip_of(&[("word/document.xml", xml)]);
    // a fixed seed: the same damage on every run
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move |bound: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % bound as u64) as usize
    };
    for _ in 0..400 {
        let mut data = sound.clone();
        for _ in 0..1 + next(4) {
            let at = next(data.len());
            data[at] ^= 1 << next(8);
        }
        data.truncate(1 + next(data.len()));
        let _ = preview(OfficeKind::Docx, &data, &ViewerLimits::default());
    }
    for end in 0..xml.len() {
        let _ = read(OfficeKind::Docx, &[("word/document.xml", &xml[..end])]);
    }
}
