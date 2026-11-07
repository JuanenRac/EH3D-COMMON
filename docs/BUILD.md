# The build text (`eh3d_common::build`)

Every program of the suite can carry a small JSON text inside its binary that says which build it is. The About window, a bug report and the updater read it from the running program: no file beside the program, no network and no clock are needed, so the text is the same on every machine.

```json
{"schema":1,"repo":"EH3D-DRILLPRO","version":"0.1.6","commit":"1a2b3c4","kind":"release"}
```

| Key | Type | Rule |
|---|---|---|
| `schema` | whole number | `1`; another number is refused |
| `repo` | text | the repository the program is built from: 1 to 64 letters, digits, `.`, `_` and `-` |
| `version` | text | three or four whole numbers split by dots (`0.1.10`, `0.8.0.1`) |
| `commit` | text or `null` | 7 to 40 lower case hex digits, or `null` when the build did not know it |
| `kind` | text | `release` (published and signed), `ci` (built by the continuous integration, not published) or `dev` (built on a computer; the default) |

The reader is strict: one flat object, the five keys in any order, no key twice, no other key, no escapes in the text and at most 1024 bytes. The writer always gives the keys in the order of the table and no spaces, so the same record gives the same bytes.

## Putting it in a program

`eh3d_common::embedded_build!()` makes the record when the program is compiled, from variables of the build:

| Variable | Gives | If it is not set |
|---|---|---|
| `CARGO_PKG_VERSION` | `version` | always set by Cargo |
| `EH3D_BUILD_REPO` | `repo` | the name of the crate |
| `EH3D_BUILD_COMMIT` | `commit` | `null` |
| `EH3D_BUILD_KIND` | `kind` | `dev` |

A `build.rs` of the program sets the optional ones for the compiler, for example from `git rev-parse --short=12 HEAD` and from the workflow that builds a release:

```rust
// build.rs
fn main() {
    if let Ok(out) = std::process::Command::new("git").args(["rev-parse", "--short=12", "HEAD"]).output() {
        let commit = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !commit.is_empty() {
            println!("cargo:rustc-env=EH3D_BUILD_COMMIT={commit}");
        }
    }
    println!("cargo:rerun-if-changed=.git/HEAD");
}
```

```rust
let info = eh3d_common::embedded_build!().expect("a valid build text");
println!("{info}"); // EH3D-DRILLPRO 0.1.6 (1a2b3c4, release)
let json = info.to_json(); // what to embed or show
```

`BuildInfo::from_json` reads a text found in a binary or a file; `BuildInfo::from_parts` checks the parts one by one. `cargo run --example build_info` shows both.

## Codes

| Code | Meaning |
|---|---|
| `EH3D-BUILD-001` | the text is longer than 1024 bytes |
| `EH3D-BUILD-002` | the text is not a flat JSON object of strings, whole numbers and `null` (the byte is given) |
| `EH3D-BUILD-003` | a key appears twice |
| `EH3D-BUILD-004` | a key that is not part of the layout |
| `EH3D-BUILD-005` | a key is missing or has the wrong type |
| `EH3D-BUILD-006` | the `schema` is not `1` |
| `EH3D-BUILD-007` | the name of the repository is not valid |
| `EH3D-BUILD-008` | the version, the commit or the kind is not valid |

Nothing here has run inside a release of a program yet: the module and its ten tests run on a computer, and the programs of the suite do not embed the text until each one adds the line above.
