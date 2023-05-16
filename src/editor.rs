use std::collections::{HashMap, VecDeque};

use crate::{
    document::{Document, DocumentMap, Transaction},
    events::{Key, KeyCombo, KeyEvt, KeyMods},
};

use self::editor_mode::EditorMode;

mod editor_history;
pub mod editor_mode;
pub mod editor_server;

pub use editor_history::HistoricalEditorState;
use itertools::Itertools;

/// Represents a named function that outputs a transaction.
#[derive(Clone, Copy)]
pub struct TransactionGenerator(
    pub &'static str,
    pub fn(&KeyCombo, &DocumentMap) -> Option<Transaction>,
);

impl PartialEq for TransactionGenerator {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Debug for TransactionGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ModGenerator({})", self.0))
    }
}

#[derive(Clone, Debug)]
pub enum EditorCmd {
    UndoCurrDocument,
    RedoCurrDocument,
    SaveCurrDocument(Option<String>),
    Transaction(TransactionGenerator),
    PushMode(&'static str),
    PopMode,
    ResetCombo,
    Quit,
    ThrowErr(ModalEditorError),
}

#[derive(Clone, Debug, Default)]
pub struct EditorAction(Vec<EditorCmd>);

impl EditorAction {
    pub fn append(&mut self, cmd: EditorCmd) {
        self.0.push(cmd)
    }

    pub fn prepend(&mut self, cmd: EditorCmd) {
        self.0.insert(0, cmd)
    }
}

impl FromIterator<EditorCmd> for EditorAction {
    fn from_iter<T: IntoIterator<Item = EditorCmd>>(iter: T) -> Self {
        Self(iter.into_iter().collect_vec())
    }
}

impl IntoIterator for EditorAction {
    type Item = EditorCmd;

    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Represents a named function that outputs a squence of editor commands.
#[derive(Copy, Clone)]
pub struct ActionGenerator(
    &'static str,
    fn(&[&str], state: &EditorStateSummary) -> Option<EditorAction>,
);

impl ActionGenerator {
    pub fn name(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Debug for ActionGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ActionGenerator({})", self.0))
    }
}

#[derive(Clone, Debug, Default)]
pub struct EditorDisplay {
    pub btm_bar_text: Option<String>,
    pub right_box_text: Option<String>,
    pub mid_box_text: Option<String>,
    pub cursor_text: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum ModalEditorResult {
    QuitRequested,
    StateUpdated,
}

#[derive(Clone, Debug)]
pub struct ModalEditorError(pub String);

impl std::fmt::Display for ModalEditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ModalEditorError {}

pub struct ModalEditor {
    historical_state: HistoricalEditorState,
    registered_modes: HashMap<&'static str, Box<dyn EditorMode>>,
    active_modes: VecDeque<&'static str>,
    curr_combo: KeyCombo,
}

impl ModalEditor {
    pub fn new(historical_state: HistoricalEditorState, base_mode: &'static str) -> Self {
        ModalEditor {
            historical_state,
            registered_modes: Default::default(),
            active_modes: VecDeque::from([base_mode]),
            curr_combo: Default::default(),
        }
    }
}

impl ModalEditor {
    pub fn with_mode(mut self, mode: Box<dyn EditorMode>) -> Self {
        self.registered_modes.insert(mode.id(), mode);
        self
    }

    pub fn receive_key(&mut self, evt: KeyEvt) {
        self.curr_combo.add(evt)
    }

    pub fn curr_mode_mut(&mut self) -> Option<&mut Box<dyn EditorMode>> {
        let curr_mode_name = self.active_modes.front()?;
        self.registered_modes.get_mut(curr_mode_name)
    }

    pub fn curr_mode(&self) -> Option<&Box<dyn EditorMode>> {
        let curr_mode_name = self.active_modes.front()?;
        self.registered_modes.get(curr_mode_name)
    }

    /// Updates the editor with the action induced by the current mode.
    /// May also change the mode or reset the current key combo if appropriate.
    pub fn update(&mut self) -> Result<ModalEditorResult, ModalEditorError> {
        // Get the current state summary.
        let state_summary = self.summary();
        // Reset key clears the combo.
        if self.curr_combo.len() > 1
            && self
                .curr_combo
                .ends_with([KeyEvt::Key(Key::Esc, KeyMods::NONE)])
        {
            self.curr_combo.reset();
            return Ok(ModalEditorResult::StateUpdated);
        }
        // Try to handle the current key combo with the current mode.
        let curr_combo = self.curr_combo.clone();
        let mut results = vec![];
        if let Some(curr_mode) = self.curr_mode_mut() {
            let action = curr_mode.handle_combo(&curr_combo, &state_summary);
            for cmd in action {
                let result = match cmd {
                    EditorCmd::UndoCurrDocument => {
                        self.historical_state.undo();
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::RedoCurrDocument => {
                        self.historical_state.redo();
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::Transaction(tx_gen)
                        if self
                            .historical_state
                            .modify_with_tx_gen(&self.curr_combo, &tx_gen) =>
                    {
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::PushMode(mode_name)
                        if self.registered_modes.contains_key(mode_name) =>
                    {
                        self.active_modes.push_front(mode_name);
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::PopMode if self.active_modes.len() > 1 => {
                        self.active_modes.pop_front();
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::ResetCombo => {
                        self.curr_combo.reset();
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::SaveCurrDocument(file_path) => {
                        let curr_buf = self.historical_state.doc_map.get_curr_doc_mut();
                        if let Some(file_path) = file_path {
                            curr_buf
                                .and_then(|buf| buf.save_as(&file_path).ok())
                                .ok_or(ModalEditorError("could not save".to_string()))?;
                        } else {
                            curr_buf
                                .and_then(|buf| buf.save().ok())
                                .ok_or(ModalEditorError("could not save".to_string()))?;
                        }
                        Some(ModalEditorResult::StateUpdated)
                    }
                    EditorCmd::Quit => {
                        return Ok(ModalEditorResult::QuitRequested);
                    }
                    EditorCmd::ThrowErr(err) => {
                        self.curr_combo.reset();
                        return Err(err);
                    }
                    _ => None,
                };
                result.map(|r| results.push(r));
            }
        }
        if results.len() > 0 {
            self.curr_combo.reset();
        }
        Ok(ModalEditorResult::StateUpdated)
    }

    pub fn summary(&self) -> EditorStateSummary {
        let mut summary = EditorStateSummary {
            curr_doc: self
                .historical_state
                .doc_map
                .get_curr_doc()
                .cloned() // TODO optimize
                .unwrap_or(Document::new_empty()),
            curr_buffer_idx: self.historical_state.doc_map.curr_doc_id(),
            curr_mode: self.curr_mode().map(|mode| mode.id()).unwrap_or_default(),
            curr_combo: self.curr_combo.clone(),
            display: EditorDisplay::default(),
        };
        if let Some(display) = self.curr_mode().map(|m| m.get_display(&summary)) {
            summary.display = display
        }
        summary
    }
}

#[derive(Clone, Debug)]
pub struct EditorStateSummary {
    pub curr_doc: Document,
    pub curr_buffer_idx: usize,
    pub curr_mode: &'static str,
    pub curr_combo: KeyCombo,
    pub display: EditorDisplay,
}

impl Default for EditorStateSummary {
    fn default() -> Self {
        EditorStateSummary {
            curr_mode: "none",
            curr_doc: Document::new_empty(),
            curr_buffer_idx: 0,
            curr_combo: Default::default(),
            display: Default::default(),
        }
    }
}
