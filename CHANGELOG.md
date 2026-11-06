# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **The files of the touch screens that each screen grows on its own leave the shared firmware base:** the keyboard, the notices, the link to the server, the sound files and the flow of a job are no longer copied between the two screens, so the check of the base passes again after one screen changed them.
- **Words for the window of the positions of the tools in the bar in the 37 languages:** the title, the buttons to assign in order and to clear the assignment, the empty choice, the three states of a tool (pending, in use, done), the three warnings (a tool with no place, two tools in one place, a place the machine file does not define) and the note about how the choice is kept.

## [0.1.9] - Texts of the job stages, the progress blocks and the jog panel in every language, and the zip crate at 9

- **Words for the notices of the job in the 37 languages:** the notice between the drilling and the routing, the end of the job and the blocks under the progress bar (tool and step, time, position and depth, counters, elapsed, end time, depth, feed, tool changes, hole and segment) now exist in the 27 languages of the files too, so every language has every text. The translations wait for a native review.
- **Words for the panel that moves the machine by hand in the 37 languages:** the title, the step and the continuous mode, the three speeds, the machine and the work positions, the zero of the work, home, the saved positions with their buttons, the four reasons a move is refused (a job runs, the emergency stop is on, end of travel, Z too low to move across) and whether the machine is real or simulated. The translations in the 29 languages of the files wait for a native review.
- **Dependency:** the `zip` crate is raised to 9.0.5 (the old range had a security advisory); packages are still written with deflate, the fixed 1980 time and the same entry order.
- **Words for the list of board materials in the 37 languages:** the types of material (laminate, PTFE and RF, ceramic filled, metal core, plain sheets), the search box, the mark of a material without data of the manufacturer and the request for its thickness and copper.

## [0.1.8] - The board materials database with 723 materials and a classified catalogue of 1873 documents

- **The board materials database:** the new crate `eh3d-materials` holds laminates, prepregs, flex, metal cores and PTFE boards of Shengyi, Isola, Panasonic, Rogers, Taconic, Arlon, ITEQ, Ventec, DuPont, Doosan, Bungard and other makers, every value as the sheet publishes it with the PDF it comes from. It checks the data when it loads it (the codes `EH3D-MAT-001` to `EH3D-MAT-009`), answers by identifier, maker, kind and minimum Tg, Dk and Df, interpolates Dk and Df between the published frequencies without extrapolating, and gives the small view that the machine program needs. A catalogue of the reference documents classifies each by category and by whether it serves a CNC that drills and mills boards or the design tool.
- **The sample files of the machine programs are tested here:** `crates/formats/tests/data` holds the sample configuration, tool tables, drill files and plots of the drilling and routing programs, and the round trips (a configuration and a tool table written back keep their comments and values) are tests of this crate.
- **Writing a configuration file gives a typed error:** `write_with_backup` returns a `WriteError` with its `kind` (character, backup, write, rename) that reads as its message (`Deref<Target = str>`, `Display`, `From` it to a `String`), so the callers go on working.
- **A place for the lints of every crate:** `[workspace.lints]` (empty for now) is inherited by all of them.
- **The CI compiles and runs the host tests of the firmware base** (`test_base` and `test_shared`) with the warnings on.
- **The CI commands run the same on the computer:** `cargo ci-lint` and `cargo ci-test` (aliases in `.cargo/config.toml`) are what the CI runs, and a plain `cargo test` covers every crate of the workspace (`default-members`).
- **The crates share what they have in common:** edition, rust-version, licence, authors and repository are written once in `[workspace.package]` and inherited.
- **A refusal of GRBL has its number and its words:** `Error::code()` gives the number of an `error:N` or `ALARM:N`, and `Error::official_text()` the text of the table of GRBL 1.1 (`codes::grbl_error_text`, `codes::grbl_alarm_text`).
- **The transport of the tests can delay a reply:** `MockTransport::with_delayed` has replies that become readable at a moment of a virtual clock, so a timeout of the controller is tested without waiting.
- **A guide to use the crates from another repository:** `docs/CONSUMING.md`.

## [0.1.7] - Height over an area with no width, the shared job bar of the touch screens, new texts and the application icon

- **Brand:** Add the application icon and the logos of the program.

