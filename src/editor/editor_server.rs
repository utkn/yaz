use std::sync::mpsc;

use crate::editor::{EditorStateSummary, ModalEditor, ModalEditorError, ModalEditorResult};

use crate::events::KeyEvt;
use crate::render_server::Style;

#[derive(Clone, Debug)]
pub enum EditorServerReq {
    UIEvent(KeyEvt),
    StylizeEvent(usize, usize, Style),
}

#[derive(Clone, Debug)]
pub enum EditorServerMsg {
    StateUpdated(EditorStateSummary),
    QuitRequested,
    Error(ModalEditorError),
    StylizeRequest(usize, usize, Style),
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

    pub fn run(mut self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            println!("EditorServer: started");
            self.broadcast(EditorServerMsg::StateUpdated(self.modal_state.summary()));
            loop {
                if let Ok(req) = self.incoming_channel_rcv.recv() {
                    match req {
                        EditorServerReq::UIEvent(evt) => {
                            self.modal_state.receive_key(evt);
                            match self.modal_state.update() {
                                Ok(ModalEditorResult::QuitRequested) => {
                                    self.broadcast(EditorServerMsg::QuitRequested);
                                    println!("EditorServer: quitting");
                                    break;
                                }
                                Ok(ModalEditorResult::StateUpdated) => {
                                    self.broadcast(EditorServerMsg::StateUpdated(
                                        self.modal_state.summary(),
                                    ));
                                }
                                Err(err) => {
                                    self.broadcast(EditorServerMsg::Error(err));
                                }
                            }
                        }
                        EditorServerReq::StylizeEvent(start, end, style) => {
                            self.broadcast(EditorServerMsg::StylizeRequest(start, end, style));
                        }
                    };
                }
            }
        })
    }
}
