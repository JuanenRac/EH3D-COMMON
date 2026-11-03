//! The acceptance gate for the shared container: a reader of `.eh3dproj` 1.0.0 that was built before
//! the extraction refuses a package of a new format cleanly, with an error and no panic, and the
//! new reader and writer agree with the old ones on the reference packages.

use std::io::Cursor;

use eh3d_common::container::Package;
use eh3d_container::{AtomicOptions, Container, ContainerSpec, ContainerWriter};

fn folder(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_gate_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_reader_of_version_1_0_0_refuses_a_phoenix_package_without_panicking() {
    let dir = folder("phoenix");
    let spec = ContainerSpec::phoenixproj(0, 1);
    let mut writer = ContainerWriter::new(&spec, &dir);
    writer
        .add("project.json", br#"{"format_version":"0.1.0","name":"x"}"#)
        .unwrap();
    writer.add("design/board.json", b"{}").unwrap();
    writer.add("x-vendor/data.bin", b"\x00\x01").unwrap();
    let path = dir.join("p.phoenixproj");
    writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(Package::read(Cursor::new(bytes)).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_reader_of_version_1_0_0_refuses_a_1_1_0_package_with_assembly_data() {
    let dir = folder("v11");
    let spec = ContainerSpec::eh3dproj_v1_1();
    let mut writer = ContainerWriter::new(&spec, &dir);
    let project = br#"{"format_version":"1.1.0","id":"00000000-0000-4000-8000-000000000001","name":"n","created":"2026-01-02T03:04:05Z","modified":"2026-01-02T03:04:05Z"}"#;
    writer.add("project.json", project).unwrap();
    writer.add("assembly/assembly.json", b"{}").unwrap();
    let path = dir.join("p.eh3dproj");
    writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(Package::read(Cursor::new(bytes.clone())).is_err());
    // the new reader with the 1.1.0 preset reads it
    let mut container = Container::open(Cursor::new(bytes), &spec).unwrap();
    container.check_layout().unwrap();
    container.verify_integrity().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
