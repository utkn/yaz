use ropey::Rope;

use crate::cursor::TextSelection;

use super::{Document, DocumentMap};

#[derive(Clone, Debug)]
pub enum BufMod {
    InsText(usize, String),
    DelRange(usize, usize),
}

impl BufMod {
    fn apply(&self, buf: &mut Rope) -> Option<Self> {
        match self {
            BufMod::InsText(char_idx, s) => buf
                .try_insert(*char_idx, &s)
                .ok()
                .map(|_| BufMod::DelRange(*char_idx, char_idx + s.len())),
            BufMod::DelRange(start_char_idx, end_char_idx) => {
                if let Some(old_txt) = buf
                    .get_slice(start_char_idx..end_char_idx)
                    .map(|old_slice| old_slice.to_string())
                {
                    buf.try_remove(start_char_idx..end_char_idx)
                        .ok()
                        .map(|_| BufMod::InsText(*start_char_idx, old_txt))
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SelectionMod {
    SetHead(usize),
    SetTail(Option<usize>),
}

impl SelectionMod {
    fn apply(&self, sel: &mut TextSelection) -> Option<Self> {
        match self {
            SelectionMod::SetHead(new_char_idx) => {
                let old_pos = sel.0;
                sel.0 = *new_char_idx;
                Some(SelectionMod::SetHead(old_pos))
            }
            SelectionMod::SetTail(new_tail) => {
                let old_tail = sel.1;
                sel.1 = *new_tail;
                Some(SelectionMod::SetTail(old_tail))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum DocMapMod {
    SwitchDoc(usize),
    CreateDoc(Document),
    PopDoc(usize),
    DeleteSel(usize, usize),
    CreateSel(usize, usize, TextSelection),
}

impl DocMapMod {
    fn apply(&self, doc_map: &mut DocumentMap) -> Option<Self> {
        match self {
            DocMapMod::SwitchDoc(new_doc_id) => {
                if doc_map.contains_key(new_doc_id) {
                    let old_doc_id = doc_map.curr_doc_id();
                    doc_map.set_curr_doc_id(*new_doc_id);
                    Some(old_doc_id)
                } else {
                    None
                }
            }
            .map(|old_doc_id| DocMapMod::SwitchDoc(old_doc_id)),
            DocMapMod::CreateDoc(new_doc) => {
                // TODO optimize cloning
                let new_doc_id = {
                    let new_doc_id = doc_map.insert(new_doc.clone());
                    new_doc_id
                };
                Some(DocMapMod::PopDoc(new_doc_id))
            }
            DocMapMod::PopDoc(doc_id) => doc_map
                .remove(doc_id)
                .map(|removed_doc| DocMapMod::CreateDoc(removed_doc)),
            DocMapMod::DeleteSel(doc_id, sel_id) => {
                let sel = doc_map.get_mut(doc_id)?.selections.remove(sel_id)?;
                Some(DocMapMod::CreateSel(*doc_id, *sel_id, sel))
            }
            DocMapMod::CreateSel(doc_id, sel_id, sel) => {
                doc_map
                    .get_mut(doc_id)?
                    .selections
                    .insert(*sel_id, sel.clone());
                Some(DocMapMod::DeleteSel(*doc_id, *sel_id))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum PrimitiveMod {
    Sel(usize, usize, SelectionMod),
    Text(usize, BufMod),
    DocMap(DocMapMod),
}

impl PrimitiveMod {
    pub fn apply(&self, doc_map: &mut DocumentMap) -> Option<Self> {
        match self {
            PrimitiveMod::Sel(doc_id, sel_id, sel_mod) => doc_map
                .get_mut(doc_id)
                .and_then(|doc| doc.selections.get_mut(sel_id))
                .and_then(|sel| sel_mod.apply(sel))
                .map(|sel_mod| PrimitiveMod::Sel(*doc_id, *sel_id, sel_mod)),
            PrimitiveMod::Text(doc_id, text_mod) => doc_map
                .get_mut(doc_id)
                .and_then(|doc| text_mod.apply(&mut doc.inner_buf))
                .map(|text_mod| PrimitiveMod::Text(*doc_id, text_mod)),
            PrimitiveMod::DocMap(editor_mod) => editor_mod
                .apply(doc_map)
                .map(|editor_mod| PrimitiveMod::DocMap(editor_mod)),
        }
    }
}
