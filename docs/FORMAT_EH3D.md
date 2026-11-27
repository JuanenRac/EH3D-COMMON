# The `.eh3d` project format

A project of the EH3D Fabrication Suite can be one file with the extension `.eh3d`. It is a standard ZIP archive (deflate only), so any archive tool can look inside it. The format version described here is **1.0.0**. The reader and writer are the crate `eh3d-project` (on top of `eh3d-container`); the decision and the reasons are in `docs/adr/0002-eh3d-project-container.md`.

The `.eh3dproj` 1.0.0 package (`docs/FORMAT.md`) is not replaced: it lives inside an `.eh3d` file as the branch `manufacturing/`, and a program that only knows `.eh3dproj` keeps reading what it always read.

## The idea: one branch for each program

Every program of the suite writes **its own branch** and only its own. The other programs may read it, import from it and keep it as it is, but never write it. To change the board, the board program opens the file. A program that meets a branch it does not know keeps it **byte for byte**.

```
project.json            the record of the project: the index of the branches
integrity.json          SHA-256 and size of every other entry
pcb/                    a branch: its files and its branch.json
  branch.json
  board.json
  origin/               the original file of an import and its report (optional)
    report.json
    design.brd
cad/  3dp/  laser/  cnc/  pnp/  vision/ ...   other programs, the same shape
common/                 what belongs to the project as a whole
  branch.json
  meta/meta.json        metadata (optional)
manufacturing/          a .eh3dproj 1.0.0 package, entry for entry
docs/<program>/         documentation of a program (a branch of its own)
docs/project/           documentation of the project as a whole
attachments/            data sheets, manuals and any other file attached to the project
x-<name>/               an extension of someone else: kept, never read
audit/log.jsonl         the audit log (optional, not a branch)
```

A **branch** is a top-level folder whose name is 1 to 32 characters of lowercase ASCII letters, digits and hyphens, starting and ending with a letter or a digit. `docs` and `audit` are not branches; `docs/<name>` (the same name rule) is. Any such folder is a branch: a program that appears in a later version writes a branch that an older program keeps. The names the crate knows (`KNOWN_BRANCHES`: `pcb`, `cad`, `3dp`, `laser`, `cnc`, `pnp`, `vision`, `routepro`, `drillpro`, `stencilcam`, `gerberviewer`, `common`, `manufacturing`, `docs`, `attachments`) are only for showing in a list. Folders that start with `x-` are extension folders: no record is needed, nothing is read, everything is kept and listed in the integrity record.

A file at the top level other than the two records, a folder that is not a valid branch name, and a file directly under `docs/` have no place in the layout and are refused (`EH3D-PKG-009`).

## `project.json`

```json
{
  "schema": 1,
  "format_version": "1.0.0",
  "container_version": "1.0",
  "id": "00000000-0000-4000-8000-000000000001",
  "name": "Synthetic test project",
  "units": "millimetres",
  "created": "2000-01-01T00:00:00Z",
  "modified": "2000-01-01T00:00:08Z",
  "branches": [
    { "id": "pcb", "app": "pcb", "format": "1.0", "app_version": "0.1.0", "content_hash": "..." }
  ],
  "seal": null,
  "signature": null
}
```

* `format_version` is `major.minor.patch`. A reader refuses a newer major or the same major with a newer minor (`EH3D-PKG-013`); it ignores fields it does not know and keeps them when it writes again.
* `container_version` is the version of the rules of the container (names, ZIP structure, integrity), as `major.minor`.
* `branches` is the **index**: one line for every branch in the file, saying who wrote it and the content hash its record states. It lets a program show the file without opening the branches. The index must agree with the branches: a line without a branch or a branch without a line is `EH3D-PKG-015`, a content hash that differs is `EH3D-PKG-016`.
* The clock is never read by the format: `created` and `modified` are written as the caller gives them, so the same content gives the same bytes.

## `branch.json`

Every branch has the record `<branch>/branch.json`, written by the container, never by the program:

