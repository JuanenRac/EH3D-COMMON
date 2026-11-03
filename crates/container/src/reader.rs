//! The generic container reader: bounded, streaming and strict.
//!
//! [`Container::open`] reads the central directory only (no entry is decompressed), checks every
//! rule of the spec that can be checked from it and refuses the package before reserving memory
//! for what it announces. Entries are read afterwards through [`Container::read_stream`] or
//! [`Container::read_limited`]; both stop at the limits and the budget of the spec while
//! decompressing, whatever the archive claims about its own sizes.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::codes::PkgCode;
use crate::error::{stream_error, ContainerError, Violation};
use crate::integrity::{Integrity, ALGORITHM};
use crate::policy::{check_name, collision_key, Policy};
use crate::spec::{ContainerSpec, Method, INTEGRITY_ENTRY, PROJECT_ENTRY};

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_LEN: usize = 22;
const FLAG_ENCRYPTED: u16 = 0x0001;
const FLAG_DESCRIPTOR: u16 = 0x0008;
const FLAG_STRONG_ENCRYPTION: u16 = 0x0040;
const FLAG_UTF8: u16 = 0x0800;
/// The largest `integrity.json` the reader will hold in memory, besides the memory budget.
const MAX_INTEGRITY_BYTES: u64 = 64 * 1024 * 1024;
/// The largest central directory the reader will hold in memory.
const MAX_DIRECTORY_BYTES: u64 = 64 * 1024 * 1024;

/// What the central directory says about one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryInfo {
    /// The name.
    pub name: String,
    /// The size once decompressed, as the archive announces it.
    pub size: u64,
    /// The size in the archive.
    pub compressed_size: u64,
    /// The CRC-32 the archive announces.
    pub crc32: u32,
    /// The compression method.
    pub method: Method,
}

/// An open container: the validated list of its entries and the stream to read them from.
pub struct Container<R: Read + Seek> {
    spec: ContainerSpec,
    entries: Vec<EntryInfo>,
    index: HashMap<String, usize>,
    archive: ZipArchive<R>,
    spent: Duration,
}

fn damaged(detail: &str) -> ContainerError {
    ContainerError::Rejected(Violation::new(PkgCode::DamagedArchive, detail))
}

