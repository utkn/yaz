use itertools::Itertools;
use macros::{tx_generator, BasicEditorMode};
use ropey::Rope;

use crate::{
    cursor::movement::*,
    document::{
        primitive_mods::{BufMod, PrimitiveMod, SelectionMod},
        DocumentMap, Transaction,
    },
    events::{Key, KeyCombo, KeyEvt, KeyMatcher, KeyMods},
};

use super::normal_mode::*;
use super::{EditorCmd, TriggerHandler};

fn delete_at_side(
    doc_map: &DocumentMap,
    side_fn: fn(usize, &Rope) -> Option<usize>,
) -> Option<Transaction> {
    let sels = doc_map.get_curr_doc()?.selections.iter();
    let buf = &doc_map.get_curr_doc()?.inner_buf;
    // (idx => # deleted chars left of idx)
    let mut modification = Transaction::new();
    sels.sorted_by_key(|(_, sel)| sel.0)
        .for_each(|(sel_id, sel)| {
            // delete the grapheme at the side
            side_fn(sel.0, buf)
                .map(|side_g_idx| {
                    let mut start = std::cmp::min(side_g_idx, sel.0);
                    let mut end = std::cmp::max(side_g_idx, sel.0);
                    start = modification
                        .map_char_idx(&doc_map.curr_doc_id(), &start)
                        .unwrap_or(0);
                    end = modification
                        .map_char_idx(&doc_map.curr_doc_id(), &end)
                        .unwrap_or(start);
                    modification.append_mods([
                        PrimitiveMod::Text(doc_map.curr_doc_id(), BufMod::DelRange(start, end)),
                        PrimitiveMod::Sel(
                            doc_map.curr_doc_id(),
                            *sel_id,
                            SelectionMod::SetHead(start),
                        ),
                    ]);
                })
                .unwrap_or_default()
        });
    Some(modification)
}

#[tx_generator]
fn insert_key(trigger: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    // Collect the text to insert from the trigger.
    let text_to_insert = trigger.extract_text();
    if text_to_insert.is_empty() {
        return None;
    }
    let mut modification = Transaction::new();
    let text_num_chars = text_to_insert.chars().count();
    doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .sorted_by_key(|(_, sel)| sel.0)
        .for_each(|(sel_id, sel)| {
            let insert_index = modification
                .map_char_idx(&doc_map.curr_doc_id(), &sel.0)
                .unwrap_or(0);
            // Move the head to the right of the inserted text
            let new_head = insert_index + text_num_chars;
            modification.append_mods([
                PrimitiveMod::Text(
                    doc_map.curr_doc_id(),
                    BufMod::InsText(insert_index, text_to_insert.clone()),
                ),
                PrimitiveMod::Sel(
                    doc_map.curr_doc_id(),
                    *sel_id,
                    SelectionMod::SetHead(new_head),
                ),
            ]);
        });
    Some(modification)
}

#[tx_generator]
fn delete_left(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    delete_at_side(doc_map, left_grapheme)
}

#[tx_generator]
fn delete_right(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    delete_at_side(doc_map, right_grapheme)
}

#[derive(BasicEditorMode)]
pub struct InsertMode {
    trigger_handler: TriggerHandler,
}

impl InsertMode {
    pub fn new() -> Self {
        let trigger_handler = TriggerHandler::default()
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('z', KeyMods::CTRL))]],
                [EditorCmd::UndoCurrDocument],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('y', KeyMods::CTRL))]],
                [EditorCmd::RedoCurrDocument],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Left, KeyMods::NONE))]],
                [EditorCmd::Transaction(MOVE_HEAD_LEFT)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Right, KeyMods::NONE))]],
                [EditorCmd::Transaction(MOVE_HEAD_RIGHT)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Up, KeyMods::NONE))]],
                [EditorCmd::Transaction(MOVE_HEAD_UP)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Down, KeyMods::NONE))]],
                [EditorCmd::Transaction(MOVE_HEAD_DOWN)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(
                    Key::Backspace,
                    KeyMods::NONE,
                ))]],
                [EditorCmd::Transaction(DELETE_LEFT)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Del, KeyMods::NONE))]],
                [EditorCmd::Transaction(DELETE_RIGHT)],
            )
            .with(
                [[
                    KeyMatcher::AnyChar(KeyMods::NONE),
                    KeyMatcher::Exact(KeyEvt::Key(Key::Tab, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Key(Key::Enter, KeyMods::NONE)),
                ]],
                [EditorCmd::Transaction(INSERT_KEY)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Esc, KeyMods::NONE))]],
                [EditorCmd::PopMode],
            );
        InsertMode { trigger_handler }
    }
}
