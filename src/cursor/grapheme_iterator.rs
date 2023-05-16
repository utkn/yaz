use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

// Should be set to max char length of a grapheme.
const LOOKAHEAD_WIDTH: usize = 12;

pub trait GraphemeIterable<'a> {
    fn graphemes(&'a self, init_char_idx: usize) -> GraphemeIterator<'a>;
    fn grapheme_starting_at(&self, start_idx: usize) -> Option<String>;
    fn grapheme_ending_at(&self, end_idx: usize) -> Option<String>;
}

impl<'a> GraphemeIterable<'a> for Rope {
    /// Creates an iterator that iterates through the graphemes in this buffer.
    fn graphemes(&'a self, init_char_idx: usize) -> GraphemeIterator<'a> {
        GraphemeIterator::new(init_char_idx, self)
    }

    /// Returns the grapheme starting at the given index (inclusive).
    fn grapheme_starting_at(&self, idx: usize) -> Option<String> {
        let g = self
            .get_slice(idx..(idx + LOOKAHEAD_WIDTH).clamp(0, self.len_chars()))?
            .to_string()
            .graphemes(true)
            .next()?
            .to_string();
        Some(g)
    }

    /// Returns the grapheme ending at the given index (exclusive).
    fn grapheme_ending_at(&self, idx: usize) -> Option<String> {
        let g = self
            .get_slice(idx.saturating_sub(LOOKAHEAD_WIDTH)..idx)?
            .to_string()
            .graphemes(true)
            .rev()
            .next()?
            .to_string();
        Some(g)
    }
}

pub struct GraphemeIterator<'a> {
    next_range: (usize, usize), // exclusive grapheme range
    reverse: bool,
    buf: &'a Rope,
}

impl<'a> GraphemeIterator<'a> {
    /// Creates a new grapheme iterator that yields graphemes starting from the `init_char_id`
    /// on the given buffer `buf`.
    pub fn new(init_char_idx: usize, buf: &'a Rope) -> Self {
        let first_g_offset = buf
            .grapheme_starting_at(init_char_idx)
            .map(|g| g.chars().count())
            .unwrap_or(0);
        GraphemeIterator {
            next_range: (init_char_idx, init_char_idx + first_g_offset),
            reverse: false,
            buf,
        }
    }

    /// Returns the current character index, clamped between 0 and buffer length.
    /// This means when the iterator is on BOF or EOF, it will return 0 or buffer length respectively.
    pub fn curr_idx(&self) -> usize {
        self.next_range.0
    }

    /// Returns a new grapheme iterator with reversed direction.
    /// If the iterator is on top of EOF/BOF, it will return an empty string first.
    pub fn rev(mut self) -> Self {
        self.reverse = !self.reverse;
        self
    }

    pub fn at_bof(&self) -> bool {
        self.next_range.1 == 0
    }

    pub fn at_eof(&self) -> bool {
        self.next_range.0 == self.buf.len_chars()
    }

    fn next_grapheme(&mut self) -> Option<String> {
        if self.at_eof() {
            return None;
        } else if self.at_bof() {
            // reset to the first grapheme
            let first_g_end = self
                .buf
                .grapheme_starting_at(0)
                .map(|g| g.chars().count())
                .unwrap_or(0);
            self.next_range = (0, first_g_end);
            // indicates EOF
            return Some(String::new());
        }
        let g = self
            .buf
            .get_slice(self.next_range.0..self.next_range.1)?
            .to_string();
        let next_g = self
            .buf
            .grapheme_starting_at(self.next_range.1)
            .unwrap_or(String::new()); // EOF
        let next_start = self.next_range.1;
        let next_end = next_start + next_g.chars().count();
        self.next_range = (next_start, next_end);
        Some(g)
    }

    fn prev_grapheme(&mut self) -> Option<String> {
        if self.at_bof() {
            return None;
        } else if self.at_eof() {
            // reset to the last grapheme
            let last_g_width = self
                .buf
                .grapheme_ending_at(self.buf.len_chars())
                .map(|g| g.chars().count())
                .unwrap_or(0);
            self.next_range = (
                self.buf.len_chars().saturating_sub(last_g_width),
                self.buf.len_chars(),
            );
            // indicates BOF
            return Some(String::new());
        }
        let g = self
            .buf
            .get_slice(self.next_range.0..self.next_range.1)?
            .to_string();
        let prev_g = self
            .buf
            .grapheme_ending_at(self.next_range.0)
            .unwrap_or(String::new()); // BOF
        let prev_end = self.next_range.0;
        let prev_start = prev_end.saturating_sub(prev_g.chars().count());
        self.next_range = (prev_start, prev_end);
        Some(g)
    }