```json
{
  "schema": 1,
  "branch": "cad",
  "app": "cad",
  "format": "1.0",
  "app_version": "0.1.0",
  "content_hash": "<sha256 of the files of the branch>",
  "depends": [
    {
      "branch": "pcb",
      "hash": "<content hash of pcb when this branch was derived>",
      "format": "1.0",
      "artifact": "outline2d",
      "artifact_version": 1,
      "uses": ["outline.json"]
    }
  ],
  "seal": null,
  "signature": null,
  "encryption": null
}
```

* `app` is the program that owns and writes the branch; `format` is the format of **this branch** (`major.minor`, each program versions its own); `app_version` is the version of the program that wrote it.
* `content_hash` is the SHA-256, in lowercase hexadecimal, of the canonical JSON array of `[name, size, sha256]` for every file of the branch except `branch.json`, with names relative to the branch and sorted by their bytes. Two branches with the same files have the same hash whatever their records say. A different hash from the one the record states is `EH3D-PKG-016`.
* An unknown field in `project.json`, `branch.json` or any record is **kept** when a program writes the record again.
* A reader that finds a branch `format` with a newer major (or minor, where the program refuses it) than it supports reports `EH3D-PKG-024` for that branch only; the rest of the file stays usable and the branch is kept as it is.
* All JSON is written in canonical form (sorted keys, no spaces, fixed number form) so that the bytes do not depend on the program.

## References between branches and derived data

A branch that is built from another one (the case from the outline of the board) says so in `depends`: **the branch, the content hash it had, the format, what was taken (`artifact`, `artifact_version`) and which files (`uses`)**. The reference never points outside the file and never to the branch itself.

When the source branch changes, its hash changes and the reference no longer matches: the derived branch is **out of date**. This is not an error. The file stays valid and the reader reports `EH3D-PKG-019` (a warning) with `Project::stale_refs`. The program that owns the derived branch derives again and saves with a fresh reference (`Project::dependency_on`). A reference to a branch that is not in the file, a malformed one or one to itself is `EH3D-PKG-018`. No program ever rewrites a derived branch for another.

## Permissions

* A program writes the branches it owns (`Writer`): its own and `docs/<its name>`. Writing another's branch is refused (`EH3D-PKG-017`), also when the writer's name differs from the `app` of the branch.
* `common`, `docs/project` and `attachments` belong to the project as a whole; any program may write them.
* `manufacturing` belongs to the program that prepares the job (a CAM program, or the importer of a `.eh3dproj`).
* Saving a branch writes that branch and the two records of the project and copies every other entry through: **their bytes do not change**. An entry that does not change (of another branch, an attachment, or a file of the saved branch with the same content) is **not compressed again**: its compressed bytes and its CRC are copied as they are. An entry that another tool stored in another way (another method, time stamp or attributes) is compressed again once, so that the file keeps the form of the suite. The file is replaced atomically (a temporary file in the same folder, then a rename), so a crash never leaves half a project, and **before it replaces the old one every entry of the new file is read back and checked** against its CRC and its SHA-256: a damaged entry, or one that is not what the record says, refuses the save and leaves the file as it was.
* This is a rule of the programs that follow the format, not a defence against someone who edits the file by hand. Against that there are the signature and the seal below.

## The `manufacturing` branch

The files of the branch are the entries of a `.eh3dproj` 1.0.0 package (`project.json`, `layers/`, `toolpaths/`, `vision/`), kept as the package has them, except that `integrity.json` of the package is not stored (the integrity of the container covers every entry) and is written again, identical, when the branch is taken out. `manufacturing_update` reads a package into the branch and `Project::export_manufacturing` writes a package that is, **byte for byte, the original** (the three reference packages of `tests/golden` do the round trip in the tests). The package is read under the rules of its own format, then checked against the strict names of this container.

## Optional records

All of the following are optional. When a record is there, it is checked; when it is not, the file is valid.

### Metadata: `common/meta/meta.json`

