use std::sync::mpsc;

use crate::document::DocumentView;
use crate::editor::{EditorStateSummary, ModalEditor, ModalEditorError, ModalEditorResult};

use crate::events::KeyEvt;
use crate::render_server::ConcreteStyle;

#[derive(Clone, Debug)]
pub enum EditorServerReq {
    UIEvent(KeyEvt),
    StylizeInitEvent,
    StylizeEvent(usize, usize, ConcreteStyle),
    StylizeEndEvent,
    UpdateViewEvent(usize, usize),
}

#[derive(Clone, Debug)]
pub enum EditorServerMsg {
    QuitRequested,
    ErrorThrown(ModalEditorError),
    EditorResult(ModalEditorResult, EditorStateSummary),
    StylizeInit(EditorStateSummary),
    Stylize(usize, usize, ConcreteStyle, EditorStateSummary),
    StylizeEnd(EditorStateSummary),
    ViewUpdated(DocumentView, EditorStateSummary),
}

pub struct EditorConnection(
    mpsc::Sender<EditorServerReq>,
    mpsc::Receiver<EditorServerMsg>,
);

impl EditorConnection {
    pub fn receive_msg(&self) -> Result<EditorServerMsg, mpsc::RecvError> {
        self.1.recv()
    }

    pub fn try_receive_msg(&self) -> Result<EditorServerMsg, mpsc::TryRecvError> {
        self.1.try_recv()
    }

    pub fn send_req(&self, msg: EditorServerReq) {
        self.0.send(msg).unwrap();
    }
}

pub struct EditorServer {
    incoming_channel_rcv: mpsc::Receiver<EditorServerReq>,
    incoming_channel_snd: mpsc::Sender<EditorServerReq>,
    outgoing_channels: Vec<mpsc::Sender<EditorServerMsg>>,
    modal_state: ModalEditor,
}

impl EditorServer {
    pub fn new(init_state: ModalEditor) -> Self {
        let (snd, rcv) = mpsc::channel();
        EditorServer {
            incoming_channel_rcv: rcv,
            incoming_channel_snd: snd,
            outgoing_channels: Default::default(),
            modal_state: init_state,
        }
    }

    pub fn new_connection(&mut self) -> EditorConnection {
        let (snd, rcv) = mpsc::channel();
        self.outgoing_channels.push(snd);
        EditorConnection(self.incoming_channel_snd.clone(), rcv)
    }

    fn broadcast(&self, msg: EditorServerMsg) {
        for c in &self.outgoing_channels {
            c.send(msg.clone()).unwrap();
        }
    }

    fn handle_editor_results(
        &mut self,
        results: impl IntoIterator<Item = ModalEditorResult>,
    ) -> bool {
        let summary = self.modal_state.summarize();
        for result in results {
            match result {
                ModalEditorResult::QuitRequested => {
                    self.broadcast(EditorServerMsg::QuitRequested);
                    println!("EditorServer: quitting");
                    return false;
                }
                ModalEditorResult::ErrorThrown(err) => {
                    self.broadcast(EditorServerMsg::ErrorThrown(ModalEditorError::ModeError(
                        err,
                    )));
                }
                _ => {
                    self.broadcast(EditorServerMsg::EditorResult(result, summary.clone()));
                }
            }
        }
        return true;
    }

    pub fn run(mut self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            println!("EditorServer: started");
            loop {
                if let Ok(req) = self.incoming_channel_rcv.recv() {
                    match req {
                        EditorServerReq::UIEvent(evt) => {
                            self.modal_state.receive_key(evt);
                            match self.modal_state.update() {
                                Ok(results) => {
                                    let should_continue = self.handle_editor_results(results);
                                    if !should_continue {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    self.broadcast(EditorServerMsg::ErrorThrown(err));
                                }
                            }
                            self.modal_state.update_view();
                        }
                        EditorServerReq::UpdateViewEvent(new_width, new_height)
                            if new_height != self.modal_state.get_view().max_height
                                || new_width != self.modal_state.get_view().max_width =>
                        {
                            self.modal_state.get_view_mut().max_height = new_height;
                            self.modal_state.get_view_mut().max_width = new_width;
                            let summary = self.modal_state.summarize();
                            self.broadcast(EditorServerMsg::ViewUpdated(
                                *self.modal_state.get_view(),
                                summary,
                            ));
                        }
                        EditorServerReq::StylizeInitEvent => {
                            let summary = self.modal_state.summarize();
                            self.broadcast(EditorServerMsg::StylizeInit(summary));
                        }
                        EditorServerReq::StylizeEvent(start, end, style) => {
                            let summary = self.modal_state.summarize();
                            self.broadcast(EditorServerMsg::Stylize(start, end, style, summary));
                        }
                        EditorServerReq::StylizeEndEvent => {
                            let summary = self.modal_state.summarize();
                            self.broadcast(EditorServerMsg::StylizeEnd(summary));
                        }
                        _ => {}
                    };
                }
            }
        })
    }
}
