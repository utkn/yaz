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
    Resized(usize, usize),
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

    fn redraw(&mut self, state: EditorStateSummary) {
        let buf = state.curr_doc.get_buf();
        let mut tmp_stylizer = self.stylizer.clone();
        state
            .curr_doc
            .selections
            .values()
            .cloned()
            .collect_merged(buf)
            .into_iter()
            .for_each(|(start, end)| {
                tmp_stylizer.layer_region_style(start, end, [StyleAttr::Highlight]);
            });
        let max_chars = state.view.approx_displayed_len_chars(buf);
        let regions = tmp_stylizer.compute_regions(max_chars);
        self.frontend.state_updated(&state, regions);
    }

    pub fn run(mut self) {
        std::thread::spawn(move || {
            println!("RendererServer: started");
            loop {
                // First, try to receive an event from the backend.
                if let Ok(rnd_evt) = self.evt_chan.try_recv() {
                    match rnd_evt {
                        RendererEvent::KeyEvent(evt) => {
                            self.editor_conn.send_req(EditorServerReq::UIEvent(evt))
                        }
                        RendererEvent::Resized(new_width, new_height) => {
                            self.editor_conn
                                .send_req(EditorServerReq::UpdateViewEvent(new_width, new_height));
                        }
                    }
                }
                // Then, try to receive a message from the editor server.
                if let Ok(editor_msg) = self.editor_conn.try_receive_msg() {
                    match editor_msg {
                        EditorServerMsg::ErrorThrown(err) => {
                            self.frontend.error(err);
                        }
                        EditorServerMsg::QuitRequested => {
                            println!("RendererServer: quitting");
                            self.frontend.quit();
                            break;
                        }
                        EditorServerMsg::ViewUpdated(_new_height, state) => {
                            self.redraw(state);
                        }
                        EditorServerMsg::EditorResult(res, state) => {
                            self.redraw(state);
                        }
                        EditorServerMsg::StylizeInit(state) => {
                            self.stylizer.reset();
                            self.stylizer.layer_region_style(
                                0,
                                state.curr_doc.get_buf().len_chars(),
                                ConcreteStyle::default(),
                            );
                        }
                        EditorServerMsg::Stylize(start, end, style, _state) => {
                            self.stylizer.layer_region_style(start, end, style);
                        }
                        EditorServerMsg::StylizeEnd(state) => {
                            self.redraw(state);
                        }
                    }
                }
            }
        });
    }
}

pub trait RendererFrontend: Send {
    fn new(evt_chan: mpsc::Sender<RendererEvent>) -> Self;
    fn state_updated(
        &mut self,
        new_state: &EditorStateSummary,
        styles: Vec<(usize, usize, ConcreteStyle)>,
    );
    fn error(&mut self, error: ModalEditorError);
    fn quit(&mut self);
}