- **A height map over an area with no width or no height gives a height:** the interpolation no longer returns NaN when all the points are in one column or row, and a position that is not a finite number has no height.- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.

- **A height map with a huge grid is an error:** reading a saved map checks the number of points (at most `MAX_MAP_POINTS`) before anything is reserved for it, so a map that announces billions of points no longer aborts the program; a move is cut in at most `MAX_PATH_PIECES` pieces.

## [0.1.6] - Strict container, PKG codes, shared object identifier and the 37-language texts of the machine and the job bar

- Add the object identity to `eh3d-container`: `ObjectId` (UUID version 7 in canonical lowercase text), the `IdGenerator` over an injected `Clock` and `RandomSource` (a system clock and a seeded one for tests), and `IdError` with the codes `EH3D-ID-001` and `EH3D-ID-002`, so that every program of the suite names its objects the same way. The design tool and the mechanical tool use it instead of each having its own.
- Add the stable codes `EH3D-PKG-001` to `EH3D-PKG-014` (`PkgCode`) with the common diagnostic for each, and a test that checks the list against the registry of the documentation repository when it is checked out next to this one.
- Add the strict name and ZIP policy for new formats: names in Unicode normal form C with the UTF-8 flag, duplicates that differ only in case or form, links, devices, directory entries, ZIP64, encryption, methods outside the preset, other flags, data after the end of the archive and entries outside the roots are refused with their code; the `.eh3dproj` 1.0.0 policy keeps its rules.
- Add `Container`, a bounded reader that opens from the central directory only, refuses a package before decompressing or reserving memory for what it announces, checks the format version before the roots, and streams entries under the limits and a time budget whatever the archive claims.
- Add `ContainerWriter`: entries are added in any order from a stream, compressed once into a temporary file while their SHA-256 is computed, and assembled in the order of the preset with a canonical `integrity.json` last; the same content gives the same bytes in any insertion order.
- Add `write_atomic`: temporary file in the target folder, flush and sync, rename, a bounded retry (2 s by default) when another program holds the file on Windows, `EH3D-PKG-014` with the target untouched when it fails, an optional backup copy and a cleaner for old temporary files.
- Add `Budget` (time and memory) next to `Limits` and `AtomicOptions`, all plain data that a program can read from its configuration file.
- Refuse in the strict policy the parts of a name that end with a dot or a space and the names that Windows treats as devices (`CON`, `NUL`, `COM1`, ...), with the code `EH3D-PKG-001`.
- Add the presets `ContainerSpec::phoenixproj` and `ContainerSpec::eh3dproj_v1_1` (with the `assembly/` and `heightmaps/` folders); exports stay at 1.0.0 unless 1.1.0 is asked for.
- Add the acceptance tests of the container: the generic writer with the 1.0.0 preset reproduces the three reference packages byte for byte, the new reader reads them, and a reader of 1.0.0 refuses a package of the new formats without panicking.
- Add the `eh3d-container` crate with the pieces that every container format shares: the integrity record with its verification and a content hash that leaves out audit entries, the entry-name rule of version 1.0.0 and canonical JSON (RFC 8785) with the vectors of the standard.
- Describe the parameters of a container format (roots, limits, JSON form, entry order, versions) in a `ContainerSpec` and add the preset that records `.eh3dproj` 1.0.0 as it behaves; a test keeps the preset and the code in agreement.
- Use the shared integrity record and name rule inside the `.eh3dproj` container without changing a byte of what it reads or writes: the reference packages and their tests pass unchanged.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.

## [0.1.5] - Diagnostic crate and reference packages of the project container

- Add the `eh3d-diagnostic` crate: the diagnostic of the suite with a stable code, severity, category, recoverability, location and translatable arguments, a stable JSON form and a redacted form for private corpora; its only dependency is the optional `serde`.

- Record how the `.eh3dproj` container behaves today as tests: entry order, the exact form of `project.json` and `integrity.json`, ZIP flags, compression, time stamps, no ZIP64 and the verdict on damaged packages.
- Add a corpus of reference packages (three valid, nineteen damaged) that the writer must reproduce byte for byte and the reader must judge as recorded.

## [0.1.4] - Travel of a GRBL controller and words for the queue of programs