fn le16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}
fn le64(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

/// Where the central directory is and how many entries it has.
struct Directory {
    offset: u64,
    size: u64,
    count: u64,
}

fn find_directory<R: Read + Seek>(
    reader: &mut R,
    len: u64,
    spec: &ContainerSpec,
) -> Result<Directory, ContainerError> {
    if len < EOCD_LEN as u64 {
        return Err(damaged("the file is too small to be an archive"));
    }
    let tail_len = len.min(EOCD_LEN as u64 + u64::from(u16::MAX));
    let mut tail = vec![0u8; tail_len as usize];
    reader.seek(SeekFrom::Start(len - tail_len))?;
    reader.read_exact(&mut tail)?;
    // The last signature that is consistent with its own comment length ends the archive.
    let mut trailing = false;
    let mut found = None;
    let mut at = tail.len() - EOCD_LEN;
    loop {
        if le32(&tail, at) == EOCD_SIGNATURE {
            let end = at + EOCD_LEN + usize::from(le16(&tail, at + 20));
            if end == tail.len() {
                found = Some(at);
                break;
            }
            if end < tail.len() {
                trailing = true;
            }
        }
        if at == 0 {
            break;
        }
        at -= 1;
    }
    let at = match found {
        Some(at) => at,
        None if trailing && spec.policy == Policy::Strict => {
            return Err(damaged("there is data after the end of the archive"))
        }
        None if trailing => {
            // The old reader tolerates data after the archive; this one cannot find where the
            // archive ends without it, so the legacy policy still refuses what it cannot place.
            return Err(damaged("the end of the archive cannot be placed"));
        }
        None => return Err(damaged("the archive is truncated: no end record")),
    };
    let eocd = &tail[at..at + EOCD_LEN];
    if le16(eocd, 4) != 0 || le16(eocd, 6) != 0 {
        return Err(damaged(
            "archives split over several disks are not supported",
        ));
    }
    let eocd_pos = len - tail_len + at as u64;
    let mut count = u64::from(le16(eocd, 10));
    let mut size = u64::from(le32(eocd, 12));
    let mut offset = u64::from(le32(eocd, 16));
    let announces_zip64 = le16(eocd, 10) == u16::MAX
        || le32(eocd, 12) == u32::MAX
        || le32(eocd, 16) == u32::MAX
        || (at >= 20 && le32(&tail, at - 20) == ZIP64_LOCATOR_SIGNATURE);
    let mut directory_end = eocd_pos;
    if announces_zip64 {
        if !spec.allow_zip64 {
            return Err(ContainerError::Rejected(Violation::new(
                PkgCode::Zip64,
                "the archive has ZIP64 records",
            )));
        }
        if at < 20 || le32(&tail, at - 20) != ZIP64_LOCATOR_SIGNATURE {
            return Err(damaged("the ZIP64 locator is missing"));
        }
        let record_pos = le64(&tail, at - 20 + 8);
        let mut record = [0u8; 56];
        if record_pos + 56 > len {
            return Err(damaged("the ZIP64 end record is outside the file"));
        }
        reader.seek(SeekFrom::Start(record_pos))?;
        reader.read_exact(&mut record)?;
        if le32(&record, 0) != ZIP64_EOCD_SIGNATURE {
            return Err(damaged("the ZIP64 end record has a wrong signature"));
        }
        count = le64(&record, 32);
        size = le64(&record, 40);
        offset = le64(&record, 48);
        directory_end = record_pos;
    }
    if offset
        .checked_add(size)
        .is_none_or(|end| end > directory_end)
    {
        return Err(damaged("the central directory is outside the file"));
    }
    Ok(Directory {
        offset,
        size,
        count,
    })
}

/// The ZIP64 extended information of a central record: the fields that were 0xFFFFFFFF.
fn zip64_extra(extra: &[u8], size: &mut u64, compressed: &mut u64, offset: &mut u64) -> bool {
    let mut at = 0;
    while at + 4 <= extra.len() {
        let id = le16(extra, at);
        let len = usize::from(le16(extra, at + 2));
        let body = &extra[(at + 4).min(extra.len())..(at + 4 + len).min(extra.len())];
        if id == 1 {
            let mut p = 0;
            for field in [&mut *size, &mut *compressed, &mut *offset] {
                if *field == u64::from(u32::MAX) {
                    if p + 8 > body.len() {
                        return false;
                    }
                    *field = le64(body, p);
                    p += 8;
                }
            }
            return true;
        }
        at += 4 + len;
    }
    false
}

/// True when the extra field carries a ZIP64 block.
fn extra_has_zip64(extra: &[u8]) -> bool {
    let mut at = 0;
    while at + 4 <= extra.len() {
        if le16(extra, at) == 1 {
            return true;
        }
        at += 4 + usize::from(le16(extra, at + 2));
    }
    false
}

impl Container<File> {
    /// Opens the container stored at `path`.
    pub fn open_path(path: &Path, spec: &ContainerSpec) -> Result<Self, ContainerError> {
        Container::open(File::open(path)?, spec)
    }
}

impl<R: Read + Seek> Container<R> {
    /// Reads and validates the central directory. Nothing is decompressed. Checked here: the
    /// structure of the archive (007), ZIP64 (005), encryption and methods (006), flags (008),
    /// links and special entries (003), directory entries (004), names (001), duplicates (002),
    /// and the limits on counts and announced sizes (011). The roots, the integrity record and the
    /// format version are checked by [`Container::check_layout`],
    /// [`Container::verify_integrity`] and [`Container::open_checked`].
    pub fn open(mut reader: R, spec: &ContainerSpec) -> Result<Self, ContainerError> {
        let len = reader.seek(SeekFrom::End(0))?;
        let directory = find_directory(&mut reader, len, spec)?;
        let limits = &spec.limits;
        if directory.count > limits.max_entries as u64 {
            return Err(Violation::new(
                PkgCode::Limit,
                format!("more than {} entries", limits.max_entries),
            )
            .into());
        }
        // A record is at least 46 bytes; this also keeps the directory small enough to hold.
        let most_per_record = 46 + 3 * u64::from(u16::MAX);
        if directory.size < directory.count * 46
            || directory.size > directory.count.saturating_mul(most_per_record)
            || directory.size > MAX_DIRECTORY_BYTES
        {
            return Err(damaged(
                "the size of the central directory does not fit its entries",
            ));
        }
        let mut bytes = vec![0u8; directory.size as usize];
        reader.seek(SeekFrom::Start(directory.offset))?;
        reader.read_exact(&mut bytes)?;

        let mut entries = Vec::with_capacity(directory.count as usize);
        let mut seen: HashMap<String, String> = HashMap::new();
        let mut total: u64 = 0;
        let mut at = 0usize;
        for _ in 0..directory.count {
            if at + 46 > bytes.len() || le32(&bytes, at) != CENTRAL_SIGNATURE {
                return Err(damaged("a record of the central directory is damaged"));
            }
            let made_by = le16(&bytes, at + 4);
            let flags = le16(&bytes, at + 8);
            let method = le16(&bytes, at + 10);
            let crc32 = le32(&bytes, at + 16);
            let mut compressed = u64::from(le32(&bytes, at + 20));
            let mut size = u64::from(le32(&bytes, at + 24));
            let name_len = usize::from(le16(&bytes, at + 28));
            let extra_len = usize::from(le16(&bytes, at + 30));
            let comment_len = usize::from(le16(&bytes, at + 32));
            let external = le32(&bytes, at + 38);
            let mut local = u64::from(le32(&bytes, at + 42));
            let next = at + 46 + name_len + extra_len + comment_len;
            if next > bytes.len() {
                return Err(damaged("a record of the central directory is cut short"));
            }
            let name_bytes = &bytes[at + 46..at + 46 + name_len];
            let extra = &bytes[at + 46 + name_len..at + 46 + name_len + extra_len];
            at = next;

            // Names first: every later message is tied to one.
            let name = match std::str::from_utf8(name_bytes) {
                Ok(n) => n.to_owned(),
                Err(_) => {
                    return Err(Violation::new(
                        PkgCode::InvalidName,
                        "an entry name is not valid UTF-8",
                    )
                    .into())
                }
            };
            if spec.policy == Policy::Strict && !name.is_ascii() && flags & FLAG_UTF8 == 0 {
                return Err(Violation::new(
                    PkgCode::InvalidName,
                    "a name with non-ASCII characters lacks the UTF-8 flag, so its encoding would be a guess",
                )
                .at(name)
                .into());
            }

            let sizes_are_64 = size == u64::from(u32::MAX)
                || compressed == u64::from(u32::MAX)
                || local == u64::from(u32::MAX);
            if sizes_are_64 || extra_has_zip64(extra) {
                if !spec.allow_zip64 {
                    return Err(
                        Violation::new(PkgCode::Zip64, "an entry uses ZIP64 records")
                            .at(name)
                            .into(),
                    );
                }
                if !zip64_extra(extra, &mut size, &mut compressed, &mut local) {
                    return Err(damaged("a ZIP64 field is missing"));
                }
            }

            if flags & (FLAG_ENCRYPTED | FLAG_STRONG_ENCRYPTION) != 0 {
                return Err(Violation::new(PkgCode::Method, "an entry is encrypted")
                    .at(name)
                    .into());
            }
            let method = match method {
                0 => Method::Stored,
                8 => Method::Deflate,
                other => {
                    return Err(Violation::new(
                        PkgCode::Method,
                        format!("compression method {other} is not supported"),
                    )
                    .at(name)
                    .into())
                }
            };
            if !spec.methods.contains(&method) {
                return Err(Violation::new(
                    PkgCode::Method,
                    format!(
                        "compression method {method:?} is not allowed by {}",
                        spec.format_id
                    ),
                )
                .at(name)
                .into());
            }
            if spec.policy == Policy::Strict && flags & !(FLAG_UTF8 | FLAG_DESCRIPTOR) != 0 {
                return Err(Violation::new(
                    PkgCode::Flags,
                    format!("general-purpose flags {flags:#06x} are not allowed"),
                )
                .at(name)
                .into());
            }

            let host = (made_by >> 8) as u8;
            let unix_kind = if host == 3 {
                (external >> 16) & 0o170_000
            } else {
                0
            };
            if unix_kind != 0 && unix_kind != 0o100_000 && unix_kind != 0o040_000 {
                return Err(Violation::new(
                    PkgCode::SpecialEntry,
                    "a link, device or special entry",
                )
                .at(name)
                .into());
            }
            if name.ends_with('/') || unix_kind == 0o040_000 || external & 0x10 != 0 {
                return Err(
                    Violation::new(PkgCode::DirectoryEntry, "an explicit directory entry")
                        .at(name)
                        .into(),
                );
            }

            check_name(spec.policy, &name, limits.max_name_bytes).map_err(ContainerError::from)?;
            let key = if spec.policy == Policy::Strict {
                collision_key(&name)
            } else {
                name.clone()
            };
            if let Some(first) = seen.insert(key, name.clone()) {
                return Err(Violation::new(
                    PkgCode::DuplicateName,
                    format!("same as '{first}' once case and Unicode form are ignored"),
                )
                .at(name)
                .into());
            }

            let allowed = limits.max_entry_bytes_for(&name);
            if size > allowed {
                return Err(Violation::new(
                    PkgCode::Limit,
                    format!("the entry announces {size} bytes, the limit is {allowed}"),
                )
                .at(name)
                .into());
            }
            total = total.saturating_add(size);
            if total > limits.max_total_bytes {
                return Err(Violation::new(
                    PkgCode::Limit,
                    format!(
                        "the entries announce more than {} bytes",
                        limits.max_total_bytes
                    ),
                )
                .into());
            }
            if local.saturating_add(30) > directory.offset {
                return Err(damaged("an entry starts after the central directory"));
            }
            entries.push(EntryInfo {
                name,
                size,
                compressed_size: compressed,
                crc32,
                method,
            });
        }
        if at != bytes.len() {
            return Err(damaged(
                "the central directory has bytes beyond its records",
            ));
        }
        let index = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.name.clone(), i))
            .collect();
        reader.seek(SeekFrom::Start(0))?;
        let archive = ZipArchive::new(reader)
            .map_err(|e| damaged(&format!("the archive cannot be read: {e}")))?;
        if archive.len() != entries.len() {
            return Err(damaged("the entry count differs between the two readings"));
        }
        Ok(Container {
            spec: spec.clone(),
            entries,
            index,
            archive,
            spent: Duration::ZERO,
        })
    }

    /// The spec this container was opened with.
    pub fn spec(&self) -> &ContainerSpec {
        &self.spec
    }

    /// The entries, in archive order. Metadata only.
    pub fn entries(&self) -> impl Iterator<Item = &EntryInfo> {
        self.entries.iter()
    }

    /// The entry called `name`, if there is one.
    pub fn entry(&self, name: &str) -> Option<&EntryInfo> {
        self.index.get(name).map(|&i| &self.entries[i])
    }

    /// The time spent decompressing so far.
    pub fn time_spent(&self) -> Duration {
        self.spent
    }

    /// Checks that every required entry exists (009) and that every other entry is under an
    /// allowed root or an extension folder (009). `integrity.json` missing is 010.
    pub fn check_layout(&self) -> Result<(), ContainerError> {
        for required in &self.spec.required_entries {
            if !self.index.contains_key(required) {
                let code = if required == INTEGRITY_ENTRY {
                    PkgCode::Integrity
                } else {
                    PkgCode::OutsideRoots
                };
                return Err(Violation::new(code, "a required entry is missing")
                    .at(required.clone())
                    .into());
            }
        }
        for e in &self.entries {
            let known = self.spec.required_entries.contains(&e.name)
                || self.spec.is_data_entry(&e.name)
                || self.spec.is_extension_entry(&e.name);
            if !known {
                return Err(Violation::new(
                    PkgCode::OutsideRoots,
                    "the entry is not under a root this format allows",
                )
                .at(e.name.clone())
                .into());
            }
        }
        Ok(())
    }

    /// A reader for one entry that stops at the limits of the spec and at the budget. A wrong
    /// CRC, a size different from the announced one or a damaged stream end as an io error that
    /// carries a [`Violation`] (see [`crate::violation_in`]).
    pub fn read_stream(&mut self, name: &str) -> Result<EntryReader<'_, R>, ContainerError> {
        let &i = self.index.get(name).ok_or_else(|| {
            ContainerError::Rejected(
                Violation::new(PkgCode::OutsideRoots, "there is no such entry").at(name),
            )
        })?;
        let info = self.entries[i].clone();
        let limit = self.spec.limits.max_entry_bytes_for(name);
        let budget = self.spec.budget;
        let file = self
            .archive
            .by_index(i)
            .map_err(|e| damaged(&format!("the entry cannot be opened: {e}")))?;
        Ok(EntryReader {
            inner: file,
            info,
            limit,
            budget,
            spent: &mut self.spent,
            read: 0,
        })
    }

    /// Reads a whole entry into memory, at most `max_bytes`. The announced size is checked
    /// first, so a package cannot make this reserve more than the call allows.
    pub fn read_limited(&mut self, name: &str, max_bytes: u64) -> Result<Vec<u8>, ContainerError> {
        let ceiling = max_bytes.min(self.spec.budget.max_memory_bytes);
        if let Some(info) = self.entry(name) {
            if info.size > ceiling {
                let code = if info.size > max_bytes {
                    PkgCode::Limit
                } else {
                    PkgCode::Budget
                };
                return Err(Violation::new(
                    code,
                    format!(
                        "the entry announces {} bytes, this read allows {ceiling}",
                        info.size
                    ),
                )
                .at(name)
                .into());
            }
        }
        let mut stream = self.read_stream(name)?;
        let mut out = Vec::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let n = stream.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            if out.len() as u64 + n as u64 > ceiling {
                return Err(
                    Violation::new(PkgCode::Limit, "the entry grew past the allowed size")
                        .at(name)
                        .into(),
                );
            }
            out.extend_from_slice(&buffer[..n]);
        }
        Ok(out)
    }

    /// Checks `integrity.json` against every entry, hashing each one while it streams (the
    /// memory used does not depend on the size of the entries). Returns the record on success.
    /// The order of the checks is the one of [`Integrity::verify`].
    pub fn verify_integrity(&mut self) -> Result<Integrity, ContainerError> {
        let raw = self.read_limited(INTEGRITY_ENTRY, MAX_INTEGRITY_BYTES)?;
        let wrong = |detail: &str, entry: &str| {
            ContainerError::Rejected(Violation::new(PkgCode::Integrity, detail).at(entry))
        };
        let record: Integrity = serde_json::from_slice(&raw)
            .map_err(|_| wrong("integrity.json is not a valid record", INTEGRITY_ENTRY))?;
        if record.algorithm != ALGORITHM {
            return Err(wrong(
                "the record names an unknown algorithm",
                INTEGRITY_ENTRY,
            ));
        }
        for (name, expected) in &record.files {
            if name == INTEGRITY_ENTRY || self.entry(name).is_none() {
                return Err(wrong(
                    "the record lists an entry the container does not have",
                    name,
                ));
            }
            let info = self.entry(name).cloned().expect("checked above");
            if info.size != expected.size {
                return Err(wrong("the size differs from the record", name));
            }
            let mut stream = self.read_stream(name)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut count: u64 = 0;
            loop {
                let n = stream.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                count += n as u64;
                hasher.update(&buffer[..n]);
            }
            if count != expected.size {
                return Err(wrong("the size differs from the record", name));
            }
            let digest: String = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if digest != expected.sha256 {
                return Err(wrong("the SHA-256 differs from the record", name));
            }
        }
        if let Some(extra) = self
            .entries
            .iter()
            .find(|e| e.name != INTEGRITY_ENTRY && !record.files.contains_key(&e.name))
        {
            return Err(wrong(
                "the entry is not listed in the record",
                &extra.name.clone(),
            ));
        }
        Ok(record)
    }

    /// The content hash of the container (see [`Integrity::content_hash`]) after the integrity
    /// record has been verified.
    pub fn content_hash(&mut self) -> Result<String, ContainerError> {
        let record = self.verify_integrity()?;
        record
            .content_hash()
            .map_err(|e| ContainerError::Io(io::Error::other(e.to_string())))
    }

    /// Opens, then checks the format version before the roots (013), then the layout, then the
    /// integrity record. `version_of` receives the bytes of `project.json` and returns the
    /// `(major, minor)` of its format version, or `None` when it has none.
    pub fn open_checked(
        reader: R,
        spec: &ContainerSpec,
        version_of: impl FnOnce(&[u8]) -> Option<(u64, u64)>,
    ) -> Result<Self, ContainerError> {
        let mut container = Container::open(reader, spec)?;
        if container.entry(PROJECT_ENTRY).is_some() {
            let project = container.read_limited(PROJECT_ENTRY, MAX_INTEGRITY_BYTES)?;
            if let Some((major, minor)) = version_of(&project) {
                if !spec.supports_version(major, minor) {
                    return Err(Violation::new(
                        PkgCode::NewerVersion,
                        format!(
                            "version {major}.{minor} is newer than this reader supports ({}.{})",
                            spec.major_version, spec.known_minor
                        ),
                    )
                    .at(PROJECT_ENTRY)
                    .into());
                }
            }
        }
        container.check_layout()?;
        container.verify_integrity()?;
        Ok(container)
    }
}

