# The docking frame (`eh3d-dock`)

The programs of the suite are professional desktop applications: panels that can be taken away and put back, moved, tabbed, split, floated over the window, folded into an edge, maximised and locked, and a menu bar with the usual menus. `eh3d-dock` is the frame that does it, written once for all of them. It has no dependency beyond `eh3d-view` (for the languages) and, with the feature `gui`, `egui` 0.37 and `png` (for the picture of the window about a program).

The crate has two layers.

* **The model has no window in it and is always there.** The tree of the layout and every operation on it (`model`), the geometry (`geom`, `arrange`), the text form (`serial`), the named workspaces (`workspaces`), the lines of the settings file (`config`, `recent`, `shortcuts`), the menus as data (`menu`, `stdmenus`), the panels and workspaces proposed for each sector (`sectors`), the whole state (`core::DockCore`) and a picture of a layout as SVG (`preview`). All of it is tested without a window, many tests on inputs made with a fixed seed.
* **The drawing needs the feature `gui`.** `gui::Dock` wraps a `DockCore` and draws it with egui: the tab bars, the dragging with the zones and their preview, the dividers, the floating windows, the folded tabs, the menu bar and the small windows. The drawing only reads the model and collects what the person did as a list of events; the events are applied to the model after the frame, so every rule lives in the model.

```toml
[dependencies]
eh3d-dock = { git = "https://github.com/JuanenRac/EH3D-COMMON", tag = "<release tag>", features = ["gui"] }
```

`<release tag>` is the tag of the release of this repository that the program is built against (see [CONSUMING.md](CONSUMING.md)).

## What a person can do

| Action | How |
|---|---|
| Move a panel | Drag its tab. Over the middle of a group it becomes a tab (at the place between the tabs where it is dropped); near a side of a group it splits the group; along the edge of the window it takes a quarter of the window. A preview shows the result before the button is released. Tabs that do not fit in one row go to further rows with their whole name, and they are dragged, dropped between and reordered in any row. |
| Float a panel | Drag the tab out of the window, or the tab's context menu, `Float`. A floating window can hold several tabs, can be moved and resized, has a button that brings it back (`Dock`) and a close button. |
| Fold a panel into an edge | The context menu, `Auto-hide`, then the side. Only a tab remains on the edge; clicking it shows the panel over the work area until the pointer clicks elsewhere or `Escape` is pressed. |
| Hide and show a panel | `x` on the tab bar, the context menu, or the View menu, which lists every panel with a check mark. A panel comes back to the place it left, when that place still exists. A panel that is asked for (`DockAction::ShowPanel`, `Layout::bring_to_front`) is left in sight: its tab on show, its floating window over the others, and a maximising of another group ended. |
| Resize | Drag a divider. The share of each part is kept inside its range, and no part goes below the smallest size its panels declare while there is room. |
| Maximise and restore | The button of the tab bar, the context menu or the shortcut (`Ctrl+M`). |
| Lock | View, `Lock the layout`. A locked layout refuses every change of the arrangement (moving, floating, hiding, folding, resizing, reordering, saving and deleting workspaces); changing the tab on show, maximising and switching workspace still work. |
| Workspaces | Window, the list of workspaces; `Save the layout as…` keeps the arrangement under a name; `Delete this workspace`; `Restore the layout` gives the workspace its factory arrangement. |

Keys: `Ctrl+M` maximise, `Ctrl+PageDown` and `Ctrl+PageUp` next and previous tab, `Alt+Shift+Arrow` move the panel with the focus one step towards that side, `Escape` cancels a drag and closes a folded panel, and every panel can be given a shortcut that shows or hides it. All of them can be changed in the settings file (below). Tabs are reachable with the keyboard like any other button, and each one announces itself to a screen reader as «Panel Layers, docked».

## The smallest program