- **The travel of a GRBL controller can be read:** the controller trait has a `max_travel` call (default: none) and the GRBL controller answers it from `$130`, `$131` and `$132`.
- **Words for the queue of programs and the travel reading** in the 37 languages.

## [0.1.3] - Crash record and words for the new windows

- **The record of a crash is shared (`eh3d_view::crash`):** a panic is appended, with the program, its version and the place, to `eh3d-crash.log` in the configuration folder of the user, so a window program that has no console leaves something to read; the record and the append are tested.

- **The words of the machine-control window for the summary before a start, the safety settings and the shortcuts, in the 37 languages:** the moves and the cutting distance of a G-code program, the arcs and the missing feed that the estimate leaves out, Start anyway, showing the summary, the rapid speed, refusing a program that leaves the travel, the keyboard shortcuts, the toolbar, the safety section and the message of a file that cannot be opened.

## [0.1.2] - Shared pictures reader and shared dry run

- **The dry run and the summary before a start are shared (`eh3d-summary`):** the exact steps of a plan are followed on paper (`dryrun::Tracker`, fed by the program that plans the job) to know how far each axis goes and how long it takes, and the plan that leaves the course of the machine is found before a motor turns; `summary` holds the sanity checks of the work (wrong unit, bigger than the table, far from the corner, a scale that is not 1), with thresholds (`Limits`) that come from the tuning of the machine, the refusal and the lines of the summary. The drilling and routing programs, the modern interface and the server say the same. 21 tests.
- **The words of the summary in `eh3d-view`:** `SummaryTitle`, `SummaryHoles`, `SummaryTools`, `SummarySize`, `SummaryTime`, `SummaryReach`, `WarnTiny`, `WarnBig`, `WarnFar`, `WarnScale`, `DryOut` and `DryOk` in the 37 languages.
- **The reader of pictures is shared (`eh3d-image`):** the grey levels of a PGM (P2, P5) or PNG file (grey, colour, palette or transparent, 8 or 16 bits) with limits on the size so that a hostile file cannot ask for gigabytes, `resize` with a box filter, `blank` and `disc`. The laser engraving and the fiducial finder read their pictures with it instead of a copy each. 7 tests.
- **The crate that uses `eh3d-common` by path says the version it needs** (`eh3d-view`: `>=0.1.1`).

- **The CI builds the repositories that depend on this one:** a `consumers` job reads which repositories name this one in `depends_on` of their manifest and runs `cargo check --all-targets` on each with the change in place of the sibling, so a break reaches the author before the merge.

## [0.1.1] - The words of the tool changer bar, of the text boxes and of a board that is turned, and the firmware base for the two touch screens

- **A text to swap the width and the depth of a board (eh3d-view):** one new text, in the 37 languages, for the button that gives a board the other way round (a board that does not go into the machine often does turned).
- **The bar of the tool changer in the machine file (eh3d-bungard-formats):** `RackSetup` (the length of the tools, from the ring to the tip, the clearance over the tools, the side the chuck comes in from and how far past the table the bar may be) is read from lines of the form `RACK_TOOL_LENGTH 38.00` after the places, with defaults when they are missing, so the files that exist read as before; `rack_line` spaces the places evenly between the first and the last tool and `rack_ends` finds them again; `rewrite` writes the places and those lines back (and the line of the changer to a file of the manual layout) only when they changed. The words of the window of the bar are in the 37 languages of eh3d-view.

- **The words of the text-box menu, of the full-screen setting, of the board warning and of the list of makers (eh3d-view):** `ClipCut`, `ClipCopy`, `ClipPaste`, `ClipSelectAll`, `ClipUndo`, `FullScreen`, `BoardTooBig`, `TableNoun` and `AllMakers` in the 37 languages, and `eh3d_view::clip`, which says which entries of the menu of a text box can be used (a selection to cut or copy, a box that can be changed, a clipboard with text) and who gets a clipboard key (the text box when it has the keyboard, the drawing otherwise). 5 tests.

- **The firmware base is current and watched in the CI:** the base took back the node's 37 languages and the packing of the language file of the panel (a `sync` would have reverted them), EH3D-NODE-BUNGARD-AUX is registered with its own names (web_server.hpp and check_sdkconfig.py stay the node's own), and the CI now clones every project next to this repository and runs `check --strict`, which fails when a project is missing instead of skipping it.

