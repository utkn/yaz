use std::sync::mpsc;

use crate::{
    editor::{editor_server::*, EditorStateSummary, ModalEditorError},
    events::KeyEvt,
};

#[derive(Clone, Debug)]
pub enum RendererEvent {
    KeyEvent(KeyEvt),
}

pub struct RendererServer<T> {
    editor_conn: EditorConnection,
    backend: T,
    evt_chan: mpsc::Receiver<RendererEvent>,
}

impl<T> RendererServer<T>
where
    T: RendererBackend + 'static,
{
    pub fn new(editor_conn: EditorConnection) -> Self {
        let (snd, rcv) = mpsc::channel();
        let backend = T::new(snd);
        RendererServer {
            editor_conn,
            backend,
            evt_chan: rcv,
        }
    }

    pub fn get_backend_mut(&mut self) -> &mut T {
        &mut self.backend
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
                    }
                }
                // Then, try to receive a message from the editor server.
                if let Ok(editor_msg) = self.editor_conn.try_receive_msg() {
                    match editor_msg {
                        EditorServerMsg::StateUpdated(new_state) => {
                            self.backend.state_updated(new_state);
                        }
                        EditorServerMsg::HighlightResetRequest(line_idx) => {}
                        EditorServerMsg::HighlightRequest(line_idx, style, s) => {}
                        EditorServerMsg::Error(err) => {
                            self.backend.error(err);
                        }
                        EditorServerMsg::QuitRequested => {
                            println!("RendererServer: quitting");
                            self.backend.quit();
                            break;
                        }
                    }
                }
            }
        });
    }
}

pub trait RendererBackend: Send {
    fn new(evt_chan: mpsc::Sender<RendererEvent>) -> Self;
    fn state_updated(&mut self, new_state: EditorStateSummary);
    fn error(&mut self, error: ModalEditorError);
    fn quit(&mut self);
}
