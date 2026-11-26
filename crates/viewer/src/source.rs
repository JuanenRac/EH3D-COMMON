//! Where the bytes of an attachment come from. A program gives the viewer a [`Source`]; the viewer
//! never trusts the length it announces and stops one byte over every limit.

use std::fs::File;
use std::io::{self, Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::ViewerError;

/// One attachment: its name and a way to read its bytes from the start, as often as needed.
pub trait Source: Send + Sync {
    /// The file name the attachment has in the project. It is not trusted: it is cleaned before use.
    fn name(&self) -> &str;
    /// The size announced.
    fn len(&self) -> u64;
    /// A reader from the first byte.
    fn reader(&self) -> io::Result<Box<dyn Read + '_>>;
    /// True when nothing is announced.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An attachment held in memory.
#[derive(Debug, Clone)]
pub struct MemorySource {
    name: String,
    data: Arc<Vec<u8>>,
}

impl MemorySource {
    pub fn new(name: impl Into<String>, data: Vec<u8>) -> Self {
        MemorySource {
            name: name.into(),
            data: Arc::new(data),
        }
    }
}

impl Source for MemorySource {
    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> u64 {
        self.data.len() as u64
    }

    fn reader(&self) -> io::Result<Box<dyn Read + '_>> {
        Ok(Box::new(Cursor::new(self.data.as_slice())))
    }
}

/// An attachment that is a file on the computer.
#[derive(Debug, Clone)]
pub struct FileSource {
    name: String,
    path: PathBuf,
    len: u64,
}

impl FileSource {
    /// Opens nothing yet; the name of the attachment is the file name of the path.
    pub fn new(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let meta = std::fs::metadata(&path)?;
        if !meta.is_file() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file"));
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(FileSource {
            name,
            path,
            len: meta.len(),
        })
    }
}

impl Source for FileSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> u64 {
        self.len
    }

    fn reader(&self) -> io::Result<Box<dyn Read + '_>> {
        Ok(Box::new(File::open(&self.path)?))
    }
}

/// The first `count` bytes (fewer when the attachment is shorter).
pub fn read_head(source: &dyn Source, count: usize) -> Result<Vec<u8>, ViewerError> {
    let mut head = Vec::new();
    source.reader()?.take(count as u64).read_to_end(&mut head)?;
    Ok(head)
}

/// Every byte, refused when there are more than `limit`, whatever the length announced.
pub fn read_all(source: &dyn Source, limit: u64) -> Result<Vec<u8>, ViewerError> {
    if source.len() > limit {
        return Err(ViewerError::TooLarge { limit });
    }
    let mut data = Vec::new();
    source
        .reader()?
        .take(limit.saturating_add(1))
        .read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(ViewerError::TooLarge { limit });
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Liar;

    impl Source for Liar {
        fn name(&self) -> &str {
            "liar.txt"
        }
        fn len(&self) -> u64 {
            1
        }
        fn reader(&self) -> io::Result<Box<dyn Read + '_>> {
            Ok(Box::new(io::repeat(b'a')))
        }
    }

    #[test]
    fn a_source_that_announces_less_than_it_gives_is_cut_at_the_limit() {
        assert_eq!(
            read_all(&Liar, 100),
            Err(ViewerError::TooLarge { limit: 100 })
        );
        assert_eq!(read_head(&Liar, 4).unwrap(), b"aaaa");
    }

    #[test]
    fn a_source_within_the_limit_is_read_whole() {
        let source = MemorySource::new("a.txt", b"hello".to_vec());
        assert_eq!(read_all(&source, 5).unwrap(), b"hello");
        assert_eq!(
            read_all(&source, 4),
            Err(ViewerError::TooLarge { limit: 4 })
        );
    }
}
