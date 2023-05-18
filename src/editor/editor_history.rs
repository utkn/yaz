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
    /// Undoes the state. Returns the applied transaction.
    fn undo(&mut self, doc_map: &mut DocumentMap) -> Option<Transaction> {
        let prev_tx = self.prev.pop_front();
        prev_tx
            .clone()
            .and_then(|m| m.apply_tx(doc_map))
            .map(|m_inv| {
                self.next.push_front(m_inv);
            });
        prev_tx
    }

    /// Redoes the state. Returns the applied transaction.
    fn redo(&mut self, doc_map: &mut DocumentMap) -> Option<Transaction> {
        let next_tx = self.next.pop_front();
        next_tx
            .clone()
            .and_then(|m| m.apply_tx(doc_map))
            .map(|m_inv| {
                self.prev.push_front(m_inv);
            });
        next_tx
    }

    /// Moves forward with the given transaction. Returns true if the application
    /// is successful.
    fn next(&mut self, m: &Transaction, doc_map: &mut DocumentMap) -> bool {
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
    /// Returns the applied transaction.
    pub fn undo(&mut self) -> Option<Transaction> {
        self.history.undo(&mut self.doc_map)
    }

    /// Moves the state one point forward in the future.
    /// Returns the applied transaction.
    pub fn redo(&mut self) -> Option<Transaction> {
        self.history.redo(&mut self.doc_map)
    }

    /// Applies the transaction outputted by the given generator.
    /// Returns the applied transaction.
    pub fn modify_with_tx_gen(
        &mut self,
        trigger: &KeyCombo,
        tx_gen: &TransactionGenerator,
    ) -> Option<Transaction> {
        tx_gen.1(trigger, &self.doc_map).filter(|tx| self.modify_with_tx(&tx))
    }

    /// Applies the given transaction.
    /// Returns true iff the transaction is applied successfully
    pub fn modify_with_tx(&mut self, tx: &Transaction) -> bool {
        // Empty transactions do not have any effect and always succeed.
        if tx.primitive_mods.is_empty() {
            return true;
        }
        // Apply the modification to the appropriate history.
        self.history.next(tx, &mut self.doc_map)
    }
}