- **The firmware base also serves the two touch screens:** `tools/sync_firmware_base.py` now knows EH3D-NODE-BUNGARD-HMI and EH3D-NODE-HMI-GUI and keeps 21 of their files identical (the flow of a job, the link to the server, the job memory, the tool reader, the card files, the keyboard, the notices, the orientation, the quality reading, the voice, the board and their host tests); the gate, the texts, the settings and the screens stay each project's own. `import` for a screen project takes its own list of files, and `check` in the CI reports a drift in a screen as it does in a node.
- **Nothing is dropped without a word (eh3d-bungard-formats):** `drill_tools::parse_report` and `route_tools::parse_report` give the tools of a table and the lines that could not be read, with their numbers, and both tables read a decimal comma; a drill file lists in `ignored` the slots and routed moves (`G00`/`G01`, `M15`/`M16`) it does not hold, and a headerless body whose first line is already Excellon keeps its holes though a `%` closes it. A metric file is read in millimetres (a test pins `X12345` at 12.345 mm).
- **A text for a job that cannot go on (eh3d-view):** `ResumeChanged` in the 37 languages.

## [0.1.0] - Texts of the tool rack and the loading tips, the sheets without copper and the homing of GRBL

- **Waits of the GRBL controller by the kind of line (eh3d-controller):** `Timeouts` (an ordinary line 5 s, the homing cycle `$H` 120 s, a jog `$J=...` 60 s, all set with `GrblController::with_timeouts`) instead of one short wait for everything, and the wait is the total for the line, not the time between two lines of the controller. A line that does not end in time holds the machine by its kind (the homing cycle is reset, because it ignores a hold; a jog is cancelled; anything else is held) before the `Timeout` is returned, so a failure never leaves it moving. The test transport keeps the waits it was given. 2 tests more.
- **The simulator takes the safe `$` lines of GRBL (eh3d-controller):** `$H` (a homing cycle that lasts `home_duration`, 20 s on the clock of the simulator, really waited only with `real_time`, and leaves the machine at its home; it also ends an alarm), `$X`, `$G`, `$$`, `$#`, `$I`, `$N`, `$C` (check mode: the lines are judged and nothing moves), `$J=...` (one move that changes no mode and needs a feed) and the numbered settings (`$100=250`, kept in `settings`). `$RST`, `$N0=...`, `$SLP` and any other `$` line are still refused. 4 tests more.

- **Tips for loading the tool rack (eh3d-view):** the size of the tools, the air and the depth limiter with the changer, in the 37 languages.

- **Texts for the sheets without copper of the board library (eh3d-view):** the names with words of aluminium, copper, birch plywood, balsa, basswood, PTFE, acrylic, polycarbonate, nylon, G10 and phenolic paper, and "No copper", in the 37 languages (the eight inside the program and the 27 files of `locales/`); the designations that are the same everywhere (MDF, ABS, POM, PVC, HDPE, PETG) need none.

## [0.0.9] - G-code jobs in the fleet and the texts of editing, the rack and the serial ports

- **Free G-code jobs in the fleet (eh3d-fleet):** a job of `kind` `gcode` is read with its state, its `line` (0 when the fleet answer does not carry it), the question it waits at (`hold` when the operator held it, `resume` when it goes on after a stop and waits before its first move) and the tool of a tool change; `Job::is_gcode`, `is_held`, `is_going_on` and `wants_resume` tell the answer to give (`resume` or `confirm`). `Machine::takes` says which kinds of job a connection takes (a node drilling and routing, a GRBL controller G-code, the simulator all three) and `Fleet::pick_free_for` picks the free machine for a kind of job. The text table shows the line and the question. 3 tests more.
- **The name of a board material in the 37 languages (eh3d-view):** FR4 600 series, positive presensitized.
- **Texts of the tool rack of the ATC models (eh3d-view):** the title, the slot 0 of the test tool, the request to load the rack and the name of a load, in the 37 languages.
- **Texts of the connection to the server (eh3d-view):** the Server section of the settings, the address, the access token and the machine id, and the entry that sends a program to the server, in the 37 languages.
- **Texts of the serial ports, of undo, redo and cut, and of starting a job at a tool (eh3d-view):** no ports found, look for ports again, undo, redo, cut and start the job from this tool, in the 37 languages.
## [0.0.8] - Texts for the editing of the plan view, the preview and the catalogue

