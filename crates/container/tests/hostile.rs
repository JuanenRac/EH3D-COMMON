//! Untrusted archives and writes that fail half way: no panic, nothing accepted that is not what was written, and nothing left
//! behind. Every package here is made up and the damage comes from a fixed rule.

use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use eh3d_container::{AtomicOptions, Container, ContainerSpec, ContainerWriter, PkgCode};

const PROJECT: &[u8] = b"{\"format_version\":\"1.0.0\"}";

fn temp_dir(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "eh3d_hostile_{tag}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn files(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn spec() -> ContainerSpec {
    ContainerSpec::phoenixproj(1, 0)
}

fn package() -> Vec<u8> {
    let dir = temp_dir("package");
    let path = dir.join("p.bin");
    let mut writer = ContainerWriter::new(&spec(), &dir);
    writer.add("project.json", PROJECT).unwrap();
    writer.add("design/a.txt", &b"abcdefgh".repeat(40)).unwrap();
    writer.add("results/b.bin", &[7u8; 300]).unwrap();
    writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

/// The content hash when the bytes are accepted whole.
fn accepted(bytes: &[u8]) -> Option<String> {
    let mut container = Container::open(Cursor::new(bytes.to_vec()), &spec()).ok()?;
    container.check_layout().ok()?;
    container.content_hash().ok()
}

struct Random(u64);

impl Random {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

#[test]
fn a_package_cut_anywhere_is_refused() {
    let whole = package();
    assert!(accepted(&whole).is_some());
    for cut in 0..whole.len() {
        assert_eq!(accepted(&whole[..cut]), None, "cut at {cut}");
    }
}

#[test]
fn a_package_with_changed_bytes_is_refused_or_holds_the_same_content() {
    let whole = package();
    let hash = accepted(&whole).unwrap();
    let mut random = Random(0x5EED_0001);
    for round in 0..4_000 {
        let mut bytes = whole.clone();
        for _ in 0..1 + random.below(4) {
            let at = random.below(bytes.len());
            bytes[at] ^= 1 << random.below(8);
        }
        if let Some(read) = accepted(&bytes) {
            assert_eq!(read, hash, "round {round}");
        }
    }
}

#[test]
fn a_zip64_end_record_far_outside_the_file_is_refused() {
    // a locator that says the record is at the last possible byte
    let mut bytes = vec![0u8; 64];
    bytes.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&u64::MAX.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&u16::MAX.to_le_bytes());
    bytes.extend_from_slice(&u16::MAX.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    let mut spec = spec();
    spec.allow_zip64 = true;
    let error = Container::open(Cursor::new(bytes), &spec).err().unwrap();
    assert_eq!(error.code(), Some(PkgCode::DamagedArchive));
}

#[test]
fn the_writer_counts_the_integrity_record_as_the_reader_does() {
    let dir = temp_dir("total");
    let mut spec = spec();
    spec.limits.max_total_bytes = PROJECT.len() as u64 + 100;
    let path = dir.join("p.bin");
    let mut writer = ContainerWriter::new(&spec, &dir);
    writer.add("project.json", PROJECT).unwrap();
    writer.add("design/a.txt", &[1u8; 90]).unwrap();
    // the entries fit, but not with the record of their hashes: the reader would refuse the file
    let error = writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap_err();
    assert_eq!(error.code(), Some(PkgCode::Limit));
    assert!(files(&dir).is_empty(), "{:?}", files(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

struct CutAfter(usize);

impl Read for CutAfter {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.0 == 0 {
            return Err(io::Error::other("the source went away"));
        }
        let n = out.len().min(self.0);
        out[..n].fill(5);
        self.0 -= n;
        Ok(n)
    }
}

#[test]
fn a_source_that_fails_half_way_leaves_nothing_behind() {
    let dir = temp_dir("source");
    let mut writer = ContainerWriter::new(&spec(), &dir);
    writer.add("project.json", PROJECT).unwrap();
    for at in [0, 1, 65_536, 200_000] {
        assert!(writer.add_stream("design/a.bin", CutAfter(at)).is_err());
    }
    // the name is free again and the package written has what was added, no more
    writer.add("design/a.bin", b"whole").unwrap();
    let path = dir.join("p.bin");
    writer
        .write_to_path(&path, &AtomicOptions::default())
        .unwrap();
    assert_eq!(files(&dir), ["p.bin"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_replaced_file_keeps_who_may_read_it() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("mode");
    let path = dir.join("p.bin");
    std::fs::write(&path, b"old").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    eh3d_container::write_atomic(&path, &AtomicOptions::default(), |file| {
        std::io::Write::write_all(file, b"new").map_err(Into::into)
    })
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let _ = std::fs::remove_dir_all(&dir);
}
