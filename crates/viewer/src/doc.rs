//! A document as a preview shows it: headings, paragraphs, lists, code and tables, with no
//! formatting of the original. Every preview is bounded; what does not fit is left out and said.

use crate::limits::ViewerLimits;

/// One piece of a preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A level from 1 to 6.
    Heading(u8, String),
    Paragraph(String),
    /// A list item at a depth from 0.
    Item(u8, String),
    /// Text kept as it is, one line per line.
    Code(String),
    Quote(String),
    /// Rows of cells; the rows need not have the same length.
    Table(Vec<Vec<String>>),
    Rule,
}

/// The preview of a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preview {
    pub blocks: Vec<Block>,
    /// True when something was left out because of a limit.
    pub truncated: bool,
}

/// The longest cell or paragraph kept.
pub(crate) const MAX_PIECE_CHARS: usize = 20_000;
/// The most bytes of text of a whole preview: a value shown in many cells counts every time.
const MAX_PREVIEW_BYTES: usize = 16 * 1024 * 1024;
/// The most columns of a table.
pub(crate) const MAX_COLUMNS: usize = 256;

/// Builds a [`Preview`] within the limits.
pub(crate) struct Builder {
    preview: Preview,
    blocks_left: u64,
    cells_left: u64,
    bytes_left: usize,
}

impl Builder {
    pub(crate) fn new(limits: &ViewerLimits) -> Self {
        Builder {
            preview: Preview::default(),
            blocks_left: limits.max_blocks,
            cells_left: limits.max_cells,
            bytes_left: MAX_PREVIEW_BYTES,
        }
    }

    /// True when nothing more will be kept.
    pub(crate) fn full(&self) -> bool {
        self.blocks_left == 0
    }

    pub(crate) fn truncate(&mut self) {
        self.preview.truncated = true;
    }

    /// Cuts a text that is too long, at a character boundary, and counts it against the text of the whole preview. What is
    /// cut gives its memory back.
    pub(crate) fn clip(&mut self, mut text: String) -> String {
        if let Some((at, _)) = text.char_indices().nth(MAX_PIECE_CHARS) {
            text.truncate(at);
            self.truncate();
        }
        if text.len() > self.bytes_left {
            let mut end = self.bytes_left;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            self.truncate();
            // nothing more is kept
            self.blocks_left = 0;
            self.bytes_left = text.len();
        }
        self.bytes_left -= text.len();
        text.shrink_to_fit();
        text
    }

    pub(crate) fn push(&mut self, block: Block) {
        if self.blocks_left == 0 {
            self.truncate();
            return;
        }
        let block = match block {
            Block::Heading(level, t) => Block::Heading(level.clamp(1, 6), self.clip(t)),
            Block::Paragraph(t) => Block::Paragraph(self.clip(t)),
            Block::Item(depth, t) => Block::Item(depth.min(8), self.clip(t)),
            Block::Code(t) => Block::Code(self.clip(t)),
            Block::Quote(t) => Block::Quote(self.clip(t)),
            other => other,
        };
        // a text that used up the preview has already left no block
        self.blocks_left = self.blocks_left.saturating_sub(1);
        self.preview.blocks.push(block);
    }

    /// Adds one row to a table being read, within the limits of columns and cells.
    pub(crate) fn row(&mut self, table: &mut Vec<Vec<String>>, mut row: Vec<String>) {
        if row.len() > MAX_COLUMNS {
            row.truncate(MAX_COLUMNS);
            self.truncate();
        }
        let cost = row.len().max(1) as u64;
        if self.blocks_left == 0 || cost > self.cells_left {
            self.truncate();
            return;
        }
        self.cells_left -= cost;
        self.blocks_left -= 1;
        let row = row.into_iter().map(|c| self.clip(c)).collect();
        table.push(row);
    }

    /// Adds a table already read row by row with [`Builder::row`]; it costs no more blocks.
    pub(crate) fn table(&mut self, mut table: Vec<Vec<String>>) {
        while table.last().is_some_and(|r| r.iter().all(String::is_empty)) {
            table.pop();
        }
        if !table.is_empty() {
            self.preview.blocks.push(Block::Table(table));
        }
    }

    pub(crate) fn finish(self) -> Preview {
        self.preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limits_cut_blocks_cells_and_long_texts() {
        let limits = ViewerLimits {
            max_blocks: 3,
            max_cells: 4,
            ..ViewerLimits::default()
        };
        let mut builder = Builder::new(&limits);
        builder.push(Block::Paragraph("é".repeat(MAX_PIECE_CHARS + 5)));
        let mut table = Vec::new();
        builder.row(&mut table, vec!["a".into(); 3]);
        builder.row(&mut table, vec!["b".into(); 3]);
        builder.table(table);
        builder.push(Block::Rule);
        builder.push(Block::Rule);
        assert!(builder.full());
        let preview = builder.finish();
        assert!(preview.truncated);
        assert_eq!(preview.blocks.len(), 3);
        match (&preview.blocks[0], &preview.blocks[1]) {
            (Block::Paragraph(p), Block::Table(rows)) => {
                assert_eq!(p.chars().count(), MAX_PIECE_CHARS);
                assert_eq!(rows.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }
}
