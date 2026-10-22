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
  "created": "2026-10-22T09:00:00Z",
  "modified": "2026-10-22T09:00:00Z",
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
