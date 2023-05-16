use std::collections::HashMap;

use itertools::Itertools;
use macros::action_generator;

use crate::{
    editor::{
        ActionGenerator, EditorAction, EditorCmd, EditorDisplay, EditorStateSummary,
        ModalEditorError,
    },
    events::{Key, KeyCombo, KeyEvt, KeyMods},
};

use super::EditorMode;

#[action_generator]
fn quit(_args: &[&str], _state: &EditorStateSummary) -> Option<EditorAction> {
    Some([EditorCmd::Quit].into_iter().collect())
}

#[action_generator]
fn save(args: &[&str], _state: &EditorStateSummary) -> Option<EditorAction> {
    Some(
        [EditorCmd::SaveCurrDocument(
            args.get(0).map(|path| path.to_string()),
        )]
        .into_iter()
        .collect(),
    )
}

pub struct CommandMode {
    curr_cmd: String,
    cmd_generators: HashMap<&'static str, ActionGenerator>,
}

const ALL_COMMANDS: &[ActionGenerator] = &[QUIT, SAVE];

impl CommandMode {
    pub fn new() -> Self {
        let mut cmd_mode = CommandMode {
            curr_cmd: String::new(),
            cmd_generators: Default::default(),
        };
        for cmd in ALL_COMMANDS {
            cmd_mode.register_command(*cmd);
        }
        cmd_mode
    }
}

impl CommandMode {
    pub fn id() -> &'static str {
        "command"
    }

    pub fn register_command(&mut self, cmd_gen: ActionGenerator) {
        self.cmd_generators.insert(cmd_gen.name(), cmd_gen);
    }

    pub fn similar_cmd_generators(&self, limit: usize) -> Vec<&ActionGenerator> {
        use rust_fuzzy_search::fuzzy_search_best_n;
        let all_cmds = self.cmd_generators.keys().cloned().collect_vec();
        fuzzy_search_best_n(&self.curr_cmd, &all_cmds, limit)
            .into_iter()
            .map(|(cmd_key, _)| cmd_key)
            .filter(|cmd_key| cmd_key.len() >= self.curr_cmd.len())
            .map(|cmd_key| self.cmd_generators.get(cmd_key).unwrap())
            .collect_vec()
    }
}

impl EditorMode for CommandMode {
    fn id(&self) -> &'static str {
        Self::id()
    }

    fn handle_combo(&mut self, kc: &KeyCombo, state: &EditorStateSummary) -> EditorAction {
        // Exit with discard
        if kc.len() == 1 && kc.ends_with([KeyEvt::Key(Key::Esc, KeyMods::NONE)]) {
            self.curr_cmd = String::new();
            return [EditorCmd::PopMode].into_iter().collect();
        }
        // Exit with accept
        if kc.len() == 1
            && kc.ends_with([KeyEvt::Key(Key::Enter, KeyMods::NONE)])
            && self.curr_cmd.len() > 0
        {
            // Extract the current command
            let mut full_cmd_str = String::new();
            std::mem::swap(&mut full_cmd_str, &mut self.curr_cmd);
            let mut args = full_cmd_str.trim().split_whitespace();
            let target_cmd = args.next().unwrap_or_default();
            let args = args.collect_vec();
            return if let Some(cmd_gen) = self.cmd_generators.get(&target_cmd) {
                let mut generated_action = cmd_gen.1(&args, state).unwrap_or(
                    [EditorCmd::ThrowErr(ModalEditorError(
                        "couldn't apply action".to_string(),
                    ))]
                    .into_iter()
                    .collect(),
                );
                generated_action.prepend(EditorCmd::ResetCombo);
                generated_action.prepend(EditorCmd::PopMode);
                generated_action
            } else {
                [
                    EditorCmd::PopMode,
                    EditorCmd::ThrowErr(ModalEditorError(format!(
                        "invalid command `{}`",
                        target_cmd
                    ))),
                ]
                .into_iter()
                .collect()
            };
        }
        // Autocomplete on tab.
        if kc.len() == 1 && kc.ends_with([KeyEvt::Key(Key::Tab, KeyMods::NONE)]) {
            if let Some(most_similar_cmd_gen) = self.similar_cmd_generators(1).first() {
                self.curr_cmd = most_similar_cmd_gen.name().to_string();
            }
        }
        // Delete the command on backspace.
        if kc.len() == 1 && kc.ends_with([KeyEvt::Key(Key::Backspace, KeyMods::NONE)]) {
            self.curr_cmd = self.curr_cmd[0..self.curr_cmd.len().saturating_sub(1)].to_string();
        }
        // Mutate the command
        let additional_txt = kc
            .extract_text()
            .replace("\n", "")
            .replace("\t", " ")
            .replace(":", "");
        self.curr_cmd.push_str(&additional_txt);
        return [EditorCmd::ResetCombo].into_iter().collect();
    }

    fn get_display(&self, _state: &EditorStateSummary) -> EditorDisplay {
        let mut similar_cmds_str = self
            .similar_cmd_generators(5)
            .iter()
            .map(|cmd_gen| cmd_gen.name())
            .join("\t");
        if similar_cmds_str.is_empty() {
            similar_cmds_str = "no similar command".into();
        }
        EditorDisplay {
            btm_bar_text: Some(format!(":{}", self.curr_cmd.clone())),
            mid_box_text: Some(similar_cmds_str),
            ..Default::default()
        }
    }
}
