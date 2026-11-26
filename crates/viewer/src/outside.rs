//! Taking an attachment out of the program: a cleaned name, a copy in a private temporary folder and
//! a request to the system to open it. Nothing is done without a call of the program, a dangerous or
//! unknown file needs the agreement of a person, and a program or a script is never launched.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::ViewerError;
use crate::kind::{classify, inspect};
use crate::limits::ViewerLimits;
use crate::source::{read_head, FileSource, Source};

/// The longest name kept, in bytes.
const MAX_NAME_BYTES: usize = 150;

/// What a person said to a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    NotAsked,
    Given,
}

fn hidden_or_reordering(c: char) -> bool {
    // marks that change the order the letters are shown in, or are not shown at all
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2069}' | '\u{feff}')
}

fn reserved_on_windows(stem: &str) -> bool {
    let stem = stem.trim_end().to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|p| {
        stem.strip_prefix(p).is_some_and(|n| {
            matches!(
                n,
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    })
}

/// A file name that is safe on every system: only the last part of a path, no separators, controls,
/// invisible or reordering marks, no leading or trailing dots and spaces, no reserved device name and
/// not too long. The extension is kept.
pub fn sanitize_name(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = last
        .chars()
        .filter(|c| !c.is_control() && !hidden_or_reordering(*c))
        .map(|c| if "<>:\"|?*,;".contains(c) { '_' } else { c })
        .collect();
    let mut cleaned = cleaned
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .to_string();
    // a name that starts with a dash could be read as an option by the program that opens it
    if cleaned.starts_with('-') {
        cleaned.replace_range(..1, "_");
    }
    if cleaned.is_empty() {
        return "attachment".to_string();
    }
    if reserved_on_windows(cleaned.split('.').next().unwrap_or("")) {
        cleaned.insert(0, '_');
    }
    if cleaned.len() > MAX_NAME_BYTES {
        let (stem, extension) = match cleaned.rsplit_once('.') {
            Some((s, e)) if e.len() <= 24 => (s, format!(".{e}")),
            _ => (cleaned.as_str(), String::new()),
        };
        let mut end = (MAX_NAME_BYTES - extension.len()).min(stem.len());
        while !stem.is_char_boundary(end) {
            end -= 1;
        }
        cleaned = format!("{}{extension}", stem[..end].trim_end());
    }
    cleaned
}

fn copy_within(source: &dyn Source, to: &mut File, limit: u64) -> Result<(), ViewerError> {
    let mut reader = source.reader()?.take(limit.saturating_add(1));
    let copied = io::copy(&mut reader, to)?;
    if copied > limit {
        return Err(ViewerError::TooLarge { limit });
    }
    Ok(to.flush()?)
}

fn new_private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Marks a file as coming from outside, so Windows and the office programs treat it with care.
fn mark_as_foreign(path: &Path) {
    if cfg!(windows) {
        let mut stream = path.as_os_str().to_os_string();
        stream.push(":Zone.Identifier");
        // a file system without streams has no mark; there is nothing else to do
        let _ = fs::write(stream, "[ZoneTransfer]\r\nZoneId=3\r\n");
    }
}

/// Writes the attachment to a new file, never replacing one and never past the limit; a partial
/// file is removed.
fn write_copy(source: &dyn Source, path: &Path, limit: u64) -> Result<(), ViewerError> {
    let mut file = new_private_file(path)?;
    if let Err(e) = copy_within(source, &mut file, limit) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(e);
    }
    drop(file);
    mark_as_foreign(path);
    Ok(())
}

/// Saves a copy where the person said. Dangerous and unknown files need their agreement, and a file
/// that is already there is never replaced.
pub fn save_copy(
    source: &dyn Source,
    to: &Path,
    consent: Consent,
    limits: &ViewerLimits,
) -> Result<(), ViewerError> {
    // the name that counts is the one the file gets
    let chosen = to
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let named = classify(&chosen, &read_head(source, 4096)?);
    if (inspect(source, limits).needs_consent() || named.needs_consent())
        && consent != Consent::Given
    {
        return Err(ViewerError::ConsentRequired);
    }
    write_copy(source, to, limits.max_attachment_bytes)
}

/// Asks the system to open a file. A program gives [`SystemLauncher`]; tests give their own.
pub trait Launcher: Send {
    fn launch(&mut self, path: &Path) -> Result<(), ViewerError>;
}

/// The opener of the operating system, called by its full path, never through a shell.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemLauncher;

fn opener() -> Option<PathBuf> {
    if cfg!(windows) {
        let root = std::env::var_os("SystemRoot")?;
        Some(PathBuf::from(root).join("explorer.exe"))
    } else if cfg!(target_os = "macos") {
        Some(PathBuf::from("/usr/bin/open"))
    } else {
        Some(PathBuf::from("/usr/bin/xdg-open"))
    }
}

impl Launcher for SystemLauncher {
    fn launch(&mut self, path: &Path) -> Result<(), ViewerError> {
        let opener = opener()
            .filter(|p| p.is_absolute() && p.is_file())
            .ok_or_else(|| ViewerError::Io("the system has no known opener".into()))?;
        let mut child = Command::new(opener)
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        // wait aside so the process does not stay behind
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}

/// The temporary folder of the copies that are opened outside. It is made when first needed, is
/// private to the user and is removed with its copies when dropped, as far as the system lets go.
#[derive(Debug, Default)]
pub struct Outbox {
    base: Option<PathBuf>,
    root: Option<PathBuf>,
    next: u64,
}

static FOLDERS: AtomicU64 = AtomicU64::new(0);

fn new_private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

impl Outbox {
    pub fn new() -> Self {
        Outbox::default()
    }

    /// Puts the folder under `base` instead of the temporary folder of the system.
    pub fn under(base: impl Into<PathBuf>) -> Self {
        Outbox {
            base: Some(base.into()),
            root: None,
            next: 0,
        }
    }

    fn root(&mut self) -> Result<PathBuf, ViewerError> {
        if let Some(root) = &self.root {
            return Ok(root.clone());
        }
        let base = self.base.clone().unwrap_or_else(std::env::temp_dir);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let root = base.join(format!(
            "eh3d-open-{}-{}-{nanos:x}",
            std::process::id(),
            FOLDERS.fetch_add(1, Ordering::Relaxed)
        ));
        // fails when something of that name is already there: it is never reused
        new_private_dir(&root)?;
        self.root = Some(root.clone());
        Ok(root)
    }

    /// A copy of the attachment under its cleaned name, in a folder of its own.
    pub fn extract(
        &mut self,
        source: &dyn Source,
        consent: Consent,
        limits: &ViewerLimits,
    ) -> Result<PathBuf, ViewerError> {
        let class = inspect(source, limits);
        if class.needs_consent() && consent != Consent::Given {
            return Err(ViewerError::ConsentRequired);
        }
        self.next += 1;
        let folder = self.root()?.join(self.next.to_string());
        new_private_dir(&folder)?;
        let path = folder.join(&class.name);
        write_copy(source, &path, limits.max_attachment_bytes)?;
        Ok(path)
    }

    /// Makes the copy and asks the system to open it. A program or a script is refused before
    /// anything is written, whatever the person agreed to.
    pub fn open(
        &mut self,
        source: &dyn Source,
        consent: Consent,
        limits: &ViewerLimits,
        launcher: &mut dyn Launcher,
    ) -> Result<PathBuf, ViewerError> {
        if !inspect(source, limits).may_launch() {
            return Err(ViewerError::Refused);
        }
        let path = self.extract(source, consent, limits)?;
        // judged again from the copy that is about to be opened
        let copy = FileSource::new(&path)?;
        let class = inspect(&copy, limits);
        if !class.may_launch() {
            return Err(ViewerError::Refused);
        }
        if class.needs_consent() && consent != Consent::Given {
            return Err(ViewerError::ConsentRequired);
        }
        launcher.launch(&path)?;
        Ok(path)
    }
}

impl Drop for Outbox {
    fn drop(&mut self) {
        if let Some(root) = &self.root {
            // a file another program holds stays; the system cleans its temporary folder
            let _ = fs::remove_dir_all(root);
        }
    }
}
