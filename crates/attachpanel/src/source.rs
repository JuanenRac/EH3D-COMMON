//! An attachment of a project as the viewer reads it: no copy is made on the disk to show it.

use std::io::{self, Read};

use eh3d_project::attachments::AttachmentSource;
use eh3d_viewer::{Danger, Source};

/// One attachment of a project file.
#[derive(Debug, Clone)]
pub struct ProjectSource(pub AttachmentSource);

impl Source for ProjectSource {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn len(&self) -> u64 {
        self.0.len()
    }

    fn reader(&self) -> io::Result<Box<dyn Read + '_>> {
        Ok(Box::new(self.0.reader()?))
    }

    /// The mark of the record: the viewer is at least as careful as the project says.
    fn declared_danger(&self) -> Option<Danger> {
        self.0.record().danger()
    }
}
