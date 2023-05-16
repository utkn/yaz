use itertools::Itertools;
use macros::{tx_generator, BasicEditorMode};
use ropey::Rope;

use crate::{
    cursor::{movement::*, SelectionIterator, TextSelection},
    document::{
        primitive_mods::{BufMod, DocMapMod, PrimitiveMod, SelectionMod},
        DocumentMap, Transaction,
    },
    events::{Key, KeyCombo, KeyEvt, KeyMatcher, KeyMods},
};

use super::*;

fn move_all_heads(
    movement_fn: impl Fn(usize, &Rope) -> Option<usize>,
    doc_map: &DocumentMap,
) -> Option<Transaction> {
    let buf = &doc_map.get_curr_doc()?.inner_buf;
    Some(
        Transaction::new().with_mods(doc_map.get_curr_doc()?.selections.iter().map(
            |(sel_id, sel)| {
                let new_head = movement_fn(sel.0, buf).unwrap_or(sel.0);
                PrimitiveMod::Sel(
                    doc_map.curr_doc_id(),
                    *sel_id,
                    SelectionMod::SetHead(new_head),
                )
            },
        )),
    )
}

#[tx_generator]
pub fn move_head_left(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(left_grapheme, doc_map)
}

#[tx_generator]
pub fn move_head_right(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(right_grapheme, doc_map)
}

#[tx_generator]
pub fn move_head_up(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(upper_grapheme_or_start, doc_map)
}

#[tx_generator]
pub fn move_head_down(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(lower_grapheme_or_end, doc_map)
}

#[tx_generator]
pub fn move_head_line_start(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(line_start, doc_map)
}

#[tx_generator]
pub fn move_head_line_end(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(line_end, doc_map)
}

#[tx_generator]
pub fn move_head_file_start(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(file_start, doc_map)
}

#[tx_generator]
pub fn move_head_file_end(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(file_end, doc_map)
}

#[tx_generator]
pub fn move_head_right_word_start(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(right_word_start, doc_map)
}

#[tx_generator]
pub fn move_head_right_word_end(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(right_word_end, doc_map)
}

#[tx_generator]
pub fn move_head_left_word_start(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(left_word_start, doc_map)
}

#[tx_generator]
pub fn move_head_left_word_end(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    move_all_heads(left_word_end, doc_map)
}

#[tx_generator]
pub fn move_head_right_occurrence(tr: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let target = match tr.0.iter().nth(1)? {
        KeyEvt::Char(c, _) => Some(c),
        _ => None,
    }?
    .to_string();
    move_all_heads(|idx, buf| right_occurrence(idx, &target, buf), doc_map)
}

#[tx_generator]
pub fn move_head_left_occurrence(tr: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let target = match tr.0.iter().nth(1)? {
        KeyEvt::Char(c, _) => Some(c),
        _ => None,
    }?
    .to_string();
    move_all_heads(|idx, buf| left_occurrence(idx, &target, buf), doc_map)
}

#[tx_generator]
fn select_this_or_next_line(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let buf = &doc_map.get_curr_doc()?.inner_buf;
    Some(
        Transaction::new().with_mods(
            doc_map
                .get_curr_doc()?
                .selections
                .iter()
                .flat_map(|(sel_id, sel)| {
                    let min = std::cmp::min(sel.0, sel.1.unwrap_or(sel.0));
                    let max = std::cmp::max(sel.0, sel.1.unwrap_or(sel.0));
                    let curr_line_start = line_start(sel.0, buf)?;
                    let curr_line_end = line_end(sel.0, buf)?;
                    if curr_line_start == min && curr_line_end == max {
                        let next_line_start_idx = next_line_start(max, buf)?;
                        let next_line_end_idx = line_end(next_line_start_idx, buf)?;
                        Some(vec![
                            PrimitiveMod::Sel(
                                doc_map.curr_doc_id(),
                                *sel_id,
                                SelectionMod::SetHead(next_line_end_idx),
                            ),
                            PrimitiveMod::Sel(
                                doc_map.curr_doc_id(),
                                *sel_id,
                                SelectionMod::SetTail(Some(next_line_start_idx)),
                            ),
                        ])
                    } else {
                        Some(vec![
                            PrimitiveMod::Sel(
                                doc_map.curr_doc_id(),
                                *sel_id,
                                SelectionMod::SetHead(curr_line_end),
                            ),
                            PrimitiveMod::Sel(
                                doc_map.curr_doc_id(),
                                *sel_id,
                                SelectionMod::SetTail(Some(curr_line_start)),
                            ),
                        ])
                    }
                })
                .flatten(),
        ),
    )
}

