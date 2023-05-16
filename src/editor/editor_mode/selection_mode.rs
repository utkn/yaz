use crate::{
    editor::{EditorAction, EditorStateSummary},
    events::{Key, KeyCombo, KeyEvt, KeyMods},
};

use super::{normal_mode::*, EditorCmd, EditorMode, InsertMode, NormalMode};

pub struct SelectionMode {
    normal_mode: NormalMode,
}

impl SelectionMode {
    pub fn new() -> Self {
        SelectionMode {
            normal_mode: NormalMode::new(),
        }
    }

    pub fn id() -> &'static str {
        "selection"
    }
}

impl EditorMode for SelectionMode {
    fn id(&self) -> &'static str {
        Self::id()
    }

    fn handle_combo(&mut self, kc: &KeyCombo, state: &EditorStateSummary) -> EditorAction {
        if kc.len() == 1 && kc.ends_with([KeyEvt::Key(Key::Esc, KeyMods::NONE)]) {
            return [EditorCmd::Transaction(COLLAPSE_SELS), EditorCmd::PopMode]
                .into_iter()
                .collect();
        }
        self.normal_mode
            .handle_combo(kc, state)
            .into_iter()
            .flat_map(|mode_resp| match mode_resp {
                EditorCmd::Transaction(cmd) if cmd == COLLAPSE_SELS => None,
                EditorCmd::PushMode(mode_id) if mode_id == InsertMode::id() => None,
                EditorCmd::PushMode(mode_id) if mode_id == self.id() => None,
                _ => Some(mode_resp),
            })
            .collect()
    }

    fn get_display(&self, _state: &EditorStateSummary) -> super::EditorDisplay {
        Default::default()
    }
}