- **Texts of the editing of the plan view (eh3d-view):** choose, copy, paste, delete and remove the tool from the job, in the 37 languages.
- **Texts of the preview in the Open window (eh3d-view):** the name of the switch and the note for a file that cannot be drawn, in the 37 languages.
- **Texts of the bits taken from the catalogue (eh3d-view):** the board material that was applied, the three notes (the material wants fewer turns than the bit, fewer than the spindle can give, the cutting length does not go through the board and the extra) and the short warning of the window of tools, in the 37 languages.
- **Texts for the bar of the job, the kinds of link, the catalogue and the materials (eh3d-view):** job, Z feed, spindle, tool, the tag of the modern interface, series, tip, demo, EH3D node, EH3D server, the maker "Generic" and eight material names, in the 37 languages. Four labels of the switches of the plan view were shortened in Greek, Dutch, Russian and Ukrainian so they fit.

## [0.0.7] - More languages, the spindles and the texts of the parts of a job

- **Twenty-nine more languages (eh3d-view):** Bulgarian, Croatian, Czech, Danish, Dutch, Estonian, Finnish, Greek, Hungarian, Irish, Latvian, Lithuanian, Maltese, Polish, Romanian, Slovak, Slovenian, Swedish, Norwegian, Icelandic, Ukrainian, Russian, Serbian, Albanian, Catalan, Galician, Basque, Brazilian Portuguese and Canadian French, read from the files of `crates/view/locales` (one line per text); a text a file does not have is taken from English, or from Portuguese and French for the two variants. `Lang::ALL` lists the 37, with `native_name`, `fallback`, `coverage` and codes such as `pt-br`. The translations are the work of the model and are open to review by native speakers. A test checks that each new language has all its texts and its own alphabet.
- **Text of the choice of several files** in the eight first languages (and the new ones as they are translated).

- **Texts of the parts of the job:** the notice when nothing is switched on and the hints of the switches of the drilling, the routing and each tool, in the eight languages.
- **Texts of the spindles:** the two spindles of the machine, the warnings of the catalogue and of the tools (fits, needs it slower, Z or XY slower, not compatible because of the thickness or of the speed) and the button that takes the values of a bit into a tool, in the eight languages.
- **Texts of the sounds of the visual check:** the two rows of the list of sounds of the settings (when the check is correct, when it is not correct), in the eight languages.

## [0.0.6] - The quality analyser, the ring of LEDs and the picture of the drill head node

- **The quality analyser (`eh3d-inspect`):** judges a picture of the camera of the drill head: every hole of the job (there, round, the right size, in place, clean around), the milled channels (their width from the exact distance to the copper, how even they are, where they pinch) and the picture itself (light, contrast, focus), with a profile of tolerances (presets standard, fine pitch and prototype, kept as JSON) and one verdict: ok, not ok or cannot judge. It also suggests the level of the ring of LEDs that would give a good picture. 43 tests.
- **The ring of LEDs and the picture in the node client (`eh3d-aux`):** `Link::set_light`, `Link::light`, `Link::snapshot` and `Link::put_inspect_profile`, the colour signals (`Cue`), and the light of the status; older nodes that do not say it count as without a ring.
- **Texts of the quality check and of the light:** 58 texts in the eight languages for the window of the quality check, its tolerances and the findings of the analyser.

- **Texts of the catalogue of bits:** the three new kinds (chip-breaker routers, slot drills, diamond-coated routers), the words of the list (series, cutting length, schematic drawing, the top speed of the spindle) and what each kind of bit is for, in eight languages; and the label of the button that moves only the design.

- **The node of the drill head through the server:** `eh3d-aux::Link::via_server` asks EH3D-SERVER for the status, the measure and the camera of the node of a machine (`/api/v1/machines/:id/head/...`), and the status carries `camera_model` (`ov2640` or `ov5640`, empty for an older node). 2 more tests.
- **Texts of the head node window:** the note of the way through the server, the missing server data and the camera model, in the eight languages.

## [0.0.5] - The client of the drill head node, the texts of the catalogue, the voices and the head node window

