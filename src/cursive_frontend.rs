use std::sync::mpsc;

use cursive::{
    theme::{BorderStyle, Palette, Style},
    utils::markup::StyledString,
    CbSink, CursiveRunnable, With,
};

pub mod views;

use crate::{
    cursor::SelectionIterator,
    document::Document,
    editor::{EditorStateSummary, ModalEditorError},
    events::{Key, KeyEvt, KeyMods},
    renderer_server::{RendererBackend, RendererEvent},
};

use self::views::{RootView, ViewBuilder};

impl From<cursive::event::Key> for Key {
    fn from(v: cursive::event::Key) -> Self {
        match v {
            cursive::event::Key::Enter => Key::Enter,
            cursive::event::Key::Tab => Key::Tab,
            cursive::event::Key::Backspace => Key::Backspace,
            cursive::event::Key::Esc => Key::Esc,
            cursive::event::Key::Left => Key::Left,
            cursive::event::Key::Right => Key::Right,
            cursive::event::Key::Up => Key::Up,
            cursive::event::Key::Down => Key::Down,
            cursive::event::Key::Ins => Key::Ins,
            cursive::event::Key::Del => Key::Del,
            cursive::event::Key::Home => Key::Home,
            cursive::event::Key::End => Key::End,
            cursive::event::Key::PageUp => Key::PageUp,
            cursive::event::Key::PageDown => Key::PageDown,
            cursive::event::Key::PauseBreak => Key::PauseBreak,
            cursive::event::Key::NumpadCenter => Key::NumpadCenter,
            cursive::event::Key::F0 => Key::F0,
            cursive::event::Key::F1 => Key::F1,
            cursive::event::Key::F2 => Key::F2,
            cursive::event::Key::F3 => Key::F3,
            cursive::event::Key::F4 => Key::F4,
            cursive::event::Key::F5 => Key::F5,
            cursive::event::Key::F6 => Key::F6,
            cursive::event::Key::F7 => Key::F7,
            cursive::event::Key::F8 => Key::F8,
            cursive::event::Key::F9 => Key::F9,
            cursive::event::Key::F10 => Key::F10,
            cursive::event::Key::F11 => Key::F11,
            cursive::event::Key::F12 => Key::F12,
        }
    }
}

impl KeyEvt {
    fn try_from_cursive_evt(evt: cursive::event::Event) -> Option<Self> {
        match evt {
            cursive::event::Event::Char(ch) => Some(KeyEvt::Char(ch, KeyMods::NONE)),
            cursive::event::Event::CtrlChar(ch) => Some(KeyEvt::Char(ch, KeyMods::CTRL)),
            cursive::event::Event::AltChar(ch) => Some(KeyEvt::Char(ch, KeyMods::ALT)),
            cursive::event::Event::Key(k) => Some(KeyEvt::Key(k.into(), KeyMods::NONE)),
            cursive::event::Event::Shift(k) => Some(KeyEvt::Key(k.into(), KeyMods::SHIFT)),
            cursive::event::Event::Alt(k) => Some(KeyEvt::Key(k.into(), KeyMods::ALT)),
            cursive::event::Event::AltShift(k) => {
                Some(KeyEvt::Key(k.into(), KeyMods::ALT | KeyMods::SHIFT))
            }
            cursive::event::Event::Ctrl(k) => Some(KeyEvt::Key(k.into(), KeyMods::CTRL)),
            cursive::event::Event::CtrlShift(k) => {
                Some(KeyEvt::Key(k.into(), KeyMods::CTRL | KeyMods::SHIFT))
            }
            cursive::event::Event::CtrlAlt(k) => {
                Some(KeyEvt::Key(k.into(), KeyMods::CTRL | KeyMods::ALT))
            }
            _ => None,
        }
    }
}

pub struct CursiveFrontend {
    cb_sink: Option<CbSink>,
    evt_chan: mpsc::Sender<RendererEvent>,
}

pub struct CursiveFrontendUserData {
    evt_chan: mpsc::Sender<RendererEvent>,
}

