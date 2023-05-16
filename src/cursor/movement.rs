use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::GraphemeIterable;

pub fn right_grapheme(char_idx: usize, buf: &Rope) -> Option<usize> {
    let mut it = buf.graphemes(char_idx);
    it.next()?;
    Some(it.curr_idx())
}

pub fn left_grapheme(char_idx: usize, buf: &Rope) -> Option<usize> {
    // Handle EOF
    if char_idx == buf.len_chars() {
        return Some(char_idx.saturating_sub(1));
    }
    let mut it = buf.graphemes(char_idx).rev();
    it.next()?;
    Some(it.curr_idx())
}

fn jump_to_line(
    curr_char_idx: usize,
    curr_line_idx: usize,
    target_line_idx: usize,
    buf: &Rope,
) -> Option<usize> {
    let tab_replacement = "    ";
    let curr_line_start = buf.try_line_to_char(curr_line_idx).ok()?;
    let target_line_start = buf.try_line_to_char(target_line_idx).ok()?;
    let mut target_line_end =
        (target_line_start + buf.get_line(target_line_idx)?.len_chars()).saturating_sub(1);
    let target_width = buf
        .get_slice(curr_line_start..curr_char_idx)?
        .to_string()
        .replace('\t', tab_replacement)
        .replace('\n', " ")
        .width();
    let mut target_line = buf.get_line(target_line_idx)?.to_string();
    if target_line_idx == buf.len_lines().saturating_sub(1) {
        target_line.push_str(" ");
        target_line_end += 1;
    }
    let mut target_line_char_offset = 0;
    let mut curr_width = 0;
    for g in target_line.graphemes(true) {
        let next_width = if g == "\t" {
            curr_width + tab_replacement.width()
        } else if g == "\n" {
            curr_width + 1
        } else {
            curr_width + g.width()
        };
        if next_width > target_width {
            break;
        }
        curr_width = next_width;
        target_line_char_offset += g.chars().count();
    }
    Some((target_line_start + target_line_char_offset).clamp(target_line_start, target_line_end))
}

pub fn upper_grapheme_or_start(char_idx: usize, buf: &Rope) -> Option<usize> {
    let curr_line_idx = buf.try_char_to_line(char_idx).ok()?;
    if curr_line_idx == 0 {
        return Some(0);
    }
    jump_to_line(char_idx, curr_line_idx, curr_line_idx - 1, buf)
}

pub fn lower_grapheme_or_end(char_idx: usize, buf: &Rope) -> Option<usize> {
    let curr_line_idx = buf.try_char_to_line(char_idx).ok()?;
    if curr_line_idx == buf.len_lines().saturating_sub(1) {
        return Some(buf.len_chars());
    }
    jump_to_line(char_idx, curr_line_idx, curr_line_idx + 1, buf)
}

pub fn file_start(_: usize, _: &Rope) -> Option<usize> {
    Some(0)
}

pub fn file_end(_: usize, buf: &Rope) -> Option<usize> {
    Some(buf.len_chars())
}

pub fn line_start(char_idx: usize, buf: &Rope) -> Option<usize> {
    let line_idx = buf.try_char_to_line(char_idx).ok()?;
    buf.try_line_to_char(line_idx).ok()
}

pub fn line_end(char_idx: usize, buf: &Rope) -> Option<usize> {
    let line_idx = buf.try_char_to_line(char_idx).ok()?;
    let line_start = line_start(char_idx, buf)?;
    Some(line_start + buf.get_line(line_idx)?.len_chars().saturating_sub(1))
}

pub fn next_line_start(char_idx: usize, buf: &Rope) -> Option<usize> {
    let line_idx = buf.try_char_to_line(char_idx).ok()?;
    if line_idx == buf.len_lines().saturating_sub(1) {
        return None;
    }
    buf.try_line_to_char(line_idx + 1).ok()
}

pub fn right_occurrence(char_idx: usize, target: &str, buf: &Rope) -> Option<usize> {
    if char_idx >= buf.len_chars().saturating_sub(1) {
        return None;
    }
    let next_occurrence = buf
        .graphemes(char_idx)
        .stop_at(|s| s.ends_with(target))
        .curr_idx();
    Some(next_occurrence)
}

pub fn left_occurrence(char_idx: usize, target: &str, buf: &Rope) -> Option<usize> {
    if char_idx == 0 {
        return None;
    }
    let next_occurrence = buf
        .graphemes(char_idx)
        .rev()
        .stop_at(|s| s.ends_with(target))
        .curr_idx();
    Some(next_occurrence)
}

pub fn right_word_start(char_idx: usize, buf: &Rope) -> Option<usize> {
    if char_idx == buf.len_chars() {
        return None;
    }
    let mut it = buf.graphemes(char_idx);
    // Skip current word if we are at word end.
    if buf.graphemes(char_idx).nth(1)?.trim().is_empty() {
        it = it.stop_at(|s| s.trim() != s);
    }
    // Skip the delimeter
    it = it.stop_at(|s| !s.trim().is_empty());
    let idx = it.curr_idx();
    return Some(idx);
}

pub fn right_word_end(char_idx: usize, buf: &Rope) -> Option<usize> {
    if char_idx == buf.len_chars() {
        return None;
    }
    let mut it = buf.graphemes(char_idx);
    // Skip current word.
    it = it.stop_before(|s| s.trim() != s);
    let idx = it.curr_idx();
    return Some(idx);
}

pub fn left_word_start(char_idx: usize, buf: &Rope) -> Option<usize> {
    if char_idx == 0 {
        return None;
    }
    let mut it = buf.graphemes(char_idx).rev();
    // Skip current word if we are at word end.
    if buf.graphemes(char_idx).rev().nth(1)?.trim().is_empty() {
        it = it.stop_at(|s| s.trim() != s);
    }
    // Skip the delimeter
    it = it.stop_at(|s| !s.trim().is_empty());
    let idx = it.curr_idx();
    return Some(idx);
}

pub fn left_word_end(char_idx: usize, buf: &Rope) -> Option<usize> {
    if char_idx == 0 {
        return None;
    }
    let mut it = buf.graphemes(char_idx).rev();
    // Skip current word.
    it = it.stop_before(|s| s.trim() != s);
    let idx = it.curr_idx();
    return Some(idx);
}
