# The panel «Attachments» (`eh3d-attachpanel`)

The one panel a PHOENIX program mounts for the attachments of its `.eh3d` project. It holds the list, attaching a file or a link, the links to components, extracting and removing, the viewer (`eh3d-viewer`) beside the list, and its own window to choose files and folders. Its model (`model::Model`) has no window and is tested without a screen; the drawing with egui is the feature `gui`. The words are in the eight languages of the suite.

## What it does

* **List** every attachment with its size, whether it is inside or only a link, and a mark when it needs care.
* **Attach** a file or a link (address and SHA-256; the hash can be taken from a file the person has). A dangerous file is kept and marked. A settings file that gives a value to `server_token` is refused, because a project is a file that is passed on.
* **Link to a component** by designator, value or part number (with or without maker), and remove the link. A program lists the attachments of one part with `Model::set_filter`.
* **Extract** into a folder, after a confirmation for a dangerous or unknown file; never over a file that is there.
* **Remove** from the project, always after a confirmation.
* **Show** the chosen attachment inside; opening outside and saving a copy follow the rules of the viewer.

`Escape` closes what the panel has open, one thing at each press and nothing written: the window that chooses, then the question before a write (answered with no), then the form of a link. With nothing open, and while a list drops down, the key is left to the program.

A write that removes a signature of the project or of the branch of attachments is asked about first. Only the branch `attachments/` is written; the branches of the programs keep their bytes. The work runs on a thread of its own, and the panel reads the project again when another program wrote it. Nothing of an attachment is run and nothing is downloaded.

## Mounting it

`Cargo.toml` of the program:

```toml
[features]
gui = [..., "eh3d-attachpanel/gui"]

[dependencies]
eh3d-attachpanel = { version = ">=0.0.1", path = "../EH3D-COMMON/crates/attachpanel" }
```

The panels and the menu (no window needed):

```rust,ignore
defs.push(eh3d_attachpanel::panel_def());               // id eh3d_attachpanel::PANEL_ID
// add the id to a factory workspace, and in File > Suite project:
Entry::dock("file.suite.attachments", Label::Own(eh3d_attachpanel::panel_title()),
            DockAction::ShowPanel(eh3d_attachpanel::PANEL_ID.to_string()))
```

The window:

```rust,ignore
// kept for as long as the program runs (the temporary copies live as long as it does)
// and given the limits of the settings file: attachments.set_limits(PanelLimits::from_text(&settings_text));
let mut attachments = AttachPanel::new(Program::new("cnc", env!("CARGO_PKG_VERSION")));

// the body registered for PANEL_ID
if attachments.show(ui, project.as_deref(), lang_code) {
    // the project was written: read your own state of it again
}
```

A program whose window has no project of the suite uses `AttachPanel::new(..).opens_projects()`: the panel then has its own button to open one, and `choose_project()` does it from a menu.

### The limits of the settings

A program keeps the `key = value` lines of its settings file whose key starts with `attachments.` or `viewer.` (`PanelLimits::is_key`) and gives the panel what they say, whenever it reads its settings: `attachments.set_limits(PanelLimits::from_text(&text))`. Nothing happens when the limits are the ones in force. `apply_setting(key, value)` takes one line at a time. The limits come from the settings of the person, never from a file that travels inside a project.

| Key | Default | Meaning |
|---|---|---|
| `attachments.max_mb` | 1024 | largest file that is attached or taken out |
| `viewer.max_attachment_mb` | 1024 | largest attachment shown, opened or saved |
| `viewer.max_pdf_mb` | 256 | largest PDF shown inside |
| `viewer.max_image_mb` | 64 | largest picture file shown |
| `viewer.max_text_mb` | 2 | bytes of a text file shown |
| `viewer.max_office_mb` | 128 | largest office document previewed |

Each value is a whole number from 1 to 4096.

### Files dropped on the window

Instead of reading the dropped files itself, the program asks the panel at the start of every frame, and gets back the ones that are its own:

```rust,ignore
let dropped = attachments.take_drop(ctx, |path| the_program_opens(path));
```

The panel takes a file when it was drawn on the last frame with a project and the pointer is over it; it is then attached like a chosen one. Many systems (Windows among them) do not say where the pointer is while a file is dragged: the panel then takes only the files the program does not open, and asks before it attaches each one. Folders are never taken.

The SHA-256 of a file chosen for a link is taken on a thread of its own, so a large file does not stop the window.

## For the programs that do not have it yet

* **EH3D-PHOENIX-CAD:** the same four steps with `Program::new("cad", ..)` and the project of its window; without a project menu, `opens_projects()`.
* **EH3D-PHOENIX-PCB:** the same steps with `Program::new("pcb", ..)`. From a part of the design, show the panel and call `attachments.model.set_filter(Some(piece))` with the designator, value, part number and maker of that part (`Piece`), so `R5` or `U3` lists its data sheets; linking the chosen attachment to the part is the form of the panel.

## The PDF library

PDF pages need PDFium beside the executable: `tools/fetch_pdfium.py --platform <platform> --out <folder>` puts the pinned library and its licences there (see [VIEWER.md](VIEWER.md)). A package of a program carries that folder, the library as it is published (it is never signed again) and `crates/viewer/THIRD_PARTY_NOTICES`. Without it the panel says so and offers to open the file outside.

## Not done

Downloading a link (a person does it outside). Dropping a file has been tried only with made-up events, not by dragging on a desktop of each system.
