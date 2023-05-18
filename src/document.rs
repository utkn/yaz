use crate::cursor::GraphemeIterable;
use crate::cursor::TextSelection;
use ropey::Rope;
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

pub mod primitive_mods;
mod transaction;

pub use transaction::Transaction;
pub use transaction::TransactionDep;

#[derive(Clone, Debug, Default)]
pub struct DocumentSource(Option<String>);

impl std::fmt::Display for DocumentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.0 {
            f.write_str(path)
        } else {
            f.write_str("[scratch]")
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DocumentView {
    pub x_offset: usize,
    pub y_offset: usize,
    pub max_height: usize,
    pub max_width: usize,
}

impl DocumentView {
    /// Returns the approximate number of chars displayed in the view.
    /// Can be used for optimization.
    pub fn approx_displayed_len_chars(&self, buf: &Rope) -> usize {
        buf.lines()
            .skip(self.y_offset)
            .take(self.max_height)
            .map(|line| {
                line.chunks()
                    .map(|s| (s.chars().count(), s.width()))
                    .scan(0, |curr_width_sum, (char_count, w)| {
                        *curr_width_sum += w;
                        Some((char_count, *curr_width_sum))
                    })
                    .skip_while(|(_, w_sum)| *w_sum < self.x_offset)
                    .take_while(|(_, w_sum)| *w_sum < self.max_width)
                    .map(|(char_count, _)| char_count)
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn map_to_visual_position(char_idx: usize, buf: &Rope) -> (usize, usize) {
        let y_offset = buf.try_char_to_line(char_idx).unwrap_or(0);
        let line_start = buf.try_line_to_char(y_offset).unwrap_or(0);
        let char_offset_at_line = char_idx - line_start;
        let x_offset = buf
            .graphemes(line_start)
            .map(|g| (g.chars().count(), g.width()))
            .scan((0, 0), |curr_sum, (char_count, width)| {
                curr_sum.0 += char_count;
                curr_sum.1 += width;
                Some(*curr_sum)
            })
            .take_while(|(c_sum, _)| *c_sum < char_offset_at_line)
            .map(|(_, w_sum)| w_sum)
            .last()
            .unwrap_or(0);
        (x_offset, y_offset)
    }

    pub fn y_offset(char_idx: usize, buf: &Rope) -> usize {
        let y_offset = buf.try_char_to_line(char_idx).unwrap_or(0);
        y_offset
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub source: DocumentSource,
    pub selections: HashMap<usize, TextSelection>,
    pub dirty: bool,
    inner_buf: Rope,
}

impl Document {
    pub fn new_empty() -> Self {
        Document {
            selections: HashMap::from([(0, TextSelection::default())]),
            inner_buf: ropey::Rope::new(),
            source: Default::default(),
            dirty: false,
        }
    }

    pub fn new_from_file(file_path: &str) -> Self {
        if let Ok(file_str) = std::fs::read_to_string(file_path) {
            Document {
                selections: HashMap::from([(0, TextSelection::default())]),
                inner_buf: ropey::Rope::from_str(&file_str),
                source: DocumentSource(Some(file_path.to_string())),
                dirty: false,
            }
        } else {
            Self::new_empty()
        }
    }

    pub fn get_buf(&self) -> &Rope {
        &self.inner_buf
    }

    pub fn get_buf_mut(&mut self) -> &mut Rope {
        self.dirty = true;
        &mut self.inner_buf
    }

    pub fn save(&mut self) -> Result<(), std::io::Error> {
        if let DocumentSource(Some(path)) = &self.source {
            std::fs::write(path, self.inner_buf.to_string())?;
            self.dirty = false;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "buffer has no source",
            ))
        }
    }

    pub fn save_as(&mut self, new_file_path: &str) -> Result<(), std::io::Error> {
        std::fs::write(new_file_path, self.inner_buf.to_string())?;
        self.source = DocumentSource(Some(new_file_path.to_string()));
        self.dirty = false;
        Ok(())
    }

    pub fn get_ext(&self) -> Option<&str> {
        self.source
            .0
            .as_ref()
            .and_then(|path| path.split('.').last())
    }
}

impl From<DocumentSource> for Document {
    fn from(value: DocumentSource) -> Self {
        if let Some(path) = value.0 {
            Self::new_from_file(&path)
        } else {
            Self::new_empty()
        }
    }
}

/// Represents a collection of documents.
#[derive(Clone, Debug)]
pub struct DocumentMap(usize, HashMap<usize, Document>, DocumentView);

impl Default for DocumentMap {
    fn default() -> Self {
        Self(
            0,
            HashMap::from([(0, Document::new_empty())]),
            Default::default(),
        )
    }
}

impl DocumentMap {
    pub fn contains_key(&self, id: &usize) -> bool {
        self.1.contains_key(id)
    }

    pub fn curr_doc_id(&self) -> usize {
        self.0
    }

    pub fn set_curr_doc_id(&mut self, new_doc_id: usize) {
        self.0 = new_doc_id;
    }

    fn get_unused_id(&self) -> usize {
        self.1.keys().max().map(|buf_id| buf_id + 1).unwrap_or(0)
    }

    pub fn insert(&mut self, doc: Document) -> usize {
        let new_id = self.get_unused_id();
        self.1.insert(new_id, doc);
        new_id
    }

    pub fn remove(&mut self, id: &usize) -> Option<Document> {
        self.1.remove(id)
    }

    pub fn get(&self, id: &usize) -> Option<&Document> {
        self.1.get(id)
    }

    pub fn get_mut(&mut self, id: &usize) -> Option<&mut Document> {
        self.1.get_mut(id)
    }

    pub fn get_curr_doc(&self) -> Option<&Document> {
        self.get(&self.curr_doc_id())
    }

    pub fn get_curr_doc_mut(&mut self) -> Option<&mut Document> {
        self.get_mut(&self.curr_doc_id())
    }

    pub fn get_view(&self) -> &DocumentView {
        &self.2
    }

    pub fn get_view_mut(&mut self) -> &mut DocumentView {
        &mut self.2
    }
}
