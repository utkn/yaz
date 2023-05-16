use std::collections::VecDeque;

use crate::{
    document::{DocumentMap, Transaction},
    events::KeyCombo,
};

use super::TransactionGenerator;

#[derive(Clone, Debug, Default)]
pub struct EditorHistory {
    prev: VecDeque<Transaction>,
    next: VecDeque<Transaction>,
}

impl EditorHistory {
    fn undo(&mut self, doc_map: &mut DocumentMap) {
        self.prev
            .pop_front()
            .and_then(|m| m.apply_tx(doc_map))
            .map(|m_inv| {
                self.next.push_front(m_inv);
            });
    }

    fn redo(&mut self, doc_map: &mut DocumentMap) {
        self.next
            .pop_front()
            .and_then(|m| m.apply_tx(doc_map))
            .map(|m_inv| {
                self.prev.push_front(m_inv);
            });
    }

    fn next(&mut self, m: Transaction, doc_map: &mut DocumentMap) -> bool {
        self.next.clear();
        m.apply_tx(doc_map)
            .map(|m_inv| self.prev.push_front(m_inv))
            .is_some()
    }
}

#[derive(Clone, Debug)]
pub struct HistoricalEditorState {
    pub doc_map: DocumentMap,
    pub history: EditorHistory,
}

impl From<DocumentMap> for HistoricalEditorState {
    fn from(curr_state: DocumentMap) -> Self {
        HistoricalEditorState {
            doc_map: curr_state,
            history: Default::default(),
        }
    }
}

impl HistoricalEditorState {
    /// Moves the state one point back in the past.
    pub fn undo(&mut self) {
        self.history.undo(&mut self.doc_map);
    }

    /// Moves the state one point forward in the future.
    pub fn redo(&mut self) {
        self.history.redo(&mut self.doc_map);
    }

    /// Applies the transaction outputted by the given generator.
    pub fn modify_with_tx_gen(
        &mut self,
        trigger: &KeyCombo,
        tx_gen: &TransactionGenerator,
    ) -> bool {
        tx_gen.1(trigger, &self.doc_map)
            .map(|m| self.modify_with_tx(m))
            .unwrap_or(false)
    }

    /// Applies the given transaction.
    pub fn modify_with_tx(&mut self, tx: Transaction) -> bool {
        // Discard empty transactions
        if tx.primitive_mods.is_empty() {
            return false;
        }
        // Apply the modification to the appropriate history.
        self.history.next(tx, &mut self.doc_map)
    }
}
