# 0002 - The `.eh3d` project container: one file, one branch for each program

Status: accepted. The core (branches, integrity of every branch, references with hash, metadata, documentation, origin of imports, seal, audit log, the signature record and its checks) is implemented in the crate `eh3d-project`. Encryption of a branch and a signature algorithm are designed here and pending (see Pending).

## Context

Each program of the suite (the board, the 3D design, the printing, the laser, the CNC, the pick and place, the manufacturing step) used to have its own file. Moving a design from one program to the next meant exporting and importing, losing the link between the pieces: nobody could tell that the case had been drawn for a board that has changed since. The `.eh3dproj` 1.0.0 package carries only the manufacturing data (layers, toolpaths, vision) and must keep working for the CAM programs, the server and the Studio.

The owner decided (D-C1 to D-C4): the extension is `.eh3d`, one project is one file; the `.eh3dproj` package stays as a branch called `manufacturing`; links between branches are references with hash and version, and a derived branch whose source changed is noticed; each program writes only its own branch. The owner added that the format is universal and every program imports and exports the formats of others, and that the container is rich: optional security, metadata and documentation.

## Decisions

1. **Name and shape.** `.eh3d`, a ZIP archive under the strict policy of `eh3d-container` (names, ZIP structure, SHA-256 of every entry, limits, deterministic and atomic writing, stable `EH3D-PKG-NNN` codes). The records are canonical JSON. Nothing in a file is ever run.
2. **Branches are folders.** A branch is a top-level folder with a valid name (1 to 32 characters of `a-z`, `0-9` and `-`), plus `docs/<name>` for documentation. The set of names is open: the reader does not need a list of programs, so a program of a later version writes a branch that an older one keeps. `common/` is the project as a whole, `manufacturing/` is a `.eh3dproj` 1.0.0, `x-*/` are extensions of others (kept, never read), `audit/` is the log and is not a branch.
3. **Records.** `project.json` (the index of the branches, the version of the format, optional seal and signature) and `branch.json` for every branch (program, format of the branch, version of the program, content hash, dependencies, optional seal, signature and encryption). Fields that a program does not know are kept when it writes the record again.
4. **The hash of a branch** is the SHA-256 of the canonical array of `[name, size, sha256]` of its files without `branch.json`. It does not depend on the record, on the order, nor on how the file was compressed.
5. **References** between branches are `{branch, hash, format, artifact, artifact_version, uses}` inside the same file. The reference states the hash of the source when the data was derived; a different hash now means the derived data is **out of date**, reported as the warning `EH3D-PKG-019`, never as an error: the file is valid and the owner of the derived branch updates it. A dangling or self reference is the error `EH3D-PKG-018`.
6. **Permissions.** A program writes its own branch and `docs/<its name>`; `common` and `docs/project` are shared. Writing a branch of another program is `EH3D-PKG-017`. Saving a branch copies every other entry through and **never changes its bytes** (tested with a fixed seed). A branch that is not known is kept byte for byte.
7. **`manufacturing`** keeps the `.eh3dproj` 1.0.0 behaviour: the package goes in and comes out byte for byte (tested with the three reference packages). The `.eh3dproj` readers and writers and their golden corpus are not changed by this work; only new codes were added to the list of the container.
8. **Imports and exports.** A program that imports a foreign format keeps the original file and a report (`origin/report.json`: what was read, what was not, what an export cannot say) inside its own branch. Readers and writers of formats live once in the shared crates, not in each program. The container stores no export; the program exports from its branch.
9. **Metadata and documentation** are optional and checked when present: `common/meta/meta.json` (text in the eight languages of the project and no others, English required as the fall-back), and `docs/<branch>/index.json` (documents of kinds markdown, pdf, image, text and report, with titles and translations). They are branches or parts of branches, so they are covered by integrity and by signatures.
10. **Security is optional, per project and per program, off by default.** Integrity is always on (it detects damage, it does not authenticate).
    * *Signature* of a branch or of the project over the canonical record without its signature, with a fixed label for each kind. The file carries the algorithm, the key identifier and the value; **no key is ever stored in the file**, and the crate never reads a private key. The program gives a `Signer` to sign and a `Verifier` (that knows the public keys) to check. Codes 021 and 022.
    * *Encryption* by branch: the record names the algorithm, the key identifier and parameters, the other branches stay readable. A branch that says it is encrypted is refused for reading (`023`) and kept and extracted as it is.
    * *Seal* read-only of a branch or of the project with who, when and why (`020`).
    * *Audit log* `audit/log.jsonl`, one canonical line for each save, outside the content hash.
11. **Versions.** The file has `format_version` (1.0.0); each branch has its own `format` (`major.minor`). A newer major or minor of the file is refused (`013`). A newer format of a branch is reported for that branch (`024`) and the rest stays usable.
12. **Codes.** `EH3D-PKG-015` to `EH3D-PKG-024` are added to the registry. Codes 001 to 014 do not change. 019 is the only code that is a warning (`PkgCode::is_warning`).
13. **Clock and randomness.** The crate reads no clock and no random source: times come from the caller, so the same content gives the same bytes.

## Defaults taken without asking (conservative, change them here if the owner decides otherwise)

* Branch format versions are `major.minor` and a program supports a range it declares; the crate does not interpret the contents of any branch except `manufacturing` and the optional records.
* A project written by this version has `format_version` 1.0.0 and refuses a newer minor, as the strict presets of the container do.
* A language outside the eight (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt-PT`) in a metadata text or an index is refused, and so is a text without English.
* The audit log is written only when the program passes a time; the format does not decide for it.
* The limits are 50,000 entries, 4 GiB minus one byte in all, 256 MiB for one entry (1 GiB in the branches that may hold models and toolpaths); all are data of the preset.
* The signature of the project does not cover the branches' files by itself: it covers the index, which states the hash of each branch. A program that requires signed branches asks for it with `require_signed` and `require_project_signature`.
* A seal is a rule of the programs that follow the format; it can be proved by also signing the record.

## Options left out

* **One file for each program plus a manifest of links.** Rejected: the links break when files are moved or copied apart, and the owner asked for one file.
* **A single tree where every program writes anywhere.** Rejected: two programs rewriting the same data and one overwriting the other (D-C4).
* **A database file (SQLite) as the container.** Rejected: it cannot be inspected with any archive tool, is hard to check against hostile input and does not give the same bytes for the same content.
* **Signing inside the ZIP (the signature of ZIP tools).** Rejected: it differs by tool and platform and does not cover the branches as the format needs.

## Pending, with the reason

* **A signature algorithm.** The natural choice is Ed25519. No pure Rust crate for it is in the offline cache of the build (the cache holds `ring`, which needs a C compiler on every target and a new tree of dependencies for all the programs). Until a crate is accepted, the traits `Signer` and `Verifier` are the point of extension and the checks are tested with a double. Adding the algorithm changes no byte of the format.
* **Encryption of a branch.** The record and the refusal are in place; the cipher and the derivation of the key from a password or a key file are pending. An authenticated cipher (AES-GCM or ChaCha20-Poly1305) with a fresh nonce for each file and the branch name and file name as associated data is the intended design; the nonce is stored in the record, never derived from the clock.
* **Revision history of a branch** (light snapshots or differences) and **thumbnails by branch**: not designed beyond leaving `x-*` folders and unknown fields free for them.
* **Programs that use the format.** No program writes a `.eh3d` yet; each does it in its own round, with its own conversions.