    /// Keeps moving the iterator and collecting the graphemes until the collected string
    /// passes the given predicate or we reach EOF/BOF. If the traversal was terminated by the
    /// given predicate, it moves the iterator back into the terminating position.
    pub fn stop_at<F>(mut self, pred: F) -> Self
    where
        F: Fn(&str) -> bool,
    {
        let mut collected_str = String::new();
        let mut terminated_by_pred = false;
        while let Some(g) = self.next() {
            collected_str.push_str(&g);
            if pred(&collected_str) {
                terminated_by_pred = true;
                break;
            }
        }
        if terminated_by_pred {
            self = self.rev();
            self.next();
            self.rev() // will yield the terminating grapheme
        } else {
            self // terminating "grapheme" is EOF, calling next should yield None
        }
    }

    /// Keeps moving the iterator and collecting the graphemes until the collected string
    /// passes the given predicate or we reach BOF/EOF. Moves the iterator back into the
    /// last position that failed the predicate or the first/last grapheme.
    pub fn stop_before<F>(mut self, pred: F) -> Self
    where
        F: Fn(&str) -> bool,
    {
        self = self.stop_at(pred);
        // If we are at BOF/EOF, we want to return an iterator that yields the first/last grapheme.
        if self.at_bof() || self.at_eof() {
            self = self.rev();
            self.next(); // yields the BOF/EOF indication
            return self.rev(); // will yield the first/last grapheme
        }
        self = self.rev();
        self.next();
        self.rev() // will yield the last valid grapheme
    }
}

impl<'a> Iterator for GraphemeIterator<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let out = if self.reverse {
            self.prev_grapheme()
        } else {
            self.next_grapheme()
        };
        out
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_move_forwards_short() {
        let short_test_string = Rope::from_str("aşcd🧑‍🔬e");
        let mut it = GraphemeIterator::new(0, &short_test_string);
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), None);

        let mut it = GraphemeIterator::new(1, &short_test_string);
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_move_backwards_short() {
        let short_test_string = Rope::from_str("aşcd🧑‍🔬ef");
        let mut it =
            GraphemeIterator::new(short_test_string.len_chars() - 1, &short_test_string).rev();
        assert_eq!(it.next(), Some("f".into()));
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), None);

        let mut it =
            GraphemeIterator::new(short_test_string.len_chars() - 2, &short_test_string).rev();
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_move_forwards_long() {
        let s = String::from("abcdef🧑‍🔬ghş").repeat(10000);
        let rope = Rope::from_str(&s);
        let mut it_expected = s.graphemes(true);
        let num_graphemes = s.graphemes(true).count();
        let mut it = GraphemeIterator::new(0, &rope);
        for _i in 1..(num_graphemes + 1) {
            assert_eq!(it.next(), it_expected.next().map(|s| s.to_string()));
        }
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_move_backwards_long() {
        let s = String::from("abcdef🧑‍🔬ghş").repeat(10000);
        let rope = Rope::from_str(&s);
        let mut it_expected = s.graphemes(true).rev();
        let num_graphemes = s.graphemes(true).count();
        let mut it = GraphemeIterator::new(rope.len_chars() - 1, &rope).rev();
        for _i in 1..(num_graphemes + 1) {
            assert_eq!(it.next(), it_expected.next().map(|s| s.to_string()));
        }
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_inverse_ends_short() {
        let short_test_string = Rope::from_str("aşcd🧑‍🔬e");
        let mut it = GraphemeIterator::new(0, &short_test_string);
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), None);
        it = it.rev();
        assert_eq!(it.next(), Some("".into()));
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), None);
        it = it.rev();
        assert_eq!(it.next(), Some("".into()));
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), Some("ş".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), Some("🧑‍🔬".into()));
        assert_eq!(it.next(), Some("e".into()));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_until() {
        let short_test_string = Rope::from_str("aaaaabcd");
        let mut it = GraphemeIterator::new(0, &short_test_string);
        it = it.stop_at(|s| s.contains('b'));
        assert_eq!(it.curr_idx(), 5);
        assert_eq!(it.next(), Some("b".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), None);
        let mut it = GraphemeIterator::new(0, &short_test_string);
        it = it.stop_before(|s| s.contains('b'));
        assert_eq!(it.curr_idx(), 4);
        assert_eq!(it.next(), Some("a".into()));
        assert_eq!(it.next(), Some("b".into()));
        assert_eq!(it.next(), Some("c".into()));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_until_ends() {
        let short_test_string = Rope::from_str("aaaaabcd");
        let mut it = GraphemeIterator::new(0, &short_test_string);
        it = it.stop_at(|s| s.contains('e'));
        assert!(it.at_eof());
        assert_eq!(it.next(), None);
        let mut it = GraphemeIterator::new(0, &short_test_string);
        it = it.stop_before(|s| s.contains('e'));
        assert_eq!(it.next(), Some("d".into()));
        assert_eq!(it.next(), None);
    }
}
