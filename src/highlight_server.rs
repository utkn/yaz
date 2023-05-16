use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

use crate::editor::editor_server::*;

pub struct HighlightServer {
    editor_conn: EditorConnection,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl HighlightServer {
    pub fn new(editor_conn: EditorConnection) -> Self {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        HighlightServer {
            editor_conn,
            syntax_set: ps,
            theme_set: ts,
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
                            for (line_idx, line) in new_state.curr_doc.inner_buf.lines().enumerate()
                            {
                                self.editor_conn
                                    .send_req(EditorServerReq::HighlightResetEvent(line_idx));
                                for (style, s) in highlighter
                                    .highlight_line(&line.to_string(), &self.syntax_set)
                                    .unwrap()
                                {
                                    self.editor_conn.send_req(EditorServerReq::HighlightEvent(
                                        line_idx,
                                        style,
                                        s.to_string(),
                                    ));
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