/// A bounded reader of one entry; see [`Container::read_stream`].
pub struct EntryReader<'a, R: Read> {
    inner: zip::read::ZipFile<'a, R>,
    info: EntryInfo,
    limit: u64,
    budget: crate::spec::Budget,
    spent: &'a mut Duration,
    read: u64,
}

impl<R: Read> Read for EntryReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let started = Instant::now();
        // Never ask for more than the limit allows plus one byte, so an overrun is seen but
        // never produced.
        let allowed = self.limit.saturating_sub(self.read).saturating_add(1);
        let want = buf
            .len()
            .min(usize::try_from(allowed).unwrap_or(usize::MAX));
        let result = self.inner.read(&mut buf[..want]);
        *self.spent += started.elapsed();
        let n = match result {
            Ok(n) => n,
            Err(e) => {
                // The zip reader reports a wrong CRC and a damaged deflate stream this way.
                return Err(
                    if e.kind() == io::ErrorKind::InvalidData
                        || e.kind() == io::ErrorKind::UnexpectedEof
                    {
                        stream_error(
                            Violation::new(
                                PkgCode::DamagedArchive,
                                format!("the entry is damaged: {e}"),
                            )
                            .at(self.info.name.clone()),
                        )
                    } else {
                        e
                    },
                );
            }
        };
        self.read += n as u64;
        if self.read > self.limit {
            return Err(stream_error(
                Violation::new(
                    PkgCode::Limit,
                    format!("the entry is larger than its limit of {} bytes", self.limit),
                )
                .at(self.info.name.clone()),
            ));
        }
        if self.budget.max_millis != 0
            && *self.spent > Duration::from_millis(self.budget.max_millis)
        {
            return Err(stream_error(Violation::new(
                PkgCode::Budget,
                format!(
                    "decompression took longer than {} ms",
                    self.budget.max_millis
                ),
            )));
        }
        if n == 0 && self.read != self.info.size {
            return Err(stream_error(
                Violation::new(
                    PkgCode::DamagedArchive,
                    "the entry has a different size than the archive announces",
                )
                .at(self.info.name.clone()),
            ));
        }
        Ok(n)
    }
}