```rust
use eh3d_dock::core::PanelDef;
use eh3d_dock::gui::Dock;
use eh3d_dock::menu::Chosen;
use eh3d_dock::sectors::Sector;
use eh3d_dock::stdmenus::{standard_bar, HelpOptions};

// the actions of the program; the standard menus give back `StdAction`, so the type must be made from it
#[derive(Clone, PartialEq, Debug)]
enum Cmd { Std(eh3d_dock::menu::StdAction), Slice }
impl From<eh3d_dock::menu::StdAction> for Cmd { fn from(a: eh3d_dock::menu::StdAction) -> Cmd { Cmd::Std(a) } }

let sector = Sector::Pcb;
let mut dock = Dock::new(sector.panels(), sector.workspaces());
dock.register("board_view", |ui| { ui.label("the board"); });
dock.register("layers", |ui| { ui.label("the layers"); });
let bar = standard_bar::<Cmd>(HelpOptions::default());
dock.load_config(&settings_text); // at start

// every frame, in eframe's `ui`:
egui::Panel::top("menu").show(ui, |ui| {
    if let Some(chosen) = dock.menu_bar(ui, &bar) {
        match chosen {
            Chosen::App(Cmd::Std(action)) => { /* New, Open, Save, Quit... */ }
            Chosen::App(Cmd::Slice) => { /* the program's own */ }
            Chosen::Recent(path) => { /* open that file */ }
            Chosen::Dock(_) => { /* the frame did it already */ }
        }
    }
});
dock.show(ui);
// when the settings are saved:
let new_text = dock.config_text(&old_settings_text);
```

`Dock::show(&mut self, ui)` draws in the space left in a `Ui` (egui 0.37 has no top-level `show(ctx)`; `Dock::show_ctx(ctx)` does the same through the root `Ui` of the context). A body of a panel is any `FnMut(&mut Ui)`; it is called only for the tab on show.

`cargo run -p eh3d-dock --example dock_demo --features gui -- --sector cnc --workspace Run --lang es --shot cnc.svg` writes a picture of a factory layout of any sector and draws a few frames of the real egui code with no window.

## Tabs in rows

When the tabs of a group do not fit in the width of its bar, the bar grows by rows of `gui::BAR_H` points and the content of the panel starts under the last one; the first row is shorter, because the buttons of the group are at its right. A title wider than a whole row (a very narrow panel) has a row of its own, is cut at its end and is shown whole under the pointer. A bar takes at most half the height of its group, and never fewer than two rows; if the tabs need more, the rows shown are those that keep the tab on show in sight, and `Ctrl+PageDown` and `Ctrl+PageUp` reach the others. The rows are worked out by `arrange::tab_rows`, with no window.

## Declaring the panels

A panel is a `PanelDef`: a stable name (letters, digits, `_`, `-`, `.`, at most 48 characters; it is what the settings file stores), a title in eight languages (`Text8`, in the order English, Spanish, Italian, German, French, Portuguese of Portugal, Chinese, Japanese), an optional icon of one or two characters (a short sign a program may show elsewhere; the tabs always write the title), the smallest size it needs and an optional shortcut that shows or hides it.

```rust
PanelDef::new("layers", Text8(["Layers", "Capas", "Livelli", "Ebenen", "Calques", "Camadas", "图层", "レイヤー"]))
    .min(160.0, 120.0).icon("L").key(Shortcut::ctrl_shift("L"))
```

`eh3d_dock::sectors::titles` has the titles every sector shares, already in eight languages.

## The factory layout of a sector

`Sector::{Printer3d, Pcb, Cad, Cnc, Pnp, Laser}` give `panels()` and `workspaces()`: the panels and three workspaces each, the same in every program of a sector. A program takes them, adds or drops panels and hands them to `Dock::new`. A factory layout may leave a panel out (it starts hidden); a name it does not know is dropped, so a mistake in the factory data never reaches the person.

| Sector | Panels | Workspaces |
|---|---|---|
| 3D printing (slicer) | `view3d`, `objects`, `print_settings`, `materials`, `preview`, `gcode`, `printer`, `console` | Prepare (settings and materials on the left, the 3D view in the middle, objects and printer on the right, console and G-code below), Review (the view with the layer preview, G-code and settings), Compact |
| Circuit boards | `board_view`, `layers`, `properties`, `components`, `nets`, `drc`, `library`, `console` | Edit (components, nets and library on the left, the board in the middle, layers and properties on the right, console and rule check below), Review (the board with the rule check, properties and nets), Compact (layers and properties folded) |
| Mechanical design | `view3d`, `model_tree`, `properties`, `tasks`, `report`, `console` | Model (the model tree and tasks on the left, the 3D view, properties on the right), Review, Compact |
| Machine control (milling, drilling) | `viewer`, `console`, `jog`, `job`, `position`, `tool_table`, `files` | Run (position and jog on the left, the viewer and console, job and files on the right), Prepare (files and tool table, viewer, job), Compact (console and jog folded) |
| Pick and place | `board_view`, `parts`, `feeders`, `job`, `camera`, `machine`, `console` | Setup (parts and feeders on the left, the board and camera, machine and job on the right), Run, Compact |
| Laser | `view`, `layers`, `cut_settings`, `library`, `jog`, `camera`, `job`, `console` | Design (layers and library, the view, cut settings and job), Engrave (the view with the camera, job, jog and cut settings), Compact |

