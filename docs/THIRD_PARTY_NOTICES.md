# Third-party notices

A program that ships Rust crates ships their copyright notices and licence texts. `tools/third_party_notices.py` makes the file from what `cargo metadata` resolves, and checks it.

    python tools/third_party_notices.py --root <repository of the program>
    python tools/third_party_notices.py --root <repository of the program> --check

* The tree: normal dependencies (not those of the tests or of the build scripts), every feature of the members of the workspace, for the platforms of the releases (Windows 64 and 32 bits, Linux x86_64 and aarch64, macOS Apple silicon and Intel).
* The texts are copied from the source of each crate (licence, copying and notice files, and the licences of embedded fonts). A crate that ships none gets the text of its licence with its authors; a licence the tool has no text for stops it.
* A `THIRD_PARTY_NOTICES` file written by hand beside a sibling crate (`crates/materials`, `crates/printmaterials`, `crates/viewer`) is carried into the file of the program.
* `--check` compares crate names and licences, not versions: raising a version does not fail, a new crate or a changed licence does.
* `--package <member>` and `--output <file>` limit it to one member; `crates/project/THIRD_PARTY_NOTICES` is made that way.

It needs the sibling repositories beside the repository, as a build does. The licences of PDFium are not part of this file: `tools/fetch_pdfium.py` puts them beside the library.
