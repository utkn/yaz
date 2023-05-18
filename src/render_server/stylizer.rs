use std::collections::BTreeMap;

use itertools::Itertools;

use crate::document::DocumentView;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct RGBAColor(pub u8, pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleAttr {
    Fg(RGBAColor),
    Bg(RGBAColor),
    Highlight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleAttrMod {
    AddAttr(StyleAttr),
    RemAttr(StyleAttr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ConcreteStyle {
    pub fg: Option<RGBAColor>,
    pub bg: Option<RGBAColor>,
    pub highlight: bool,
}

impl ConcreteStyle {
    /// Constructs a new style from the given list of attributes.
    fn new<T: IntoIterator<Item = StyleAttr>>(attr_set: T) -> Self {
        let mut style: Self = Default::default();
        attr_set.into_iter().for_each(|attr| match attr {
            StyleAttr::Fg(color) => style.fg = Some(color),
            StyleAttr::Bg(color) => style.bg = Some(color),
            StyleAttr::Highlight => style.highlight = true,
        });
        style
    }
}

impl IntoIterator for ConcreteStyle {
    type Item = StyleAttr;

    type IntoIter = <Vec<StyleAttr> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let mut attrs = Vec::new();
        if let Some(color) = self.fg {
            attrs.push(StyleAttr::Fg(color));
        }
        if let Some(color) = self.bg {
            attrs.push(StyleAttr::Bg(color));
        }
        if self.highlight {
            attrs.push(StyleAttr::Highlight);
        }
        attrs.into_iter()
    }
}

fn extend_attrs(attrs: &mut Vec<StyleAttr>, mods: &Vec<StyleAttrMod>) {
    mods.iter().fold(attrs, |v, attr_mod| {
        match attr_mod {
            StyleAttrMod::AddAttr(attr) => v.push(*attr),
            StyleAttrMod::RemAttr(attr) => {
                v.iter()
                    .position(|a| a == attr)
                    .map(|idx| v.swap_remove(idx));
            }
        };
        v
    });
}

#[derive(Clone, Debug, Default)]
pub struct Stylizer {
    stylization_points: BTreeMap<usize, Vec<StyleAttrMod>>,
}

impl Stylizer {
    fn add_attribute(&mut self, point: usize, style_attr: StyleAttr) {
        self.stylization_points
            .entry(point)
            .or_default()
            .push(StyleAttrMod::AddAttr(style_attr));
    }

    fn remove_attribute(&mut self, point: usize, style_attr: StyleAttr) {
        self.stylization_points
            .entry(point)
            .or_default()
            .push(StyleAttrMod::RemAttr(style_attr));
    }

    pub fn layer_region_style(
        &mut self,
        start: usize,
        end: usize,
        attrs: impl IntoIterator<Item = StyleAttr>,
    ) {
        attrs.into_iter().for_each(|attr| {
            self.add_attribute(start, attr);
            self.remove_attribute(end, attr);
        })
    }

    pub fn reset(&mut self) {
        self.stylization_points.clear();
    }

    pub fn compute_regions(&self, max_chars: usize) -> Vec<(usize, usize, ConcreteStyle)> {
        self.stylization_points
            .iter()
            .tuple_windows()
            .scan(Vec::new(), |curr_attrs, (start, end)| {
                // extend by the start style
                extend_attrs(curr_attrs, start.1);
                // output the range
                Some((*start.0, *end.0, curr_attrs.clone()))
            })
            .map(|(start, end, attrs)| (start, end, ConcreteStyle::new(attrs)))
            // .take(max_chars)
            .collect_vec()
    }
}

mod tests {
    use super::*;

    #[test]
    fn stylizer_simple() {
        let mut stylizer = Stylizer::default();
        let color = RGBAColor(0, 0, 0, 0);
        let style_1 = ConcreteStyle::new([StyleAttr::Highlight, StyleAttr::Fg(color)]);
        let style_2 = ConcreteStyle::new([StyleAttr::Highlight]);
        stylizer.layer_region_style(0, 10, style_1);
        stylizer.layer_region_style(0, 20, style_2);
        let regions = stylizer.compute_regions(100);
        assert_eq!(regions, vec![(0, 10, style_1), (10, 20, style_2)]);
    }
}