The names of the standard workspaces are shown in the language of the window (`sectors::workspace_title`); the name kept in the settings is the English one. A name a person types is shown as typed.

## What goes into the settings file

The frame writes only keys that start with `dock.` and leaves every other line of the file as it is, in the line ending the file already uses. `DockCore::config_text(existing)` gives the new text and `DockCore::load_config(text)` reads it.

| Key | Meaning |
|---|---|
| `dock.version` | The version of the form, `1`. A different one makes every layout the factory one, with the warning `layout_other_version`. |
| `dock.current` | The index of the workspace in use. |
| `dock.ws.<n>.name` | The name of workspace `n` (1 to 32 characters, no control character). At most 16 workspaces. |
| `dock.ws.<n>.tree` | The tree, on one line: `H200(T0(files,layers),V700(T0(view),T1(console,log)))`. `H` puts the parts side by side, `V` one above the other, the number is the share of the first part in thousandths, `T<k>(…)` is a group of tabs with tab `k` on show. |
| `dock.ws.<n>.float.<i>` | A floating window: `x,y,width,height;active tab;panel,panel`. |
| `dock.ws.<n>.auto` | The folded panels: `left:notes,bottom:console`. |
| `dock.ws.<n>.hidden` | The hidden panels, comma separated. |
| `dock.ws.<n>.max` | The maximised panel. |
| `dock.ws.<n>.locked` | `1` when the layout is locked. |
| `dock.key.<action>` | A shortcut that differs from the default, as `Ctrl+Shift+S`, or `none`. Actions: `maximize`, `next_tab`, `prev_tab`, `restore_layout`, `toggle_lock`, `move_left`, `move_right`, `move_top`, `move_bottom` and `panel.<name>` (show or hide that panel). |
| `dock.recent_limit`, `dock.recent.<n>` | How many recent files are kept (1 to 30, 10 by default) and the files, newest first. |

A layout that is damaged, from another version, too long (a tree of more than 64 KiB, deeper than 24 divisions or with more than 256 panels) or full of names the program does not have never makes the program fail: a damaged one is replaced by the factory layout of its workspace and a warning is kept (`DockCore::take_warnings`, shown by the frame as a small notice in the language of the window); one that only has problems that can be put right (a panel that does not exist, a share out of range, a panel in two places, a panel the program has and the layout does not name) is repaired, and the warning says so. `Dock::layout_text()` and `Dock::set_layout_text()` do the same for a single layout as a text of its own (a header `eh3d-dock-layout 1` and the same keys without the prefix).

## The menu bar

A program declares its menus as data. Every entry has a stable `id`, a text in eight languages (`Label::Own(Text8)` for its own words, `Label::Std(DockText)` for the words of the frame), an optional `Shortcut`, an optional check mark, whether it is enabled, and the action it gives back as a value of the program's own type; no closure is hidden in a menu.

```rust
let tools = Menu::new("tools", Label::Std(DockText::MenuTools))
    .entry(Entry::app("slice", Label::Own(SLICE), Cmd::Slice).key(Shortcut::plain("F5")))
    .sep()
    .entry(Entry::app("grid", Label::Own(GRID), Cmd::Grid).check(true));
```