#[tx_generator]
fn delete_sels(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let merged_sels = doc_map
        .get_curr_doc()?
        .selections
        .values()
        .cloned()
        .collect_merged(&doc_map.get_curr_doc()?.inner_buf);
    // Delete the selections while maintaining the selection positions.
    let mut modification = Transaction::new();
    merged_sels.iter().for_each(|(start, end)| {
        let start = modification
            .map_char_idx(&doc_map.curr_doc_id(), start)
            .unwrap_or(0);
        let end = modification
            .map_char_idx(&doc_map.curr_doc_id(), end)
            .unwrap_or(start);
        modification.append_mod(PrimitiveMod::Text(
            doc_map.curr_doc_id(),
            BufMod::DelRange(start, end),
        ));
    });
    // Pseudo-collapse all the selections after deletion.
    doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .for_each(|(sel_id, sel)| {
            let min = std::cmp::min(sel.0, sel.1.unwrap_or(sel.0));
            let new_head_idx = modification
                .map_char_idx(&doc_map.curr_doc_id(), &min)
                .unwrap_or(0);
            modification.append_mods([
                PrimitiveMod::Sel(
                    doc_map.curr_doc_id(),
                    *sel_id,
                    SelectionMod::SetHead(new_head_idx),
                ),
                PrimitiveMod::Sel(
                    doc_map.curr_doc_id(),
                    *sel_id,
                    SelectionMod::SetTail(Some(new_head_idx)),
                ),
            ]);
        });
    Some(modification)
}

#[tx_generator]
fn insert_newline(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let sel_heads = doc_map
        .get_curr_doc()?
        .selections
        .values()
        .map(|sel| sel.0)
        .collect_vec();
    let mods = sel_heads
        .iter()
        .map(|head| {
            PrimitiveMod::Text(
                doc_map.curr_doc_id(),
                BufMod::InsText(*head, String::from('\n')),
            )
        })
        .collect_vec();
    if mods.is_empty() {
        return None;
    }
    Some(Transaction::new().with_mods(mods))
}

#[tx_generator]
fn add_sel_down(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let max_sel_head = doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .max_by_key(|(sel_id, _)| *sel_id)
        .map(|(_, sel)| sel.0)
        .unwrap_or(0);
    let new_sel_head = lower_grapheme_or_end(max_sel_head, &doc_map.get_curr_doc()?.inner_buf)?;
    let new_sel_id = doc_map
        .get_curr_doc()?
        .selections
        .keys()
        .max()
        .map(|max| max + 1)
        .unwrap_or(0);
    let p_mod = PrimitiveMod::Editor(DocMapMod::CreateSel(
        doc_map.curr_doc_id(),
        new_sel_id,
        TextSelection(new_sel_head, None),
    ));
    Some(Transaction::new().with_mod(p_mod))
}

#[tx_generator]
fn collapse_sels(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let mods = doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .map(|(sel_id, _)| {
            PrimitiveMod::Sel(doc_map.curr_doc_id(), *sel_id, SelectionMod::SetTail(None))
        })
        .collect_vec();
    Some(Transaction::new().with_mods(mods))
}

#[tx_generator]
fn collapse_sels_force(tr: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    collapse_sels(tr, doc_map)
}

#[tx_generator]
fn reset_sels(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let min_sel_id = doc_map.get_curr_doc()?.selections.keys().min()?;
    let mods = doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .filter(|(sel_id, _)| *sel_id != min_sel_id)
        .map(|(sel_id, _)| {
            PrimitiveMod::Editor(DocMapMod::DeleteSel(doc_map.curr_doc_id(), *sel_id))
        })
        .collect_vec();
    Some(Transaction::new().with_mods(mods))
}

#[tx_generator]
fn drop_tail(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let mods = doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .filter(|(_, sel)| sel.1.is_none())
        .map(|(sel_id, sel)| {
            PrimitiveMod::Sel(
                doc_map.curr_doc_id(),
                *sel_id,
                SelectionMod::SetTail(Some(sel.0)),
            )
        })
        .collect_vec();
    Some(Transaction::new().with_mods(mods))
}

#[tx_generator]
fn collapse_or_reset_sels(kc: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let tails_exist = doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .find(|(_, sel)| sel.1.is_some())
        .map_or(false, |_| true);
    if tails_exist {
        collapse_sels(kc, doc_map)
    } else {
        reset_sels(kc, doc_map)
    }
}

