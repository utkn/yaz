use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

use crate::{
    editor::editor_server::*,
    render_server::{Color, Style},
};

pub struct HighlightServer {
    editor_conn: EditorConnection,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl From<syntect::highlighting::Color> for Color {
    fn from(value: syntect::highlighting::Color) -> Self {
        Self(value.r, value.g, value.b, value.a)
    }
}

impl From<syntect::highlighting::Style> for Style {
    fn from(value: syntect::highlighting::Style) -> Self {
        Self {
            fg: value.foreground.into(),
            bg: value.background.into(),
            highlight: false,
        }
    }
}

impl HighlightServer {
    pub fn new(editor_conn: EditorConnection) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        HighlightServer {
            editor_conn,
            syntax_set,
            theme_set,
        }
    }

    pub fn run(self) {
        std::thread::spawn(move || {
            println!("HighlightServer: started");
            loop {
                // Then, try to receive a message from the editor server.
                if let Ok(editor_msg) = self.editor_conn.try_receive_msg() {
                    match editor_msg {
                        EditorServerMsg::StateUpdated(new_state) => {
                            // get the extension
                            let syntax = new_state
                                .curr_doc
                                .get_ext()
                                .and_then(|ext| self.syntax_set.find_syntax_by_extension(&ext));
                            if syntax.is_none() {
                                continue;
                            }
                            // start highlighting.
                            let mut highlighter = HighlightLines::new(
                                &syntax.unwrap(),
                                &self.theme_set.themes["base16-ocean.dark"],
                            );
                            for (line_idx, line) in new_state.curr_doc.get_buf().lines().enumerate()
                            {
                                let mut curr_char_idx = new_state
                                    .curr_doc
                                    .get_buf()
                                    .try_line_to_char(line_idx)
                                    .unwrap_or(0);
                                for (style, s) in highlighter
                                    .highlight_line(&line.to_string(), &self.syntax_set)
                                    .unwrap()
                                {
                                    self.editor_conn.send_req(EditorServerReq::StylizeEvent(
                                        curr_char_idx,
                                        curr_char_idx + s.chars().count(),
                                        style.into(),
                                    ));
                                    curr_char_idx += s.chars().count();
                                }
                            }
                        }
                        EditorServerMsg::QuitRequested => {
                            println!("HighlightServer: quitting");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });
    }
}