* **The standard menus** (`stdmenus`): `file_menu` (New, Open…, Open recent, Save, Save as…, Export…, Quit), `edit_menu` (Undo, Redo, Cut, Copy, Paste, Preferences…), `view_menu`, `window_menu`, `help_menu` and `standard_bar`, which puts them in order. They give back `StdAction` values; the program's action type implements `From<StdAction>`.
* **The generated parts** (`Item::Generated`): `Gen::Panels` is one checked entry per panel, `Gen::Layout` is Restore the layout and Lock the layout, `Gen::Workspaces` is the list of workspaces with Save as… and Delete, `Gen::Tabs` is Next tab, Previous tab and Maximise, `Gen::Recent` is the submenu of recent files with a Clear entry. The View menu is the panels and the layout, the Window menu is the workspaces and the tabs.
* **Help:** `help_menu` offers online help (`F1`, when the program has it), the window with the keyboard shortcuts (drawn by the frame from the menus and the frame's own keys), Check for updates (only when the program asks for it) and About (the name, version and license the program gives with `Dock::set_about`, and the notice that third-party names are trademarks of their owners).
* **Turning entries off and ticking them:** `Dock::set_menu_state(disabled_ids, checks)` can be called every frame. An entry that is off is built as inert and no shortcut, click or search will give back its action (checked by tests).
* **Shortcuts:** `Dock::handle_shortcuts` (called by `menu_bar`) looks for a pressed key, most specific first (`Ctrl+Shift+S` before `Ctrl+S`); a key without a modifier is ignored while a text box has the keyboard. `MenuBar::conflicts(&ctx)` lists the shortcuts two entries share, counting the frame's own; `MenuBar::validate()` finds repeated and empty ids.
* **Recent files:** `dock.core_mut().recent_mut().push(path)` when a file is opened or saved; the list is kept in the settings file, and a program of the PHOENIX family keeps it in the folder of the user too (below).

## Where the recent files are kept

A program of the PHOENIX family calls `Dock::keep_user_recent(program)` once, when it opens its window and after it read its settings. From then on the list lives in a file of its own, whatever settings file the program was started with, and also without one:

| System | File |
|---|---|
| Windows | `%APPDATA%\EH3D\recent-phoenix-<program>.txt` |
| Linux, macOS | `$XDG_CONFIG_HOME/eh3d/recent-phoenix-<program>.txt`, or `~/.config/eh3d/` |

* The file is a line `eh3d.recent=1` and the lines `dock.recent.<n>=<path>`, newest first. It holds paths and nothing else; an entry with `://` (an address, which could carry a password) is never written nor read. On Linux and macOS only its owner can read it.
* It is written into a new file of the same folder first, which then takes the place of the old one, when the list changed.
* A file that cannot be used (not this kind, not text, over 64 KiB) is left aside without an error and replaced by the next list.
* The first time, the list that the settings file held moves to the new file. Afterwards `dock.recent_limit` still says how many are kept, and the `dock.recent.<n>` lines of the settings file are no longer read.
* Tests and the pictures of the windows do not call it: `recent_store::RecentStore::in_dir` and `DockCore::keep_recent_in` take a folder of their own.

## Where the path of the signing key is kept

A program that signs offers the key used last: `key_store::KeyStore::for_user(program)` gives it, `load` reads the path, `remember` keeps the path of the key that was just used and `forget` drops it. The file is `signing-key-phoenix-<program>.txt` in the same folder as the recent projects: a line `eh3d.signing-key=1` and a line `key.path=<path>`. It holds **the path only**: nothing of it reads the key file, and no key and no password is written. Only an absolute path of one line is kept, never an address; on Linux and macOS only its owner can read it. A key that is no longer there is the program's to notice.

## The window about a program

`Dock::set_about` gives the name, version, licence and extra lines. A program of the PHOENIX family also calls `Dock::set_brand` with an `about::AboutBrand`: which program it is (`about::Phoenix`) and the bytes of its own picture, put inside the program with `include_bytes!`.

```rust
dock.set_about(Phoenix::Cad.info(env!("CARGO_PKG_VERSION")));
dock.set_brand(AboutBrand {
    program: Phoenix::Cad,
    logo_png: include_bytes!("../images/brand/icon-256.png"),
});
```

The window then shows the picture inside a ring of the accent colour of the program, its name in large letters, one sentence about it in the eight languages, the version, the author, the e-mail, the page of the author, the repository (the addresses can be clicked), the licence, the notices of data of others when the program carries them and the notice about trademarks. The accent colour, the sentence, the repository and the notices belong to `about::Phoenix`, so they are written once. A program with a window of its own calls `gui::about_contents(ui, lang, &info, &brand)` inside it. Its words (the labels, the weak lines and the links too) are written at 4.5:1 or more on the ground of the window (`window_fill`), whatever the program changed of its visuals. Without a brand the window is the plain list of lines, as before. The picture is a PNG of at most 1024 by 1024 pixels; one that cannot be read leaves the ring empty.

| Program | Palette | Principal | Middle | Deep | Light | Glow |
|---|---|---|---|---|---|---|
| PCB | fire | `#FF5A1F` | `#E2361B` | `#B01010` | `#FFB347` | `#FFD27A` |
| PNP | emerald | `#2FFF94` | `#17C47A` | `#0B8C64` | `#8FFFA4` | `#D8FFDE` |
| CAD | ice blue | `#2F8CFF` | `#1F5FD0` | `#0B2A8C` | `#8FE3FF` | `#D8F6FF` |
| CNC | amethyst | `#A52FFF` | `#8A1FD0` | `#6F0B8C` | `#AD8FFF` | `#E2D8FF` |
| LASER | crimson | `#FF2F43` | `#D01A20` | `#971100` | `#FF8FBC` | `#FFD8E8` |
| 3DP | gold | `#FFD53E` | `#E8A317` | `#A86E00` | `#FFDC5E` | `#FFE999` |

The tones live in `eh3d_dock::palette` (`Phoenix::tones`), and the accent is the principal one. It is meant for fills and rings: gold or emerald are not legible as text on a white window, nor amethyst on a dark one. For text, `palette::readable(colour, ground)` gives the nearest lightness of the same hue with a contrast of at least 4.5 to 1, and `palette::ink_on(fill)` gives black or white for what is written on a fill (a button). The window «About» writes the name of the program with the readable tone of the ground of the window.

## The colour of a program in its window

Only the accents of a window take the colour of its program: main buttons, what is selected, the active tab and links. Grounds, panels, bars and the text stay as the program set them.

* `Dock::set_brand` is enough; a program with a window «About» of its own calls `Dock::set_program` instead. The frame then keeps the accents each time it draws, also after the program set other visuals (a change of theme).
* `palette::accents(main, ground, other grounds, text)` works them out without a window: links read at 4.5 to 1 on every ground, the selection is the ground with the strongest tint (8 to 40 hundredths of the colour) on which the text of the window still reads at 4.5 to 1, with its own text and outline at 4.5 to 1 too, and a main button is the principal tone with black or white text.
* `gui::accent_visuals` puts them on `egui::Visuals`: the selection, the links and the outlines of the widgets that were already coloured (a grey outline stays grey). The active tab of the frame has the tinted ground, text in the colour of the program and a line over it.
* `gui::primary_button(program, text)` makes a main button, and `gui::program_mark(ui, &brand, size)` draws the picture of the program for the header of its window.
* The tests check the contrasts for the six programs on light and dark looks.

## Moving a program to the frame

1. Name the panels. Give each a stable name and a title in eight languages; take the titles of `sectors::titles` where they exist.
2. Take `Sector::<yours>::panels()` and `workspaces()` and change what the program needs; keep the names of the panels the sector already has.
3. Turn the body of each panel of the program into a function `FnMut(&mut Ui)` and `register` it. A body must not assume its size: it is given what is left of its group.
4. Replace the program's own menu bar with `standard_bar` plus its Tools menu (or its own menus) and handle the `Chosen` values. Keep the program's own shortcuts out of the way of the frame's: `MenuBar::conflicts` says if two are the same.
5. Read the frame at start with `load_config` and write it back with `config_text` when the settings are saved; take `take_warnings()` into the log if the program has one (the frame also shows them).
6. Remove the program's own panel code and its saved panel sizes.

## Limits that are written down

* The frame has been run with egui 0.37 in a window-less context in the tests (frames, shortcuts, dragging a tab, dragging a divider, locked layouts); it has not been seen in a real window on every system. The check marks of a menu are drawn as the highlight of the entry.
* The title of a workspace the person types is not translated.
* A panel cannot be dropped on a window that belongs to another program; a floating window lives inside the window of the program.