`title` and `description` (a text in each language: the key is the language code, and a text that is not empty needs `en`), `authors` (name, contact), `license` (an SPDX identifier), `tags`, `revision`, `customer`, `standards`, `materials` and `links`. Language codes are the eight of the project and **no others**: `en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt-PT`.

### Documentation: `docs/<branch>/`

The documentation of a program is a branch of its own, so it takes part in the integrity and in the signature. `docs/<branch>/index.json` lists the documents: `path`, `kind` (`markdown`, `pdf`, `image`, `text`, `report`, which must agree with the extension), `title` in each language, `lang` and `translation_of` (the document this one translates, in another language). Every path must be a safe name of a file that is there and be listed once. `docs/project` documents the project as a whole (assembly and use guides, test reports, a readable history of changes); each program adds documents about its own branch to `docs/<program>`.

### Origin of an imported file: `<branch>/origin/`

A program that imports a format of another program (EAGLE, KiCad, Gerber, STEP, 3MF, SVG, G-code...) keeps in its branch, under `origin/`, the **original file as it was read** and `origin/report.json`: the format, the name, the SHA-256 and the size of the file; the converter and its version; what was read, what was not read and what an export to the same format cannot say (each as a count with the reason). With that, the way back (export to the other format) tells what was lost instead of losing it unseen. The report is checked against the files that are there (hash, size, name under `origin/`). Export to another format is done by the same program from the same branch; the container stores no export. An imported file that is itself called `report.json` is kept as `origin/original-report.json`, so that the report does not write over it.

## Attachments: `attachments/`

A project may carry any file: data sheets, manuals, office documents, projects and drawings of other programs, video, sound, anything. They live in **one shared branch**, `attachments/` (format `1.0`), written by the module `eh3d_project::attachments`. The reasons are in `docs/adr/0003-eh3d-attachments.md`.

```
attachments/
  branch.json                    the record of the branch, as for any branch
  index.json                     what each attachment is
  blobs/<sha256>/00000           the bytes of a file, under the SHA-256 of its content
  blobs/<sha256>/00001           a large file continues in more parts
```

No name given by a person is ever a path inside the project: the paths are made of the hash and a number.

### The index

```json
{
  "schema": 1,
  "attachments": [
    {
      "sha256": "<SHA-256 of the whole file: the identifier>",
      "name": "NE555.pdf",
      "size": 734003,
      "kind": "pdf",
      "storage": "inside",
      "title": { "en": "Timer data sheet" },
      "components": [
        { "reference": "U3" },
        { "value": "10k" },
        { "manufacturer": "Example", "part_number": "NE555P" }
      ]
    },
    {
      "sha256": "<SHA-256 the file at the address must have>",
      "name": "manual.pdf",
      "kind": "pdf",
      "storage": "link",
      "url": "https://example.invalid/manual.pdf"
    }
  ]
}
```

* A file is **stored once**: its identifier is the SHA-256 of its content, so attaching the same content again adds the new components, title and address to the record that is there. The attachments are sorted by identifier.
* `name` is a safe file name: no folder, no control, invisible or text-reordering characters, no leading or trailing dots or spaces, no leading dash, no reserved name of Windows, at most 150 bytes. A reader cleans it again before it makes a file.
* `kind` is `pdf`, `markdown`, `text`, `image`, `report` (`.json`), `office` (doc docx docm odt rtf xls xlsx xlsm ods csv tsv ppt pptx pptm odp and their templates), `project` (EAGLE, KiCad, `.eh3d` and other design projects), `cad` (STEP, STL, DXF, DWG, IGES, OBJ, PLY, 3MF), `video`, `audio`, `archive` or `other`. A word a reader does not know is `other`. Anything may be attached.
* `components` says which pieces of the design use it. Each entry names **one** of a designator (`reference`), a `value`, or a `part_number`, the last one with or without its `manufacturer`. Designators, part numbers and manufacturers are compared without regard to case; a value exactly (`1m` is not `1M`). `AttachmentIndex::for_component` gives the attachments of a piece, the ones of its designator first, then of its part number with the manufacturer, of its part number, and of its value.
* `storage` is `inside` (the default) or `link`. A link keeps only the address (`https://` or `http://`, with no user name or password in it) and the SHA-256, and the size when it is known. **Nothing is ever downloaded or opened by itself**; `matches_file` tells whether a file a person got from the address is the one announced. Attaching the file itself later makes the link a file inside and keeps the address.
* `added` is the time as the caller wrote it. Unknown fields are kept.

