use itertools::Itertools;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub highlight: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: Color(0, 0, 0, 255),
            bg: Color(255, 255, 255, 255),
            highlight: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stylizer {
    highlighted_regions: Vec<(usize, usize)>,
    styled_regions: Vec<(usize, usize, Style)>,
    len_chars: usize,
}

impl Stylizer {
    pub fn set_highlighted_regions(&mut self, regions: impl IntoIterator<Item = (usize, usize)>) {
        self.highlighted_regions = regions.into_iter().collect_vec();
    }

    pub fn add_styled_region(&mut self, region: (usize, usize), style: Style) {
        // Only maintain the non-overlapping regions.
        self.styled_regions
            .retain(|(start, end, _)| (*end <= region.0) || (*start >= region.1));
        // Push the new region.
        self.styled_regions.push((region.0, region.1, style));
    }

    fn get_style_at(&self, char_idx: usize) -> Style {
        let mut style = self
            .styled_regions
            .iter()
            .find(|(start, end, _)| *start <= char_idx && *end > char_idx)
            .map(|(_, _, style)| *style)
            .unwrap_or_default();
        style.highlight = self
            .highlighted_regions
            .iter()
            .any(|(start, end)| *start <= char_idx && *end > char_idx);
        style
    }

    pub fn set_len_chars(&mut self, len_chars: usize) {
        self.len_chars = len_chars;
    }

    pub fn compute_regions(&self) -> Vec<(usize, usize, Style)> {
        let mut collected = Vec::new();
        let mut last_idx = 0;
        let mut last_style = self.get_style_at(0);
        for i in 1..=self.len_chars {
            let style_at_idx = self.get_style_at(i);
            if last_style != style_at_idx || i == self.len_chars {
                collected.push((last_idx, i, last_style));
                last_idx = i;
                last_style = style_at_idx;
            }
        }
        collected
    }
}
