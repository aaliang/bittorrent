use bt_messages::Message;
//use bt_messages::Message::{Choke, Unchoke};
use buffered_reader::BufferedReader;
use std::net::TcpStream;
use std::sync::mpsc::Sender;

pub struct DefaultHandler;

/// Handles messages. This is a cheap way to force reactive style
pub trait Handler {
    type MessageType;
    fn handle(&mut self, message: Self::MessageType, peer: &mut Peer) -> Action;
}

impl DefaultHandler {
    pub fn new () -> DefaultHandler {
        DefaultHandler
    }
}

pub enum Action {
    None,
    Respond(Vec<u8>)
}

/// The default algorithm
impl Handler for DefaultHandler {
    type MessageType = Message;
    #[inline]
    fn handle (&mut self, message: Message, peer: &mut Peer) -> Action {
        println!("{:?}", message);
        match message {
            Message::Choke => {
                peer.set_choked(true);
                Action::None
            },
            Message::Unchoke => {
                peer.set_choked(false);
                Action::None
            },
            _ => {
                Action::None
            }
        }
    }
}

pub struct Peer {
    pub id: String,
    pub chan: Sender<Action>,
    stream: TcpStream,
    state: State
}

#[derive(Debug)]
struct State {
    choked: bool,
    //the intention is that eventually we will support growable files. so going with vector
    bitfield: Vec<u8>
}

impl Peer {
    pub fn new (id:String, chan: Sender<Action>, stream: TcpStream) -> Peer {
        Peer {
            id: id,
            chan: chan,
            stream: stream,
            state: State {
                choked: true,
                bitfield: Vec::new()
            }
        }
    }

    fn set_choked (&mut self, choked: bool) {
        self.state.choked = choked;
    }
}