### Dangerous files

One classification (`eh3d-filekind`) is used by the container, the viewer and the panel. Programs, scripts, office documents with macros, compressed archives (they are not looked into) and pages that can run code (HTML, SVG) are accepted, kept like any other file and marked `danger`: `executable`, `script`, `macros`, `archive` or `active-content`. The mark comes from the extension, from the first bytes (a program behind a harmless name) and, for office documents, from the macros named inside them; a document that cannot be looked into counts as one with macros, and so does one whose archive does not end where its list of entries says, or lists no entry.

* A file of a **kind the suite does not know** has no `danger` word in the record, and still asks like a dangerous one (`Attachment::needs_confirmation`, `AttachError::NeedsConfirmation` with no danger).
* Taking a dangerous or unknown file out needs the agreement of a person (`Consent::Given`, `--confirm-dangerous`). The mark of the record is not trusted alone: the name and the content of the file are looked at again, and a refused or partial file is removed.
* A file taken out gets its safe name inside the folder chosen, never replaces a file that is there, is not executable and is marked on Windows as coming from outside.
* Nothing of an attachment is ever run or opened by this crate. A program that opens one asks first, and never launches a program or a script (`Danger::never_launched`).
* The `attachments` branch is not offered to the programs as files to take (`suiteview`); `extract-branch` of it gives the parts under their hash, which have no extension.

### Limits and memory

* One attachment may be up to **1 GiB** (`AttachmentLimits`, setting `attachments.max_mb`, 1 to 4096). The whole project stays under the limit of the container (4 GiB minus one byte), with 16 MiB left free for the records.
* Files are measured, stored, checked and taken out in blocks. A file larger than 32 MiB is stored in parts of 32 MiB, and reading holds one part at most, so a large attachment is never whole in memory. The content is checked against its SHA-256 before the last part is handed over, and a file that changes while it is being attached is refused with nothing written.
* The parts are far under the limit of one entry, so **a reader that knows nothing of attachments** (any reader of format 1.0.0) opens the project, keeps the branch byte for byte when it saves its own, and verifies it like any unknown branch. Projects made before attachments open as before and have none.
* A key file of a signature is never put into a project. A sealed project or branch is not written, and a branch of attachments of a newer format is refused (`EH3D-PKG-024`). Writing the branch removes a signature of the project, and one of the branch unless a signer is given.

### For a program

```rust
use eh3d_project::attachments::{self, AttachRequest, Attacher, ComponentLink, ComponentQuery, Consent};

// attach (any program; the file is read in blocks)
let request = AttachRequest::new().component(ComponentLink::designator("U3"));
let done = Attacher::new("pcb", env!("CARGO_PKG_VERSION")).attach_file(project, path, &request, &options)?;
let mut project = done.project;          // done.notices says what to tell the person

// from a piece of the design
let index = attachments::read_index(&mut project)?;
let found = index.for_component(&ComponentQuery::new().reference("U3").value("NE555"));

// show it inside the program: a name, a size, the danger and a reader as often as needed
let source = attachments::AttachmentSource::open(project.path(), found[0].id())?;
let bytes = source.reader()?;            // std::io::Read

// take it out (NeedsConfirmation for a dangerous or unknown file until the person agrees)
attachments::extract(&mut project, found[0].id(), folder, Consent::NotAsked, &limits)?;
```

The windows of the programs use the panel `eh3d-attachpanel` (`docs/ATTACHMENTS_PANEL.md`), which holds all of this and the viewer (`eh3d-viewer`, `docs/VIEWER.md`); its `ProjectSource` hands an attachment to the viewer without writing a file first.

