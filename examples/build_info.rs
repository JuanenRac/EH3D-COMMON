//! The build text of a program: make it from the variables of Cargo, write it, read it back and see how a bad one is refused.
//!
//! Run it with `cargo run --example build_info`. Set `EH3D_BUILD_COMMIT` and `EH3D_BUILD_KIND` in the environment of the
//! build (a `build.rs` does it with `println!("cargo:rustc-env=EH3D_BUILD_COMMIT=...")`) to see them in the text.

use eh3d_common::build::BuildInfo;

fn main() {
    // what a program embeds: made when it is compiled, from the crate's own name and version
    let info = match eh3d_common::embedded_build!() {
        Ok(info) => info,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    println!("{info}");
    let json = info.to_json();
    println!("{json}");

    // what an About window or an updater does with a text it finds in a binary
    match BuildInfo::from_json(&json) {
        Ok(read) => println!("read back: {}", read == info),
        Err(error) => println!("refused: {error}"),
    }

    // a text that does not follow the layout is refused with a stable code
    let bad =
        r#"{"schema":1,"repo":"EH3D-DRILLPRO","version":"0.1","commit":null,"kind":"release"}"#;
    if let Err(error) = BuildInfo::from_json(bad) {
        println!("{} -> {error}", error.code());
    }
}
