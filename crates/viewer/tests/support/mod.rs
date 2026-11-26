//! Synthetic files and doubles shared by the tests. Every file is made here; none is a real document.

// each test binary uses part of this module
#![allow(dead_code)]

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eh3d_viewer::outside::Launcher;
use eh3d_viewer::ViewerError;

pub fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A folder of its own, removed when dropped.
pub struct Folder(pub PathBuf);

impl Folder {
    pub fn new(what: &str) -> Folder {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "eh3d-viewer-test-{what}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Folder(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Remembers what it was asked to open, and opens nothing.
#[derive(Clone, Default)]
pub struct Recorder(pub Arc<Mutex<Vec<PathBuf>>>);

impl Recorder {
    pub fn opened(&self) -> Vec<PathBuf> {
        self.0.lock().unwrap().clone()
    }
}

impl Launcher for Recorder {
    fn launch(&mut self, path: &Path) -> Result<(), ViewerError> {
        self.0.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

/// Waits for a condition, with a bound, instead of counting rounds.
pub fn until(what: &str, mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(30);
    while !condition() {
        assert!(Instant::now() < end, "timed out: {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// A PDF of `texts.len()` pages of 200 by 100 points, each with its text in Helvetica at 12 points,
/// starting 20 points from the left and 50 from the bottom. The document also carries a script to run
/// when it is opened, which a build without a script engine cannot run.
pub fn pdf(texts: &[&str]) -> Vec<u8> {
    let pages = texts.len();
    let kids: Vec<String> = (0..pages).map(|p| format!("{} 0 R", 5 + p * 2)).collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /OpenAction 4 0 R >>".to_string(),
        format!(
            "<< /Type /Pages /Count {pages} /Kids [{}] >>",
            kids.join(" ")
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        "<< /Type /Action /S /JavaScript /JS (app.alert\\(1\\)) >>".to_string(),
    ];
    for (p, text) in texts.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            6 + p * 2
        ));
        let content = format!("BT /F1 12 Tf 20 50 Td ({text}) Tj ET");
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let table = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}