impl CursiveFrontend {
    pub fn init_cursive_context(&mut self) -> CursiveRunnable {
        let mut ctx = cursive::default();
        // Start with a nicer theme than default
        ctx.set_theme(cursive::theme::Theme {
            shadow: true,
            borders: BorderStyle::Simple,
            palette: Palette::retro().with(|palette| {
                use cursive::theme::BaseColor::*;
                {
                    // First, override some colors from the base palette.
                    use cursive::theme::Color::TerminalDefault;
                    use cursive::theme::PaletteColor::*;
                    palette[Background] = TerminalDefault;
                    palette[View] = TerminalDefault;
                    palette[Primary] = White.dark();
                    palette[TitlePrimary] = Blue.light();
                    palette[Secondary] = Blue.light();
                    palette[Highlight] = Blue.dark();
                }
                {
                    // Then override some styles.
                    use cursive::theme::Effect::*;
                    use cursive::theme::PaletteStyle::*;
                    palette[Highlight] = Style::from(Blue.light()).combine(Bold);
                }
            }),
        });
        let cb_sink = ctx.cb_sink().clone();
        ctx.add_fullscreen_layer(RootView::new());
        ctx.set_user_data(CursiveFrontendUserData {
            evt_chan: self.evt_chan.clone(),
        });
        self.cb_sink = Some(cb_sink);
        ctx
    }

    /// Runs the given callback with cursive context.
    fn send_cursive_callback<C>(&mut self, callback: C)
    where
        C: FnOnce(&mut cursive::Cursive) -> () + Send + 'static,
    {
        self.cb_sink
            .as_ref()
            .unwrap()
            .send(Box::new(callback))
            .unwrap();
    }
}

impl RendererBackend for CursiveFrontend {
    fn new(evt_chan: mpsc::Sender<RendererEvent>) -> Self {
        CursiveFrontend {
            cb_sink: Option::None,
            evt_chan,
        }
    }

    fn state_updated(&mut self, new_state: EditorStateSummary) {
        self.send_cursive_callback(move |ctx| {
            // Stylize the current text.
            let stylized_str = stylize(&new_state.curr_doc);
            views::EditorTextView::get(ctx).set_content(stylized_str);
            views::CmdBarView::get(ctx)
                .set_content(new_state.display.btm_bar_text.clone().unwrap_or_default());
            views::LogView::get(ctx).set_content(format!(
                "{} ({:?})",
                new_state.curr_mode, new_state.curr_combo
            ));
            new_state
                .display
                .mid_box_text
                .map(|txt| views::LogView::get(ctx).set_content(txt));
        });
    }

    fn quit(&mut self) {
        self.send_cursive_callback(|ctx| ctx.quit());
    }

    fn error(&mut self, error: ModalEditorError) {
        self.send_cursive_callback(move |ctx| {
            views::LogView::get(ctx).set_content(format!("error: {}", error.to_string()));
        });
    }
}

fn stylize_whitespaces(s: String) -> String {
    s.replace("\t", "····").replace("\n", "↩\n")
}

fn stylize(doc: &Document) -> StyledString {
    let merged_highlights = doc
        .selections
        .values()
        .cloned()
        .collect_merged(&doc.inner_buf);
    // let selection_heads = buf
    //     .selections
    //     .values()
    //     .map(|sel| sel.0)
    //     .collect::<HashSet<usize>>();
    let mut styled_content = StyledString::new();
    let mut last_point = 0;
    merged_highlights.iter().for_each(|(start, end)| {
        if last_point != *start {
            styled_content.append_plain(stylize_whitespaces(
                doc.inner_buf
                    .get_slice(last_point..*start)
                    .map(|s| s.to_string())
                    .unwrap_or(String::new()),
            ))
        }
        styled_content.append_styled(
            stylize_whitespaces(
                doc.inner_buf
                    .get_slice(*start..*end)
                    .map(|s| s.to_string())
                    .unwrap_or(String::new()),
            ),
            Style::highlight(),
        );
        last_point = *end;
    });
    if last_point < doc.inner_buf.len_chars() {
        styled_content.append_plain(stylize_whitespaces(
            doc.inner_buf
                .get_slice(last_point..doc.inner_buf.len_chars())
                .map(|s| s.to_string())
                .unwrap_or(String::new()),
        ))
    }
    // handle highlighted EOF
    if merged_highlights
        .last()
        .map(|hl| hl.1 == doc.inner_buf.len_chars() && hl.0 == hl.1)
        .unwrap_or(false)
    {
        styled_content.append_styled(" ", Style::highlight());
    }
    styled_content
}
