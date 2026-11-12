# Using the crates of this repository from another repository

The crates of `crates/` are not published on a registry. A repository of the suite that needs one takes it from a **sibling folder**, as a path dependency, and the CI of that repository checks the sibling out next to it.

## The layout of the folders

All the repositories of the suite are cloned side by side in one folder:

```text
GitHub/
├── EH3D-COMMON/            this repository (the root crate and crates/*)
├── EH3D-CORE-LIBS/         the readers of Gerber, Excellon and G-code
└── EH3D-SERVER/            a repository that uses crates of EH3D-COMMON
```

A path always goes up one folder and into the sibling: `../EH3D-COMMON/crates/controller`.

## The line in `Cargo.toml`

Give the path **and** a version, so that a check can tell which version a repository needs:

```toml
eh3d-controller = { version = ">=0.0.1", path = "../EH3D-COMMON/crates/controller" }
eh3d-levelling  = { version = ">=0.0.1", path = "../EH3D-COMMON/crates/levelling" }
```

The root crate (`eh3d-common`, the project container) is the folder itself: `path = "../EH3D-COMMON"`.

Then list the sibling in `depends_on` of the manifest `eh3d.project.json` of the repository that uses it (`"depends_on": ["EH3D-COMMON"]`). The CI reads that list to check the siblings out, and the Updater installs them first.

## Features

| Crate | Feature | What it does |
|---|---|---|
| `eh3d-controller` | `serial` | opens a real serial port (`serialport`); without it only the tests and the simulator work |
| `eh3d-diagnostic` | `serde` (on by default) | the JSON form of the diagnostics |

A feature of a sibling is passed on with the name of the crate: `serial = ["eh3d-controller/serial"]`.

## The CI of the repository that uses it

The workflow `rust.yml` of a consuming repository has a variable that names the **tag** of each sibling it is built against, so that a change of `main` here does not break it by surprise:

```yaml
env:
  SIBLING_REFS: "EH3D-COMMON=v0.2.0 EH3D-CORE-LIBS=v0.0.8"
```

When a new version of this repository is published, move the tag in the repositories that use something new from it. A sibling that is not listed is taken at `main`.

A repository that uses something that is not yet in a tag of the sibling (an API of `main`) keeps the sibling at `main` until the next version is published; then it takes the tag.

## The oldest version it works with: `min_common`

The manifest of a repository that uses this one may say, with the optional field `min_common`, the oldest version of this repository it works with (`"min_common": "0.2.0"`). It is the tag that the CI of the repository builds against. The updater reads it and warns when the installed version of this repository is older; it never stops an update because of it. A repository whose CI does not pin a tag does not say it. The field only goes up: a repository that now needs a newer version raises it in the same change.

## The limits of the readers and the spindles

The readers of the drill file, the plot, the drawing and the Gerber outline refuse a file past the hard limits of `crates/formats/reader_limits.cfg` and say when a file is past a warning limit (`ncd::parse_limited`, `hpgl::parse_limited`; `eh3d_summary::summary::size_warning` gives the notice in the 37 languages). The limits of the spindles are in `crates/materials/spindles.cfg` (`eh3d_materials::spindle`). Both are files with one value per line: a program that needs other values keeps its own copy and reads it with `ReaderLimits::from_config`; nothing is written in the code.

## What not to do

* Do not copy a crate into another repository: take it from here.
* Do not point a path at a folder outside the sibling layout (an absolute path breaks the CI and every other computer).
* Do not use a feature that the sibling does not have: the build fails with the name of the missing feature.
