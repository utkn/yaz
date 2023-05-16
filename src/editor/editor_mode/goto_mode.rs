use macros::BasicEditorMode;

use crate::events::{Key, KeyEvt, KeyMatcher, KeyMods};

use super::{normal_mode::*, EditorCmd, TriggerHandler};

#[derive(BasicEditorMode)]
pub struct GotoMode {
    trigger_handler: TriggerHandler,
}

impl GotoMode {
    pub fn new() -> Self {
        let trigger_handler = TriggerHandler::default()
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Up, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('k', KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('g', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(MOVE_HEAD_FILE_START),
                    EditorCmd::PopMode,
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Down, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('j', KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('e', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(MOVE_HEAD_FILE_END),
                    EditorCmd::PopMode,
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Left, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('h', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(MOVE_HEAD_LINE_START),
                    EditorCmd::PopMode,
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Right, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('l', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(MOVE_HEAD_LINE_END),
                    EditorCmd::PopMode,
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Esc, KeyMods::NONE))]],
                [EditorCmd::PopMode],
            );
        GotoMode { trigger_handler }
    }
}
