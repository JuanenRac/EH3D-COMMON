# The `.eh3dproj` project format

A project of the EH3D Fabrication Suite is one file with the extension `.eh3dproj`. It is a standard ZIP archive (deflate compression only), so any archive tool can look inside it. The format version described here is **1.0.0**.

## Entries

| Entry | What it holds |
|---|---|
| `project.json` | The metadata of the project (below). |
| `integrity.json` | The SHA-256 and the size of every other entry. |
| `layers/…` | The original layer files: Gerber (RS-274X, X2), Excellon drill files, outlines. |
| `toolpaths/…` | Precalculated toolpaths: G-code (GRBL, Marlin, PlanetCNC) or HP-GL. |
| `vision/…` | Fiducial points and inspection templates. |

Data entries live only under `layers/`, `toolpaths/` and `vision/`. Any other name is refused.

## `project.json`

```json
{
  "format_version": "1.0.0",
  "id": "5f3c1c5e-5b0a-4f0e-9a43-0d6d3f6b8c11",
  "name": "Test board",
  "created": "YYYY-MM-DDThh:mm:ssZ",
  "modified": "YYYY-MM-DDThh:mm:ssZ",
  "units": "millimetres",
  "zero_piece": { "x": 0.0, "y": 0.0, "z": 0.0 },
  "layers": [{ "path": "layers/top.gbr", "kind": "copper", "name": "Top copper" }],
  "tools": [{ "id": "v30", "name": "V-bit 30 degrees", "diameter_mm": 0.1 }],
  "toolpaths": [{ "path": "toolpaths/iso.nc", "dialect": "grbl", "tool_id": "v30" }],
  "fiducials": [{ "name": "F1", "x": 2.0, "y": 2.0 }],
  "notes": null
}
```

* `format_version` is a semantic version. A reader accepts every file of the same major version, ignoring what it does not know. A **newer major version is refused**; an **older major version** is migrated by the steps the crate registers, and refused when it has none. Version 1.0.0 is the first format, so there is nothing to migrate from yet.
* `id` identifies the project and survives renames and copies of the file.
* `layers[].kind`: `copper`, `solder_mask`, `silkscreen`, `paste`, `outline`, `drill`, `other`.
* `toolpaths[].dialect`: `grbl`, `marlin`, `planet_cnc`, `hpgl`. `tool_id` must name a tool of `tools`.
* Every number must be finite, tool diameters must be positive, tool ids must be unique, and layer and toolpath paths must be under their folders.

## `integrity.json`

```json
{
  "algorithm": "sha256",
  "files": {
    "project.json": { "sha256": "…", "size": 412 },
    "layers/top.gbr": { "sha256": "…", "size": 15 }
  }
}
```

A reader recomputes the SHA-256 and the size of every entry. The package is refused when:

* an entry is missing, its size or its SHA-256 differs from the record, or it is not listed (an entry cannot be added without recording it);
* `integrity.json` or `project.json` is missing;
* the algorithm is not `sha256`.

This detects a package damaged or altered by accident. It is **not a signature**: whoever can change the entries can also rewrite `integrity.json`, so it does not prove who wrote a project.

## Limits for readers

A package from another person is untrusted input. A reader refuses:

* names that are empty, absolute, longer than 240 bytes, contain `\`, `:`, control characters, an empty part, `.` or `..`;
* duplicate names;
* more than 10,000 entries, an entry larger than 256 MiB once decompressed, or more than 1 GiB in total.

## Writing

The writer does not use the clock: the same content gives the same bytes, so two copies of a project can be compared by hash. `write_to_path` writes a temporary file next to the target and renames it, so a crash never leaves half a project.

## Shared pieces

The integrity record, its verification, the entry-name rule above and a content hash live in the crate `eh3d-container`, so that another container format can use them. The crate also has a description of the parameters of a format (`ContainerSpec`) with a preset for this one, and canonical JSON (RFC 8785) for formats that need one text form per value. `.eh3dproj` 1.0.0 does not use canonical JSON: its two JSON files keep the form recorded by the reference packages. The content hash is the SHA-256 of the canonical JSON array of `[name, size, sha256]` of every entry except those under `audit/`, in the order of the bytes of the names. The `.eh3dproj` 1.0.0 reader and writer of this crate behave as before. The crate `eh3d-container` also has a generic streaming reader and writer for new formats; see the next section.

## The container of new formats

New formats (`.phoenixproj`, `.eh3dproj` 1.1.0) are read and written by `eh3d-container` under a stricter policy, chosen by the preset (`ContainerSpec::phoenixproj`, `ContainerSpec::eh3dproj_v1_1`). The `.eh3dproj` 1.0.0 preset keeps the rules above; a test shows that the generic writer with that preset reproduces the reference packages byte for byte.

* **Reading.** `Container::open` reads only the central directory and refuses the package before decompressing anything or reserving memory for what it announces; entries are read through a stream that stops at the limits and at the time budget whatever the archive claims. Every refusal has a stable code, `EH3D-PKG-001` to `EH3D-PKG-014`, listed in the registry of the documentation repository.
* **Strict names and ZIP.** A name must be valid UTF-8 in Unicode normal form C (non-ASCII names need the UTF-8 flag), no part may end with a dot or a space, and no part may be a Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`, `LPT1` to `LPT9`, with or without an extension, in any letter case); two names that differ only in letter case or Unicode form are duplicates; links, devices, directory entries, ZIP64 records (unless the preset allows them), encryption, methods outside the preset, flags other than the UTF-8 flag (and, when reading, the data descriptor), data after the end of the archive and entries outside the allowed roots are refused. The format version is checked before the roots, so a newer package says that its version is newer.
* **Writing.** Entries may be added in any order, from a stream: each one is compressed once into a temporary file next to the target while its SHA-256 is computed, and the archive is assembled from those in the order of the preset (data entries sorted by the bytes of their names, then `project.json`, then `integrity.json`, whose JSON is canonical). Semantic reproducibility always holds; byte-for-byte reproducibility holds for one named writer profile.
* **Atomic replacement.** The package is written to a temporary file in the target folder, flushed and synced, and renamed over the target. On Windows a refused rename (an antivirus scanner or a sync client holding the file) is retried with a short back-off for a configurable time (2 s by default); if it still fails the target is untouched and the error is `EH3D-PKG-014`. A copy of the previous file can be kept for migrations, and `clean_orphan_temporaries` removes only old temporary files of this module.
* **Settings.** The limits, the time and memory budget and the retry time are plain data (`Limits`, `Budget`, `AtomicOptions`) meant to be read from a configuration file; the presets hold starting values.
* **Versions.** `.eh3dproj` exports stay at 1.0.0 by default; 1.1.0 is written only when asked for. The preset of 1.1.0 adds the `assembly/` and `heightmaps/` folders.

## Reference packages

`tests/golden/` holds three valid packages (an empty project, a typical one, and one with names beyond ASCII), nineteen damaged ones in `tests/golden/rejected/` and a `SHA256SUMS` for all of them. The tests check that the writer reproduces the valid packages byte for byte, that the reader reads them back, and that the reader gives the recorded verdict on each damaged one (for example, a name with `..` is refused as unsafe and a changed entry fails its integrity check).

The tests also record behaviour that this page does not promise: the order of the entries in the archive, the exact form of the two JSON files, the ZIP flags, deflate compression, the fixed time stamp of 1980-01-01, the absence of ZIP64, and the fact that data after the end of the archive and two names that differ only in letter case are accepted today. A change that alters any of these has to change the reference files on purpose. To rebuild them after such a change run `cargo test --test characterization regenerate_golden -- --ignored`.