- **Texts of the spoken voices and of the catalogue:** the words of the list of sounds (voice, female, male) and of the catalogue of bits, in the eight languages.
- **Texts of the node of the drill head:** the window, its three ways of reaching the node, the camera, the speed, the temperature and the protect output, in the eight languages.
- **The client of the node of the drill head (`eh3d-aux`):** the status, the measure of the table and the camera as MJPEG of EH3D-NODE-BUNGARD-AUX read into types, the requests with the token, a small blocking HTTP client and a splitter that cuts the MJPEG stream into pictures.
- **Texts of the catalogue of bits:** the kinds of bit and the notes of the window, in the eight languages.
## [0.0.4] - The height map of the table, the texts of the new windows, a serial port for GRBL and the fleet of machines

- **Texts of the calibration of the table:** the tab, its buttons and its readings, in the eight languages.
- **Height map of the table (`eh3d-levelling`):** the grid a probe measures, the surface between the points, the tilt and the spread of the table, the correction of the Z of a job (also along long moves) and a text form to keep the map with the machine.
- **Texts for the job that was left in the middle:** the question and its two answers, in the eight languages.
- **Texts of the tabs and the maker:** the panel window's routing with tabs (how many, how wide, where) and the maker of the board, in the eight languages.
- **One more text of the window of the settings:** the check box that shows the tools window when a file is opened, in the eight languages.
- **A real serial port for GRBL:** `eh3d_controller::serial` cuts the bytes of a port into lines (tested on its own, with noise and half lines) and, with the `serial` feature, `SerialTransport` opens a port and lists the ports of the computer; it plugs into `GrblController` as its line transport.
- **The README tells who uses the crates:** the status table lights "Use by other programs" (the crates of the workspace are used by DRILLPRO, ROUTEPRO, BUNGARD-GUI, MOTIONCNC and SERVER by path; the project container still has no program), the overview lists the six shared crates and the count of tests is 100.
- **Several machines and stations at once (`eh3d-fleet`):** the answer of the EH3D server for the whole fleet read into types a central screen can use (the stations and their machines, what each one is doing, its job with the progress and the question it waits at, the jobs ready), the free machine that should take the next job, the counts and a text table; and the words of a screen of the machines in the eight languages.
- **Six more themes and the file window of every program:** Midnight, Paper (a light one), Forest, Retro terminal, Ocean and High contrast join the five themes, with their names in the eight languages and `ThemeId::text` to name them; the new crate `eh3d-filedialog` holds the state of an Open and Save window (places, folder list, sorting, filters, typed path, file name, the extension a saved file gets and the check before a file is replaced) so that every program draws the same window over it; the controller interface can be told the travel limits of the machine (`set_travel`); and the words of the menus, the settings and the dialogs of a machine-control program are in the eight languages.
- **The stored positions in the simulated controller and more words of the window:** `G28` and `G30` go to the two stored positions and `G28.1` and `G30.1` store where the machine is; the words of the left panel of a machine-control window (the zero buttons, the Operator and GCode tabs, the active G-codes and M-codes, the inputs and the outputs) are in the eight languages.
- **The words of a machine-control window:** position, work and machine, jog, step, home, zero, unlock, program, MDI, log, toolpath, the states of a controller and the rest of what such a program shows, in the eight languages of the interface.
- **A simulated machine controller (`eh3d_controller::sim`):** a machine in memory that takes the orders (jog, home, zero, stop, emergency stop, spindle, coolant) and the lines of basic G-code (`G0`, `G1`, `G90`, `G91`, `G20`, `G21`, `G92`, `M3` to `M9`) as a real one would, with travel limits that raise an alarm, so that a machine-control program can be tried with nothing connected.
## [0.0.3] - The outlines of a board in Gerber and DXF as routing paths, the chord angle for arcs and circles, and the words for the recent files and the counts of holes

