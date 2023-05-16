use crate::document::{DocumentMap, Transaction};
use crate::editor::{EditorStateSummary, ModalEditorError};
use crate::events::{KeyCombo, KeyPatternClause};
use crate::events::{KeyMatcher, KeyPattern};

mod command_mode;
mod goto_mode;
mod insert_mode;
mod normal_mode;
mod selection_mode;

pub use command_mode::CommandMode;
pub use goto_mode::GotoMode;
pub use insert_mode::InsertMode;
pub use normal_mode::NormalMode;
pub use selection_mode::SelectionMode;

use super::{EditorAction, EditorCmd, EditorDisplay};

pub trait EditorMode: Send {
    fn id(&self) -> &'static str;
    fn handle_combo(&mut self, kc: &KeyCombo, state: &EditorStateSummary) -> EditorAction;
    fn get_display(&self, state: &EditorStateSummary) -> EditorDisplay;
}

/// Maps key patterns to editor actions.
#[derive(Clone, Debug)]
pub struct TriggerHandler {
    triggers: Vec<(KeyPattern, EditorAction)>,
}

impl Default for TriggerHandler {
    fn default() -> Self {
        TriggerHandler {
            triggers: Default::default(),
        }
    }
}

impl TriggerHandler {
    /// Associates a sequence of commands with the given key pattern.
    pub fn with<A, P, G>(mut self, clauses: P, action: A) -> Self
    where
        A: IntoIterator<Item = EditorCmd>,
        P: IntoIterator<Item = G>,
        G: IntoIterator<Item = KeyMatcher>,
    {
        self.triggers.push((
            clauses
                .into_iter()
                .map(|clause| clause.into_iter().collect())
                .collect(),
            action.into_iter().collect(),
        ));
        self
    }

    /// Returns the editor command that matches with the given key input combination.
    pub fn handle(&self, kc: &KeyCombo) -> Option<EditorAction> {
        self.triggers
            .iter()
            .find(|(pattern, _)| pattern.matches(kc.clone()))
            .map(|(_, resp)| resp.clone())
    }
}
