use bitflags::bitflags;
use itertools::Itertools;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct KeyMods: usize {
        const NONE = 0;
        const CTRL = 1;
        const ALT = 2;
        const SHIFT = 4;
    }
}
/// A non-character key on the keyboard
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Key {
    Enter,
    Tab,
    Backspace,
    Esc,
    Left,
    Right,
    Up,
    Down,
    Ins,
    Del,
    Home,
    End,
    PageUp,
    PageDown,
    PauseBreak,
    NumpadCenter,
    F0,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyEvt {
    Char(char, KeyMods),
    Key(Key, KeyMods),
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct KeyCombo(pub Vec<KeyEvt>);

impl FromIterator<KeyEvt> for KeyCombo {
    fn from_iter<T: IntoIterator<Item = KeyEvt>>(iter: T) -> Self {
        Self(iter.into_iter().collect_vec())
    }
}

impl IntoIterator for KeyCombo {
    type Item = KeyEvt;

    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl KeyCombo {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn starts_with(&self, other: impl IntoIterator<Item = KeyEvt>) -> bool {
        self.0.starts_with(&other.into_iter().collect_vec())
    }

    pub fn ends_with(&self, other: impl IntoIterator<Item = KeyEvt>) -> bool {
        self.0.ends_with(&other.into_iter().collect_vec())
    }

    pub fn pop_first(&mut self) -> Option<KeyEvt> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    pub fn first(&self) -> Option<&KeyEvt> {
        self.0.first()
    }

    pub fn first_matches<F: FnOnce(&KeyEvt) -> bool>(&self, pred: F) -> bool {
        self.0.first().map(|k| pred(k)).unwrap_or(false)
    }

    pub fn pop_first_if<F: FnOnce(&KeyEvt) -> bool>(&mut self, pred: F) -> Option<KeyEvt> {
        if self.first_matches(pred) {
            self.pop_first()
        } else {
            None
        }
    }

    pub fn add(&mut self, evt: KeyEvt) {
        self.0.push(evt);
    }

    pub fn reset(&mut self) -> KeyCombo {
        let mut ret = KeyCombo(Default::default());
        ret.0.append(&mut self.0);
        ret
    }

    pub fn extract_text(&self) -> String {
        self.0
            .iter()
            .flat_map(|key_evt| match key_evt {
                KeyEvt::Char(c, _) => Some(*c),
                KeyEvt::Key(Key::Enter, _) => Some('\n'),
                KeyEvt::Key(Key::Tab, _) => Some('\t'),
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum KeyMatcher {
    Exact(KeyEvt),
    Number(KeyMods),
    AnyChar(KeyMods),
    AnyKey(KeyMods),
    Digit(KeyMods),
    Any,
}

impl KeyMatcher {
    pub fn try_consume(&self, kc: &mut KeyCombo) -> Vec<KeyEvt> {
        match self {
            KeyMatcher::Exact(evt) => {
                if let Some(k) = kc.pop_first_if(|k| k == evt) {
                    return vec![k];
                }
            }
            KeyMatcher::AnyChar(kmods) => {
                if let Some(k) =
                    kc.pop_first_if(|k| matches!(k, KeyEvt::Char(_, mods) if mods == kmods))
                {
                    return vec![k];
                }
            }
            KeyMatcher::AnyKey(kmods) => {
                if let Some(k) =
                    kc.pop_first_if(|k| matches!(k, KeyEvt::Key(_, mods) if mods == kmods))
                {
                    return vec![k];
                }
            }
            KeyMatcher::Number(kmods) => {
                let mut num = vec![];
                while kc.first_matches(|k| {
                    if let KeyEvt::Char(c, mods) = k {
                        mods == kmods && c.is_ascii_digit()
                    } else {
                        false
                    }
                }) {
                    num.push(kc.pop_first().unwrap())
                }
                return num;
            }
            KeyMatcher::Digit(kmods) => {
                if let Some(k) = kc.pop_first_if(|k| {
                    if let KeyEvt::Char(c, mods) = k {
                        mods == kmods && c.is_ascii_digit()
                    } else {
                        false
                    }
                }) {
                    return vec![k];
                }
            }
            KeyMatcher::Any => {
                if let Some(k) = kc.pop_first_if(|_| true) {
                    return vec![k];
                }
            }
        }
        return vec![];
    }
}

#[derive(Clone, Debug)]
pub struct KeyPatternClause(Vec<KeyMatcher>);

impl KeyPatternClause {
    pub fn try_consume(&self, kc: &mut KeyCombo) -> Vec<KeyEvt> {
        for unit in &self.0 {
            let consumed = unit.try_consume(kc);
            if consumed.len() > 0 {
                return consumed;
            }
        }
        return vec![];
    }
}

impl FromIterator<KeyMatcher> for KeyPatternClause {
    fn from_iter<T: IntoIterator<Item = KeyMatcher>>(iter: T) -> Self {
        KeyPatternClause(iter.into_iter().collect_vec())
    }
}

impl IntoIterator for KeyPatternClause {
    type Item = KeyMatcher;

    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Clone, Debug)]
pub struct KeyPattern(Vec<KeyPatternClause>);

impl KeyPattern {
    pub fn matches(&self, mut kc: KeyCombo) -> bool {
        for clause in &self.0 {
            let consumed = clause.try_consume(&mut kc);
            if consumed.is_empty() {
                return false;
            }
        }
        return kc.is_empty();
    }
}

impl FromIterator<KeyPatternClause> for KeyPattern {
    fn from_iter<T: IntoIterator<Item = KeyPatternClause>>(iter: T) -> Self {
        KeyPattern(iter.into_iter().map(|c| c.into()).collect_vec())
    }
}

impl IntoIterator for KeyPattern {
    type Item = KeyPatternClause;

    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
