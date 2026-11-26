# The panel «Attachments» (`eh3d-attachpanel`)

The one panel a PHOENIX program mounts for the attachments of its `.eh3d` project. It holds the list, attaching a file or a link, the links to components, extracting and removing, the viewer (`eh3d-viewer`) beside the list, and its own window to choose files and folders. Its model (`model::Model`) has no window and is tested without a screen; the drawing with egui is the feature `gui`. The words are in the eight languages of the suite.

## What it does

* **List** every attachment with its size, whether it is inside or only a link, and a mark when it needs care.
* **Attach** a file or a link (address and SHA-256; the hash can be taken from a file the person has). A dangerous file is kept and marked. A settings file that gives a value to `server_token` is refused, because a project is a file that is passed on.
* **Link to a component** by designator, value or part number (with or without maker), and remove the link. A program lists the attachments of one part with `Model::set_filter`.
* **Extract** into a folder, after a confirmation for a dangerous or unknown file; never over a file that is there.
* **Remove** from the project, always after a confirmation.
* **Show** the chosen attachment inside; opening outside and saving a copy follow the rules of the viewer.

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
let mut attachments = AttachPanel::new(Program::new("cnc", env!("CARGO_PKG_VERSION")));

// the body registered for PANEL_ID
if attachments.show(ui, project.as_deref(), lang_code) {
    // the project was written: read your own state of it again
}
```

A program whose window has no project of the suite uses `AttachPanel::new(..).opens_projects()`: the panel then has its own button to open one, and `choose_project()` does it from a menu.

`apply_setting(key, value)` takes the limits from the settings of the program (`attachments.max_mb`, `viewer.max_*_mb`), before the first frame.

## For the programs that do not have it yet

* **EH3D-PHOENIX-CAD:** the same four steps with `Program::new("cad", ..)` and the project of its window; without a project menu, `opens_projects()`.
* **EH3D-PHOENIX-PCB:** the same steps with `Program::new("pcb", ..)`. From a part of the design, show the panel and call `attachments.model.set_filter(Some(piece))` with the designator, value, part number and maker of that part (`Piece`), so `R5` or `U3` lists its data sheets; linking the chosen attachment to the part is the form of the panel.

## The PDF library

PDF pages need PDFium beside the executable: `tools/fetch_pdfium.py --platform <platform> --out <folder>` puts the pinned library and its licences there (see [VIEWER.md](VIEWER.md)). A package of a program carries that folder, the library as it is published (it is never signed again) and `crates/viewer/THIRD_PARTY_NOTICES`. Without it the panel says so and offers to open the file outside.

## Not done

Dropping a file on the panel; downloading a link (a person does it outside); reading the limits from the settings files of the programs, which use the defaults (1 GB).
