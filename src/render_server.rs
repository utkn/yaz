use std::sync::mpsc;

use crate::{
    cursor::SelectionIterator,
    editor::{editor_server::*, EditorStateSummary, ModalEditorError},
    events::KeyEvt,
};

pub use self::stylizer::*;

mod stylizer;

#[derive(Clone, Debug)]
pub enum RendererEvent {
    KeyEvent(KeyEvt),
}

pub struct RendererServer<T> {
    editor_conn: EditorConnection,
    frontend: T,
    evt_chan: mpsc::Receiver<RendererEvent>,
    stylizer: Stylizer,
}

impl<T> RendererServer<T>
where
    T: RendererFrontend + 'static,
{
    pub fn new(editor_conn: EditorConnection) -> Self {
        let (snd, rcv) = mpsc::channel();
        RendererServer {
            editor_conn,
            frontend: T::new(snd),
            evt_chan: rcv,
            stylizer: Default::default(),
        }
    }

    pub fn get_frontend_mut(&mut self) -> &mut T {
        &mut self.frontend
    }

    pub fn run(mut self) {
        std::thread::spawn(move || {
            println!("RendererServer: started");
            let mut last_state = EditorStateSummary::default();
            loop {
                // First, try to receive an event from the backend.
                if let Ok(rnd_evt) = self.evt_chan.try_recv() {
                    match rnd_evt {
                        RendererEvent::KeyEvent(evt) => {
                            self.editor_conn.send_req(EditorServerReq::UIEvent(evt))
                        }
                    }
                }
                // Then, try to receive a message from the editor server.
                if let Ok(editor_msg) = self.editor_conn.try_receive_msg() {
                    match editor_msg {
                        EditorServerMsg::StateUpdated(new_state) => {
                            let buf = new_state.curr_doc.get_buf();
                            self.stylizer.set_len_chars(buf.len_chars());
                            self.stylizer.set_highlighted_regions(
                                new_state
                                    .curr_doc
                                    .selections
                                    .values()
                                    .cloned()
                                    .collect_merged(buf),
                            );
                            self.frontend
                                .state_updated(&new_state, self.stylizer.compute_regions());
                            last_state = new_state;
                        }
                        EditorServerMsg::StylizeRequest(start, end, style) => {
                            self.stylizer.add_styled_region((start, end), style);
                            self.frontend
                                .state_updated(&last_state, self.stylizer.compute_regions());
                        }
                        EditorServerMsg::Error(err) => {
                            self.frontend.error(err);
                        }
                        EditorServerMsg::QuitRequested => {
                            println!("RendererServer: quitting");
                            self.frontend.quit();
                            break;
                        }
                    }
                }
            }
        });
    }
}

pub trait RendererFrontend: Send {
    fn new(evt_chan: mpsc::Sender<RendererEvent>) -> Self;
    fn state_updated(&mut self, new_state: &EditorStateSummary, styles: Vec<(usize, usize, Style)>);
    fn error(&mut self, error: ModalEditorError);
    fn quit(&mut self);
}