#[tx_generator]
fn swap_head_tail(_: &KeyCombo, doc_map: &DocumentMap) -> Option<Transaction> {
    let mods = doc_map
        .get_curr_doc()?
        .selections
        .iter()
        .filter(|(_, sel)| sel.1.is_some())
        .flat_map(|(sel_id, sel)| {
            [
                PrimitiveMod::Sel(
                    doc_map.curr_doc_id(),
                    *sel_id,
                    SelectionMod::SetTail(Some(sel.0)),
                ),
                PrimitiveMod::Sel(
                    doc_map.curr_doc_id(),
                    *sel_id,
                    SelectionMod::SetHead(sel.1.unwrap()),
                ),
            ]
        })
        .collect_vec();
    Some(Transaction::new().with_mods(mods))
}

#[derive(BasicEditorMode)]
pub struct NormalMode {
    trigger_handler: TriggerHandler,
}

impl NormalMode {
    pub fn new() -> Self {
        let trigger_handler = TriggerHandler::default()
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('u', KeyMods::NONE))]],
                [EditorCmd::UndoCurrDocument],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('U', KeyMods::NONE))]],
                [EditorCmd::RedoCurrDocument],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Left, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('h', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_LEFT),
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Right, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('l', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT),
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Up, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('k', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_UP),
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Key(Key::Down, KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('j', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_DOWN),
                ],
            )
            .with(
                [
                    [KeyMatcher::Exact(KeyEvt::Char('f', KeyMods::NONE))],
                    [KeyMatcher::AnyChar(KeyMods::NONE)],
                ],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT),
                    EditorCmd::Transaction(DROP_TAIL),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT_OCCURRENCE),
                ],
            )
            .with(
                [
                    [KeyMatcher::Exact(KeyEvt::Char('F', KeyMods::NONE))],
                    [KeyMatcher::AnyChar(KeyMods::NONE)],
                ],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_LEFT),
                    EditorCmd::Transaction(DROP_TAIL),
                    EditorCmd::Transaction(MOVE_HEAD_LEFT_OCCURRENCE),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('w', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT_WORD_START),
                    EditorCmd::Transaction(DROP_TAIL),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT_WORD_END),
                ],
            )
            .with(
                [[
                    KeyMatcher::Exact(KeyEvt::Char('W', KeyMods::NONE)),
                    KeyMatcher::Exact(KeyEvt::Char('b', KeyMods::NONE)),
                ]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_LEFT_WORD_START),
                    EditorCmd::Transaction(DROP_TAIL),
                    EditorCmd::Transaction(MOVE_HEAD_LEFT_WORD_END),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('%', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS_FORCE),
                    EditorCmd::Transaction(MOVE_HEAD_FILE_START),
                    EditorCmd::Transaction(DROP_TAIL),
                    EditorCmd::Transaction(MOVE_HEAD_FILE_END),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char(';', KeyMods::NONE))]],
                [EditorCmd::Transaction(SWAP_HEAD_TAIL)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char(':', KeyMods::NONE))]],
                [EditorCmd::PushMode(CommandMode::id())],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('x', KeyMods::NONE))]],
                [EditorCmd::Transaction(SELECT_THIS_OR_NEXT_LINE)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('d', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(DELETE_SELS),
                    EditorCmd::Transaction(COLLAPSE_SELS),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('c', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(DELETE_SELS),
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::PushMode(InsertMode::id()),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('C', KeyMods::NONE))]],
                [EditorCmd::Transaction(ADD_SEL_DOWN)],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('i', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::PushMode(InsertMode::id()),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('a', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT),
                    EditorCmd::PushMode(InsertMode::id()),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('O', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_LINE_START),
                    EditorCmd::Transaction(INSERT_NEWLINE),
                    EditorCmd::PushMode(InsertMode::id()),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('o', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(COLLAPSE_SELS),
                    EditorCmd::Transaction(MOVE_HEAD_LINE_END),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT),
                    EditorCmd::Transaction(INSERT_NEWLINE),
                    EditorCmd::Transaction(MOVE_HEAD_RIGHT),
                    EditorCmd::PushMode(InsertMode::id()),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('v', KeyMods::NONE))]],
                [
                    EditorCmd::Transaction(DROP_TAIL),
                    EditorCmd::PushMode(SelectionMode::id()),
                ],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Char('g', KeyMods::NONE))]],
                [EditorCmd::PushMode(GotoMode::id())],
            )
            .with(
                [[KeyMatcher::Exact(KeyEvt::Key(Key::Esc, KeyMods::NONE))]],
                [EditorCmd::Transaction(COLLAPSE_OR_RESET_SELS)],
            );
        NormalMode { trigger_handler }
    }
}
