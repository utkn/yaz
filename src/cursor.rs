use std::collections::VecDeque;

use itertools::Itertools;
use ropey::Rope;

mod grapheme_iterator;
pub mod movement;
pub use grapheme_iterator::*;

use self::movement::right_grapheme;

#[derive(Clone, Copy, Default, Debug)]
pub struct TextSelection(pub usize, pub Option<usize>);

pub trait SelectionIterator {
    fn collect_merged(self, buf: &Rope) -> Vec<(usize, usize)>;
}

// Blanket implementation for all iterators that yield `TextSelection`s.
impl<T> SelectionIterator for T
where
    T: Iterator<Item = TextSelection>,
{
    /// Merges the overlapping selections and collects them into a vector of pair where
    /// the first element always denotes a character on the left.
    fn collect_merged(self, buf: &Rope) -> Vec<(usize, usize)> {
        let sels = self
            .sorted_by_key(|sel| std::cmp::min(sel.0, sel.1.unwrap_or(sel.0)))
            .map(|sel| {
                let min = std::cmp::min(sel.0, sel.1.unwrap_or(sel.0));
                let mut max = std::cmp::max(sel.0, sel.1.unwrap_or(sel.0));
                max = right_grapheme(max, buf).unwrap_or(max);
                (min, max)
            })
            .collect_vec();
        let mut merged_sels = VecDeque::new();
        for (start, end) in sels {
            if merged_sels.is_empty() {
                merged_sels.push_back((start, end));
                continue;
            }
            let last_added = merged_sels.back().cloned().unwrap();
            if start >= last_added.0 && end <= last_added.1 {
                continue;
            } else if start < last_added.0 && end >= last_added.0 && end <= last_added.1 {
                merged_sels.pop_back();
                merged_sels.push_back((start, last_added.1));
            } else if start >= last_added.0 && start <= last_added.1 && end > last_added.1 {
                merged_sels.pop_back();
                merged_sels.push_back((last_added.0, end));
            } else if start < last_added.0 && end >= last_added.1 {
                merged_sels.pop_back();
                merged_sels.push_back((start, end));
            } else {
                merged_sels.push_back((start, end));
            }
        }
        merged_sels.into()
    }
}
