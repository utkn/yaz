use std::collections::HashSet;

use itertools::Itertools;

use super::{primitive_mods::*, DocumentMap};

/// Represents a transaction dependency.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransactionDep {
    DocumentSel(usize, usize),
    DocumentBuf(usize),
    Document(usize),
    DocumentMap,
}

/// Represents a sequence of primitive modifications.
#[derive(Clone, Debug)]
pub struct Transaction {
    pub primitive_mods: Vec<PrimitiveMod>,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            primitive_mods: Default::default(),
        }
    }
}

impl Transaction {
    pub fn new() -> Self {
        Transaction {
            primitive_mods: Default::default(),
        }
    }

    pub fn append_mod(&mut self, pmod: PrimitiveMod) {
        self.primitive_mods.push(pmod)
    }

    pub fn with_mod(mut self, pmod: PrimitiveMod) -> Self {
        self.append_mod(pmod);
        self
    }

    pub fn append_mods<T: IntoIterator<Item = PrimitiveMod>>(&mut self, it: T) {
        let mut coll = it.into_iter().collect_vec();
        self.primitive_mods.append(&mut coll)
    }

    pub fn with_mods<T: IntoIterator<Item = PrimitiveMod>>(mut self, it: T) -> Self {
        self.append_mods(it);
        self
    }

    /// Applies the transaction and returns the inverse transaction iff the application succeeds.
    pub fn apply_tx(&self, doc_map: &mut DocumentMap) -> Option<Transaction> {
        let mut inv_primitives = vec![];
        for pm in &self.primitive_mods {
            if let Some(pm_inv) = pm.apply(doc_map) {
                inv_primitives.push(pm_inv);
            } else {
                break;
            }
        }
        inv_primitives.reverse();
        if inv_primitives.len() != self.primitive_mods.len() {
            for pm_inv in inv_primitives {
                pm_inv.apply(doc_map);
            }
            None
        } else {
            Some(Transaction::new().with_mods(inv_primitives))
        }
    }

    /// Maps the given character index into a new index after the primitive modifications are applied.
    pub fn map_char_idx(&self, buf_id: &usize, old_idx: &usize) -> Option<usize> {
        let mut new_idx = *old_idx;
        for pm in &self.primitive_mods {
            match pm {
                PrimitiveMod::Text(mod_buf_id, text_mod) if mod_buf_id == buf_id => {
                    match text_mod {
                        BufMod::InsText(idx, txt) if old_idx >= idx => {
                            let added_txt_len = txt.chars().count();
                            new_idx += added_txt_len;
                        }
                        BufMod::DelRange(start_idx, end_idx) if old_idx > end_idx => {
                            let deleted_txt_len = end_idx - start_idx;
                            new_idx = new_idx.saturating_sub(deleted_txt_len);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Some(new_idx)
    }

    pub fn get_dependencies(&self) -> HashSet<TransactionDep> {
        let mut deps = self
            .primitive_mods
            .iter()
            .map(|m| match m {
                PrimitiveMod::Sel(doc_id, sel_id, _) => {
                    TransactionDep::DocumentSel(*doc_id, *sel_id)
                }
                PrimitiveMod::Text(doc_id, _) => TransactionDep::DocumentBuf(*doc_id),
                PrimitiveMod::DocMap(_) => TransactionDep::DocumentMap,
            })
            .collect::<HashSet<_>>();
        // Extend the dependencies with DocumentMap >= Document >= DocumentBuf, DocumentSel
        let all_doc_ids = deps
            .iter()
            .flat_map(|dep| match dep {
                TransactionDep::DocumentSel(doc_id, _)
                | TransactionDep::DocumentBuf(doc_id)
                | TransactionDep::Document(doc_id) => Some(*doc_id),
                TransactionDep::DocumentMap => None,
            })
            .collect::<HashSet<_>>();
        // If the transaction works on more than one unique documents, then it depends
        // on the whole document map.
        if all_doc_ids.len() > 1 {
            deps.insert(TransactionDep::DocumentMap);
        }
        // If the transaction works on both the text buffer and the selection of a document,
        // then it depends on the whole document.
        all_doc_ids.into_iter().for_each(|doc_id| {
            if deps
                .iter()
                .any(|dep| matches!(dep, TransactionDep::DocumentSel(id, _) if *id == doc_id))
                && deps
                    .iter()
                    .any(|dep| matches!(dep, TransactionDep::DocumentBuf(id) if *id == doc_id))
            {
                deps.insert(TransactionDep::Document(doc_id));
            }
        });
        deps
    }
}

impl FromIterator<PrimitiveMod> for Transaction {
    fn from_iter<T: IntoIterator<Item = PrimitiveMod>>(iter: T) -> Self {
        Self {
            primitive_mods: iter.into_iter().collect_vec(),
        }
    }
}