- **One word more:** "Recent" in the eight languages of the interface.
- **The drawing of a board in DXF as a routing path:** `eh3d_bungard_formats::dxf` reads the lines, the light polylines with their bulges, the arcs and the circles of a text DXF file (in the unit its header says) into the paths of a plot, joining the strokes that meet end to end; the archives list `.dxf` files as routing files.
- **Two words for the count of the holes:** "Holes drilled in all" and "Reset the counts" in the eight languages of the interface.
- **The chord angle of a routing configuration as chords of a circle:** `hpgl::segments_for_chord` turns the angle of the routing configuration line (a minus sign only gives the direction) into the chords of a whole circle, for the arcs and circles of a plotter file.
- **The outline of a Gerber board as a routing path:** `eh3d_bungard_formats::gerber` reads the lines and the arcs of the profile layer (the `G01`, `G02` and `G03` strokes, in millimetres or inches, with the zeros left out at either end) into the paths of a plot, which can be written as an HP-GL file with `to_hpgl`; the archives list the usual outline extensions as routing files.
- **A message for a job that does not fit:** the interface words in eight languages have the sentence that tells the operator to move the board or make the table bigger.

## [0.0.2] - The shared crates for the drilling and routing programs and the modern interface: the job view, the ZIP archives, the machine files with the HP-GL arcs and circles, and the machine controllers

- **A lighter debug build:** the `dev` profile keeps the file and line of a panic (`debug = "line-tables-only"`) but not the full debug symbols, so the `target` folder of a test run is about a tenth of its size.
- **README in the universal layout, in eight languages:** the README has the same sections in the same order as the other repositories of the family (status, overview, architecture, features, security, interfaces, configuration, quick start, testing, structure, roadmap and known limits), with figures taken from the code and from the tests that were run. The Architecture and Repository Structure pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`, and the CI checks that they match.
- **The layout of the README is checked in the CI:** the validator runs `tools/universal_readme_check.py` and lists, as a warning, what differs from the universal layout of the README.
- The banner follows the animated style of the ecosystem banners, with the name, the kind and the technologies of the project.
- **Arcs, circles and scaling in the HP-GL reader:** `AA`, `AR` and `CI` are cut into chords (72 to a whole circle by default, or the number the machine configuration asks for through `parse_with`), and `SC` with `IP` lays the user units on the plotter points; the pen-up arcs only move the pen.
- **The files of the Bungard machines (`eh3d-bungard-formats`):** the Excellon drill file, the HP-GL routing file, the tool tables of both programs and the machine configuration, read and written back keeping their layout; DRILLPRO, ROUTEPRO and the modern interface read them from here.
- **The interface of the machine controllers (`eh3d-controller`):** one `Controller` trait (status, orders, lines of G-code) and the families of the market that a machine-control program can drive, with what each one needs to be reached (serial, network, USB or a library of the maker) and whether it is free software; the first controller written against it is GRBL (which grblHAL and FluidNC share), tested on a scripted line transport. The families without a controller yet are listed as not driven.
- **A workspace with shared crates:** `eh3d-view` (the scene of a job on the work table, the plan and the turnable isometric views, panel copies, the five themes and the interface words in eight languages) and `eh3d-archive` (jobs packed in ZIP archives: the drilling and routing files inside, names that could leave the work folder never listed, limits on the number and the size of the files).

## [0.0.1] - The project container, its metadata and the machine telemetry types

- **The `.eh3dproj` container:** a ZIP archive with `project.json`, `integrity.json` and the folders `layers/`, `toolpaths/` and `vision/`. The writer is deterministic and writes through a temporary file.
- **Integrity:** the SHA-256 and the size of every entry are recorded; a package with a changed, missing or unlisted entry does not open.
- **Limits:** entry names that could leave the package folder are refused, and so are oversized or too many entries.
- **Metadata:** name, identity, units, zero of the workpiece, layers, tools, toolpaths and fiducials, validated, with a semantic format version: a newer major version is refused and an older one needs a registered migration (there is none yet).
- **Telemetry types:** axes, spindle, machine states and GRBL alarm codes with their severity.
- **Theme:** the colours of the suite with a contrast check.
- **Firmware base of the nodes:** `firmware_base/`, the part of the ESP32 node firmware that every EH3D node shares (settings, network, panel server, users and sessions, update with rollback, log, self-protection, tests), written once with the project's name as placeholders, and `tools/sync_firmware_base.py` to write it into the nodes and check the copies. CI runs the check.
- 29 tests, `cargo clippy -D warnings` and `cargo fmt --check` in CI on Linux and Windows, and a repository check of the manifest, the versions, the changelog and the eight README files.
