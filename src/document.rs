use crate::cursor::TextSelection;
use ropey::Rope;
use std::collections::HashMap;

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
pub struct DocumentMap(usize, HashMap<usize, Document>);

impl Default for DocumentMap {
    fn default() -> Self {
        Self(0, HashMap::from([(0, Document::new_empty())]))
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
}