`attach_link`, `set_components` and `detach` complete it. The words of the warnings in the eight languages are in `suitetext` (`attachments::danger_word`, `AttachmentDangerous`, `AttachmentLinkOnly`, `AttachmentTooLarge`).

## Security (all optional)

A project without any of this is valid. Integrity is always there; everything else is a switch of the project and of the program, off by default.

| Layer | What it gives | State |
|---|---|---|
| Integrity | SHA-256 of every entry (`integrity.json`), content hash of every branch, content hash of the whole file (the audit log left out). Detects damage and accidental change. It is **not authenticity**: whoever can change the entries can rewrite the record. | Done |
| Signature | A signature of a branch (in its record) or of the project (in `project.json`) over the canonical JSON of the record without its `signature`, with a fixed label (`eh3d-branch-signature-v1`, `eh3d-project-signature-v1`). The record carries the algorithm, the identifier of the key and the value in hexadecimal; **the keys are never in the file** (a private key is not read or kept by the crate, public keys are given by the program through the `Verifier` trait). The record of a branch states its content hash, so a signed branch is covered file by file. Wrong signature `EH3D-PKG-021`; required and missing, or the key unknown `EH3D-PKG-022`. | Done, with Ed25519 (see Signatures below) |
| Encryption | A branch can be stored encrypted; its record says the algorithm, the key identifier and the parameters, and only its `branch.json` stays readable. The other branches stay readable. Reading an encrypted branch without its key is `EH3D-PKG-023`; the branch is kept byte for byte and can be extracted as it is. | The record and the refusal are done; the cipher is pending |
| Seal | A branch or the whole project can be sealed read-only (who, when, why). A write to a sealed branch or project is refused (`EH3D-PKG-020`). A seal is a rule of the programs; with a signature over the record it is also provable. | Done |
| Audit log | `audit/log.jsonl`: one canonical JSON line for each save (action, program, program version, time given by the caller, branch). It is outside the content hash, so it does not change the content of the project. A program decides whether to write it. | Done |

Nothing in a file is ever run, and the reader refuses before it decompresses more than its limits (below).

## Signatures

The algorithm of the suite is **Ed25519** (RFC 8032), written in the record as `ed25519`:

```json
"signature": { "algorithm": "ed25519", "key_id": "ed25519:<32 hex digits>", "value": "<128 hex digits>" }
```

