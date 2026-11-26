//! Every size the viewer allows itself, in one place and changeable from the settings of a program.

const MIB: u64 = 1024 * 1024;

/// The limits of the viewer. The defaults are conservative; a program may change them with
/// [`ViewerLimits::apply`] from its settings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerLimits {
    /// The largest attachment that is shown, copied out or saved (1 GiB).
    pub max_attachment_bytes: u64,
    /// The largest PDF shown inside (it is held in memory).
    pub max_pdf_bytes: u64,
    /// The largest picture file shown inside.
    pub max_image_file_bytes: u64,
    /// The most pixels a picture may have.
    pub max_image_pixels: u64,
    /// The bytes of a text file that are shown; the rest is cut.
    pub max_text_bytes: u64,
    /// The largest office document that is previewed.
    pub max_office_file_bytes: u64,
    /// The most bytes one part of an office document may unpack to.
    pub max_office_part_bytes: u64,
    /// The most bytes all the parts read of an office document may unpack to.
    pub max_office_total_bytes: u64,
    /// The most entries an office archive may have.
    pub max_office_entries: u64,
    /// The most paragraphs, headings and rows of a preview.
    pub max_blocks: u64,
    /// The most table cells of a preview.
    pub max_cells: u64,
    /// The most pixels of one rendered PDF page.
    pub max_render_pixels: u64,
    /// The most matches a search of a PDF keeps.
    pub max_search_hits: u64,
}

impl Default for ViewerLimits {
    fn default() -> Self {
        ViewerLimits {
            max_attachment_bytes: 1024 * MIB,
            max_pdf_bytes: 256 * MIB,
            max_image_file_bytes: 64 * MIB,
            max_image_pixels: 48_000_000,
            max_text_bytes: 2 * MIB,
            max_office_file_bytes: 128 * MIB,
            max_office_part_bytes: 32 * MIB,
            max_office_total_bytes: 96 * MIB,
            max_office_entries: 20_000,
            max_blocks: 20_000,
            max_cells: 100_000,
            max_render_pixels: 16_000_000,
            max_search_hits: 2_000,
        }
    }
}

impl ViewerLimits {
    /// Takes one line of a settings file. The keys are `viewer.max_attachment_mb`,
    /// `viewer.max_pdf_mb`, `viewer.max_image_mb`, `viewer.max_text_mb` and
    /// `viewer.max_office_mb`. Returns false for a key that is not of the viewer or a value that is
    /// not a whole number from 1 to 4096; the limit then stays as it was.
    pub fn apply(&mut self, key: &str, value: &str) -> bool {
        let Ok(mb) = value.trim().parse::<u64>() else {
            return false;
        };
        if !(1..=4096).contains(&mb) {
            return false;
        }
        let slot = match key.trim() {
            "viewer.max_attachment_mb" => &mut self.max_attachment_bytes,
            "viewer.max_pdf_mb" => &mut self.max_pdf_bytes,
            "viewer.max_image_mb" => &mut self.max_image_file_bytes,
            "viewer.max_text_mb" => &mut self.max_text_bytes,
            "viewer.max_office_mb" => &mut self.max_office_file_bytes,
            _ => return false,
        };
        *slot = mb * MIB;
        true
    }

    /// A limit of one kind of preview is never above the limit of an attachment.
    pub fn capped(&self, limit: u64) -> u64 {
        limit.min(self.max_attachment_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_of_an_attachment_is_one_gigabyte_and_the_settings_change_it() {
        let mut limits = ViewerLimits::default();
        assert_eq!(limits.max_attachment_bytes, 1_073_741_824);
        assert!(limits.apply("viewer.max_attachment_mb", " 10 "));
        assert_eq!(limits.max_attachment_bytes, 10 * MIB);
        assert_eq!(limits.capped(limits.max_pdf_bytes), 10 * MIB);
    }

    #[test]
    fn a_bad_value_or_an_unknown_key_changes_nothing() {
        let mut limits = ViewerLimits::default();
        for (key, value) in [
            ("viewer.max_pdf_mb", "0"),
            ("viewer.max_pdf_mb", "5000"),
            ("viewer.max_pdf_mb", "-1"),
            ("viewer.max_pdf_mb", "ten"),
            ("viewer.other", "5"),
        ] {
            assert!(!limits.apply(key, value), "{key}={value}");
        }
        assert_eq!(limits, ViewerLimits::default());
    }
}
