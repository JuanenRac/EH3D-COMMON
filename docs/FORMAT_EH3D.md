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
x-<name>/               an extension of someone else: kept, never read
audit/log.jsonl         the audit log (optional, not a branch)
```

A **branch** is a top-level folder whose name is 1 to 32 characters of lowercase ASCII letters, digits and hyphens, starting and ending with a letter or a digit. `docs` and `audit` are not branches; `docs/<name>` (the same name rule) is. Any such folder is a branch: a program that appears in a later version writes a branch that an older program keeps. The names the crate knows (`KNOWN_BRANCHES`: `pcb`, `cad`, `3dp`, `laser`, `cnc`, `pnp`, `vision`, `routepro`, `drillpro`, `stencilcam`, `gerberviewer`, `common`, `manufacturing`, `docs`) are only for showing in a list. Folders that start with `x-` are extension folders: no record is needed, nothing is read, everything is kept and listed in the integrity record.

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
* `common` and `docs/project` belong to the project as a whole; any program may write them.
* `manufacturing` belongs to the program that prepares the job (a CAM program, or the importer of a `.eh3dproj`).
* Saving a branch writes that branch and the two records of the project and copies every other entry through: **their bytes do not change**. The file is replaced atomically (a temporary file in the same folder, then a rename), so a crash never leaves half a project.
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

A program that imports a format of another program (EAGLE, KiCad, Gerber, STEP, 3MF, SVG, G-code...) keeps in its branch, under `origin/`, the **original file as it was read** and `origin/report.json`: the format, the name, the SHA-256 and the size of the file; the converter and its version; what was read, what was not read and what an export to the same format cannot say (each as a count with the reason). With that, the way back (export to the other format) tells what was lost instead of losing it unseen. The report is checked against the files that are there (hash, size, name under `origin/`). Export to another format is done by the same program from the same branch; the container stores no export.

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
* **Keys are always outside the project.** A key file is two lines, a header and 64 lowercase hex digits: `eh3d-ed25519-secret-v1` (the 32 byte seed) or `eh3d-ed25519-public-v1` (the public key, in the secret file's path plus `.pub`). The person gives the path. A new secret file never replaces one and is readable only by its owner (mode 600 on Unix; on Windows the inherited permissions are removed and only the owner is granted, with the system tool `icacls`, so a disk without permissions, such as FAT, refuses it). A key is never printed, an error never quotes a key file, the secret is wiped from memory, and a program refuses to save into a project an entry that starts like a secret key file. The key files have no password: keep the secret one where only you can read it.

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
```

`verify` prints the state of every signature (`signature project: ...`, `signature <branch>: ...`) and the findings with their codes, and ends with `VERIFY=PASS` or `VERIFY=FAIL` (exit 0, 2); signatures are judged with the public keys given by `--pubkey`. `keygen` writes `KEYFILE` and `KEYFILE.pub` and prints the identifier of the key, never the key. `sign` checks every branch asked for before it signs the first. Wrong use is 64 and any other failure 1. `extract-branch` and `keygen` never replace a file that is there; nothing is written except by `extract-branch`, `keygen` and `sign`.