* **The message** is the label of the kind (`eh3d-branch-signature-v1` or `eh3d-project-signature-v1`), a line feed, and the canonical JSON of the record without its `signature`. A record with a number that has no canonical form (an integer that no double equals) is never signed and a signature on it is never accepted.
* **The key identifier** is `ed25519:` and the first 32 hex digits of the SHA-256 of the 32 byte public key.
* **Checking is strict:** a weak public key (of small order) is not trusted and a signature must be in its one valid encoding.
* **Keys are always outside the project.** A key file is two lines, a header and 64 lowercase hex digits: `eh3d-ed25519-secret-v1` (the 32 byte seed) or `eh3d-ed25519-public-v1` (the public key, in the secret file's path plus `.pub`). The person gives the path. A new secret file never replaces one and is readable only by its owner (mode 600 on Unix; on Windows every other permission is removed and only the owner is granted, with the system tool `icacls`, so a disk without permissions, such as FAT, refuses it). A key is never printed, an error never quotes a key file, the secret is wiped from memory, and a program refuses to save into a project an entry that starts like a secret key file. A secret key is best kept under a password (below); a file without one, as made before, is read as it is, and a program warns about it.

### Secret keys under a password

```text
eh3d-ed25519-locked-v1
argon2id-v19 m=65536 t=3 p=1
public <64 lowercase hex digits>
salt <32 lowercase hex digits>
nonce <48 lowercase hex digits>
sealed <96 lowercase hex digits>
```

* The password becomes a 32 byte key with **Argon2id** version 0x13 over the salt: memory in KiB, passes and lanes are the second line. New files use 64 MiB, 3 passes and 1 lane. A file that asks for more than 1 GiB, 16 passes, 16 lanes or 4 GiB of memory times passes is not read.
* `sealed` is the 32 byte seed under **XChaCha20-Poly1305** with that key and the nonce (24 bytes), then the 16 byte tag. The five lines above it, each ended by a line feed, are the associated data, so none of them can be changed. Salt and nonce are new random bytes for each file.
* The public key is in clear, so a program can say which key it is (`key_protection`) before it asks for the password. It is checked against the opened seed.
* A new password (a new key, or a key being protected) has at least 12 characters and at most 1024 bytes. A key file made before with a password of 8 to 11 characters still opens: the length is not looked at when a file is read. It is never taken from the command line, never printed and, with the derived key and the seed, wiped from memory. An error never says whether the password or the file is wrong.
* `generate_locked_key_files` makes a key; `Ed25519Signer::read_with` opens it; `protect_key_file` puts a file without a password under one (written beside it, read back and then put in its place; the old text may stay on the disk until it is written over).
* A program remembers the **path** of the key a person used (`eh3d_dock::key_store`), never the key or the password.

A program that only calls `Ed25519Signer::read` keeps reading the files without a password, and gets `KeyError::NeedsPassword` for the others.

### Replacing a signature

Signing never puts another signature over one that is there unless a person agreed: `sign_branch` and `sign_project` refuse (`EH3D-PKG-026`), `signature_to_replace` gives the one that would go, so a program asks (the words are `suitetext::S::SignatureReplace`), and `sign_branch_replacing` or `sign_project_replacing` do it. Signing again with the same key what is already signed changes nothing and is not refused. On the command line, `--replace-signature`.

### What a signature covers

| Signature | Covers | Does not cover |
|---|---|---|
| Branch (`<branch>/branch.json`) | The whole record: name of the branch, program, formats, versions, references, seal, unknown fields, and through the content hash every file of the branch | The other branches |
| Project (`project.json`) | The record of the project: identifier, name, units, times, seal, and the index with the content hash of every branch | The rest of each `branch.json` (references, seals, branch signatures), the extension folders `x-*` and the audit log |

A program that needs a whole delivery proved signs the branches **and** the project. Signing a branch (`Project::sign_branch`) rewrites only its `branch.json`, so a project signature made before keeps holding; saving a branch drops the project signature and, unless a signer is given, the signature of that branch. A sealed branch or project is not signed again.

### States

`Project::open_verified` (or `Project::signatures`) says for the project and for every branch:

| State | Meaning |
|---|---|
| unsigned | There is no signature |
| signed, valid | The signature is right, for the key named, and the files are the ones that were signed |
| INVALID | The signature is wrong, or the files, the folder name or the index are not what the record says, or the record of the branch cannot be read |
| signed, unknown key | The key (or the algorithm) is not one that was given |
| signed, not checked | No key was given, or the file was opened without the integrity check |

An invalid signature never stops the file from opening; `diagnose` reports it as `EH3D-PKG-021`, and a required signature that is not valid as `EH3D-PKG-022`. Someone who can change a file can also remove a signature, so a program that depends on signatures **requires** them (`--require-signed`, `--require-project-signature`) and keeps its own list of trusted public keys.

## Another program saved the file

Two programs may have the same project open. The one that writes second must not take the work of the first away, so **a write is refused when the file no longer holds what was read** (`EH3D-PKG-025`), and nothing is written. The program offers to read the project again or to save under another name (`suiteview::copy_project` carries what is in the file now; the words are `suitetext::S::ChangedOnDisk`).

* What counts is the **content**: the mark of a project (`Project::revision`) is made from `integrity.json`, the name, size and SHA-256 of every entry. A file that was only touched, or written again with the same content, is the same file. A file that is gone or can no longer be read is a changed one.
* Every write of the crate (`save_branch`, signing, sealing, attachments) checks that the file is still the one its `Project` opened, at once and again just before the atomic replacement. The moment between that answer and the rename is not covered, and there is no lock file.
* Between **reading and writing**, which in a window may be hours, the program keeps the mark: `suiteview::view` puts it in `own_hash` and `suiteview::warnings` refuses when it no longer holds; `suiteview::check_unchanged` asks again after a question; `appbranch::open_unchanged` and `BranchKit::update_since` take the mark itself.
* The writes a program makes itself, one after another (its window and its panel of attachments, say), are remembered while it runs and do not count as another program.

## A program and its own branch

The module `eh3d_project::appbranch` holds what every program does with its branch, so it is not written again in each one:

```rust
use eh3d_project::appbranch::{self, BranchKit, Identity};

const KIT: BranchKit<'static> = BranchKit::new("laser", "1.0", "laser.json", env!("CARGO_PKG_VERSION"));

// save: every file is read and checked before this; what is not touched stays
let (_, notices) = KIT.update(path, &Identity::default(), |_, copy| {
    copy.put("drawing/a.svg", bytes)?;
    copy.choose("drawing", "drawing/", "drawing/a.svg");
    Ok(())
})?;
// open, with what is out of date or wrong
let (project, copy, warnings) = KIT.open(path)?;
// import a foreign file, kept with the report of the conversion
let report = appbranch::conversion_report("my-program", "1.0.0", "svg", &name, &bytes, conversion);
KIT.import(path, &Identity::default(), bytes, &report, |copy, original| {
    copy.choose("drawing", "drawing/", original);
    Ok(())
})?;
// take from another branch, remembering the state of the source
KIT.take(path, "pcb", "outline.json", "outline2d", check, |copy, target| copy.set("drawing", target))?;
// list the branches, extract one
let listing = appbranch::list(path)?;
appbranch::extract(path, "pcb", dir)?;
```

To adopt it, a program keeps its formats (what a drawing or a program of a machine is, what the report counts, its own messages) and drops its copies of the working branch, the open or create step, the bounded read, the identifier, the take, the warnings and the list. The keys of the record of the branch stay the ones the program already writes, so the files made before still open. Errors are `appbranch::BranchError`, whose text is the message for the person, and the notes after a write are `appbranch::Notice`.

## The job and the window

Two more modules hold what was repeated between programs.

`eh3d_project::manufacturing` is for a program that makes something from the job of a project (a picture, stencils, isolation programs, a drilling or routing plan). `read_job` gives the layer files that `manufacturing/project.json` names (at most 256 files and 256 MiB), `layers_of`, `choose` and `result_name` pick one and name what is made from it, `keep_result` writes it under `result/` of the branch of the program with a copy of the layer under `taken/manufacturing/` and a reference to the content of the job, `state` and `describe` say what is kept and whether it is out of date. A layer that changed between the read and the write is refused. A window hands the mark of its opening (`own_hash` of `suiteview::view`) to `keep_result_since` or `save_result_since`, which refuse with `EH3D-PKG-025`, writing nothing, when another program saved the file since; a text that is not such a mark is refused too. When the job changed, the results made from the older one are dropped on the next write.

`eh3d_project::suiteview` is for a window: `view` lists the branches with their state, `banner` is what is always in sight, `warnings` is what a person has to agree to before a write, `copy_project` copies a project under another name and never over a file. Its words are in `suitetext`, in the eight languages. `suiteview::sets_key` says whether a `key = value` settings file gives a value to a key; a program asks it before it puts one of its settings files in a project, because a project is passed on and a credential must not travel in it.

## Limits and the strict container

The project container is `eh3d-container` with the strict policy: names in Unicode normal form C, no part ending in a dot or a space, no Windows device names, no duplicate once case and form are ignored, no links, devices or directory entries, no ZIP64, no encryption of ZIP entries, deflate only, flags limited, and no data after the archive. Limits of the preset: at most 50,000 entries, 4 GiB minus one byte in all, 256 MiB for one entry (1 GiB under `cad/`, `3dp/`, `cnc/`, `pnp/`, `laser/` and `manufacturing/`); a record (`project.json`, `branch.json`, an index) is read up to 16 MiB. The reader reads the central directory first, refuses before it decompresses, and stops at a time and memory budget. The limits are plain data that a program may read from its configuration file.

## Diagnostic codes

`EH3D-PKG-001` to `EH3D-PKG-014` are the container's. The project container adds these (the registry the user reads is in the documentation repository of the suite):

| Code | Meaning |
|---|---|
| `EH3D-PKG-015` | A project, branch or optional record (metadata, index of documents, report of a conversion) missing, malformed or inconsistent |
| `EH3D-PKG-016` | The content hash of a branch differs from the one its record states |
| `EH3D-PKG-017` | A write to a branch that the writing program does not own |
| `EH3D-PKG-018` | A reference between branches malformed, to itself, or pointing to a branch that is not there |
| `EH3D-PKG-019` | A branch derived from a state of another branch that has changed since (a warning) |
| `EH3D-PKG-020` | A branch or the project is sealed read-only |
| `EH3D-PKG-021` | A signature does not verify |
| `EH3D-PKG-022` | A signature is required and missing, or the key is not known |
| `EH3D-PKG-023` | A branch is encrypted and its key is not available |
| `EH3D-PKG-024` | The format of a branch is newer than the program supports |
| `EH3D-PKG-025` | Another program saved the file since it was read, so nothing was written over it |
| `EH3D-PKG-026` | There is a signature already and replacing it was not agreed to |

A code never changes its meaning and is never reused.

## Versions

The version of the file (`format_version`) is for the record layout described here; each branch has its own `format`, moved by the program that owns it, so a program can change its branch without the other programs having to change. A change that an older reader could misunderstand takes a new major version. The synthetic file `crates/project/tests/data/synthetic.eh3d` (invented data) is a reference for readers in other languages; the tests rebuild it and compare the bytes.

## The command line

```
eh3d-project info FILE [--json]
eh3d-project verify FILE [--strict] [--require-signed BRANCH[,BRANCH...]] [--require-project-signature] [--pubkey KEY.pub]...
eh3d-project extract-branch FILE BRANCH OUT_DIR
eh3d-project extract-branch FILE manufacturing --eh3dproj OUT.eh3dproj
eh3d-project keygen KEYFILE
eh3d-project sign FILE --key KEYFILE [--branch BRANCH[,BRANCH...]] [--project]
eh3d-project attach FILE PATH [--name NAME] [--title TEXT] [--url URL] [LINKS] [--max-mb N]
eh3d-project attach-link FILE URL (--sha256 HEX [--size BYTES] | --of PATH) [--name NAME] [--title TEXT] [LINKS]
eh3d-project attachments FILE [--json] [LINKS]
eh3d-project extract-attachment FILE ID OUT_DIR [--confirm-dangerous] [--max-mb N]
eh3d-project attachment-links FILE ID (add|remove) LINKS
eh3d-project detach FILE ID
```

`verify` prints the state of every signature (`signature project: ...`, `signature <branch>: ...`) and the findings with their codes, and ends with `VERIFY=PASS` or `VERIFY=FAIL` (exit 0, 2); signatures are judged with the public keys given by `--pubkey`. `keygen` writes `KEYFILE` and `KEYFILE.pub` and prints the identifier of the key, never the key. `sign` checks every branch asked for before it signs the first. Wrong use is 64 and any other failure 1. `extract-branch`, `extract-attachment` and `keygen` never replace a file that is there; nothing is written except by them and by `sign`, `attach`, `attach-link`, `attachment-links` and `detach`, which rewrite the project.

`LINKS` are the pieces of the design: `--ref R5`, `--value 10k`, `--part NUMBER` and `--maker-part MAKER NUMBER`, each as often as needed. `ID` is the identifier of an attachment or its first 8 digits or more. `attach-link --of PATH` takes the hash and the size from a file the person already has. `extract-attachment` of a dangerous file without `--confirm-dangerous` writes nothing and exits 3.
