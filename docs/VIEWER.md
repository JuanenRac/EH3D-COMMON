# The viewer of attachments (`eh3d-viewer`)

A project of the suite can carry datasheets, manuals and other files. This crate shows them inside a program and takes them out with care. Its state (`viewer::Viewer`) has no window in it; the panel drawn with egui is the feature `gui` (`gui::ViewerPanel`). The words are in the eight languages of the suite. The programs reach it through the panel of attachments ([ATTACHMENTS_PANEL.md](ATTACHMENTS_PANEL.md)). What a file is and whether it is dangerous is decided by `eh3d-filekind`, the one classification of the suite.

Every attachment is untrusted input. Nothing of an attachment is ever run, nothing is downloaded and no link is followed.

## What is shown

| Files | Inside the program |
|---|---|
| PDF | Pages drawn by PDFium, with page buttons, zoom, fit width and search with the matches marked |
| PNG, JPEG, GIF (first image), BMP, WebP | The picture |
| Text, logs, G-code, Gerber, source and settings files | The text (the first 2 MB) |
| Markdown | Headings, paragraphs, lists, quotes, code and tables; pictures and links are not fetched |
| CSV, TSV | A table |
| docx, xlsx, pptx, odt, ods, odp, rtf | The text and the tables only, without formatting |
| doc, xls, ppt (before 2007), video, audio, design files, anything else | No preview: open outside |

## Dangerous files

| What | How it is recognised | What can be done |
|---|---|---|
| Programs | Extension, or the first bytes (Windows, Linux and macOS executables), whatever the name says | Never launched. A copy can be saved after confirming |
| Scripts | Extension, or a first line `#!` | Never launched. Read as text, or saved, after confirming |
| Office files with macros | Extension, or macros found inside a file with a harmless extension | Preview, open outside or save after confirming |
| Compressed archives | Extension or first bytes; they are not looked into | Open outside or save after confirming |
| HTML and SVG | Extension | Read as text, open outside or save after confirming |
| Unknown kinds | No known extension | Open outside, extract or save after confirming |

A dangerous file shows only its warning until the person confirms. A confirmation is good for one action.

## Taking a file out

* **Open outside** writes a copy in a temporary folder of its own, private to the user, under a cleaned name (no path, no control or text-reordering characters, no reserved Windows name), never executable, marked on Windows as coming from outside, and hands it to the opener of the system (`explorer.exe`, `open`, `xdg-open`) by full path, never through a shell. The folder is removed when the viewer is dropped, as far as the system lets go of the files.
* **Save a copy** never replaces a file that is there and leaves no partial file.

## Limits

An attachment may be up to **1 GB**. All limits are in `ViewerLimits`; a program passes the lines of its settings file to `ViewerLimits::apply`.

| Setting | Default | Meaning |
|---|---|---|
| `viewer.max_attachment_mb` | 1024 | Largest attachment shown, opened or saved |
| `viewer.max_pdf_mb` | 256 | Largest PDF shown inside (it is held in memory) |
| `viewer.max_image_mb` | 64 | Largest picture file shown (also at most 48 million pixels) |
| `viewer.max_text_mb` | 2 | Bytes of a text file shown; the rest is cut |
| `viewer.max_office_mb` | 128 | Largest office document previewed |

Office archives have more limits that are not settings: 32 MB unpacked for one part, 96 MB for all the parts read, 20,000 entries, 20,000 paragraphs and rows, 100,000 cells and 256 columns. A document type declaration is refused, so there are no external or expanding entities; external relationships are not followed.

## PDFium

The library is not compiled with the programs. It is put beside the executable of each program, from the build pinned in `crates/viewer/pdfium.json` (release, version, SHA-256 of every archive and library), which has no JavaScript engine and no XFA:

```
python tools/fetch_pdfium.py --platform windows-x86_64 --out <program folder>
```

The platforms are `windows-x86_64`, `windows-x86`, `windows-aarch64`, `linux-x86_64`, `linux-aarch64`, `macos-x86_64` and `macos-aarch64`. The program loads it **only from its own folder**, never from the current folder or the search path, and only if its size and SHA-256 are the pinned ones. When it is missing or different, the panel says so and offers to open the PDF outside. The library adds about 7 to 8 MB to each program (about 4 MB packed). Its licences, in `pdfium-licenses`, ship with it (`crates/viewer/THIRD_PARTY_NOTICES`).

To change the build: take a release of the same project, check it (`gh release verify`), and write the new release, version, sizes and sums in `pdfium.json`.

PDFium reads the document inside the process of the program, on a thread of its own; it is not in a separate sandbox. Keeping it up to date is part of the maintenance of the programs.

## In a program

```rust,ignore
use std::sync::Arc;
use eh3d_viewer::{gui::ViewerPanel, FileSource, ViewerEvent, ViewerLimits};

let mut panel = ViewerPanel::new(ViewerLimits::default());
panel.viewer.open(Arc::new(FileSource::new(path)?));
// inside the dock panel, on every frame:
if let Some(ViewerEvent::ChooseSavePath { suggested }) = panel.show(ui, lang) {
    // the Save window of the program, then:
    panel.viewer.save_to(&chosen);
}
```

An attachment that is not a file on disk implements the trait `Source` (a name, a length and a reader), so a program can hand over an entry of its project without writing it first. Keep one panel for as long as the program runs: the temporary copies live as long as it does. For Chinese and Japanese the program installs a font that has those characters, as for the rest of its window.

## Not done

No preview of the binary office formats before 2007, of vector graphics, of 3D files, of video or audio. The text of a PDF cannot be selected or copied, and a PDF that asks for a password is only opened outside.

## Tests

`cargo test -p eh3d-viewer --features gui` uses synthetic files only. The tests of the real library run in the CI, on Linux, Windows (64 and 32 bits) and macOS: `tools/fetch_pdfium.py` puts it beside the test programs and `EH3D_PDFIUM_REQUIRED` is set.
