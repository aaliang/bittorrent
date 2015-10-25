use bt_messages::Message;
use buffered_reader::BufferedReader;
use std::net::TcpStream;
use std::sync::mpsc::Sender;

/// Handles messages. This is a cheap way to force reactive style
pub trait Handler {
    type MessageType;
    fn handle(&mut self, message: Self::MessageType, peer: &mut Peer);
}

pub struct DefaultHandler {
    //the global piece count
    gpc: Vec<u16>
}

impl DefaultHandler {
    pub fn new () -> DefaultHandler {
        DefaultHandler {
            gpc: vec![]
        }
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
    fn handle (&mut self, message: Message, peer: &mut Peer) {
        println!("{:?}", message);
        match message {
            Message::Have{piece_index: index} => {
                peer.state.set_have(index as usize);
            }
            Message::Bitfield(bitfield) => {
                peer.state.set_bitfield(bitfield);
            },
            Message::Choke => {
                peer.state.set_choked(true);
            },
            Message::Unchoke => {
                peer.state.set_choked(false);
            },
            _ => {
            }
        };
    }
}

pub struct Peer {
    pub id: String,
    pub chan: Sender<Action>,
    stream: TcpStream,
    state: State
}

impl Peer {
    fn find_next_piece (&self) {
    }
}

#[derive(Debug)]
struct State {
    choked: bool,
    //the intention is that eventually we will support growable files. so going with vector
    bitfield: Vec<u8>
}

impl State {
    fn set_bitfield (&mut self, bitfield: Vec<u8>) {
        self.bitfield = bitfield;
    }

    fn set_have (&mut self, index: usize) {
        //lets say i have index 500 -> how do i bitwise over a u8 array?
        let chunk_index = index/8;
        let chunk_posit = index % 8;
        let chunk_mask = 128 >> chunk_posit;

        //bounds check needs to be here because the bitfield is a variable size - which we want in
        //the future
        if chunk_index+1 > self.bitfield.len() {
            self.bitfield.extend((0..chunk_index).map(|_| 0 as u8));
        }

        self.bitfield[chunk_index] = self.bitfield[chunk_index] | chunk_mask;
    }

    fn set_choked (&mut self, choked: bool) {
        self.choked = choked;
    }
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
}

#[test]
fn test_set_have_singleton_bitfield() {
    let mut state = State {
        choked: false,
        bitfield: vec![0]
    };
    state.set_have(2);

    assert_eq!(state.bitfield[0], 32);
}

#[test]
fn test_set_have_longer_bitfiled() {
    let mut state = State {
        choked: false,
        bitfield: vec![0, 0, 0, 0]
    };
    state.set_have(23);

    assert_eq!(state.bitfield[0], 0);
    assert_eq!(state.bitfield[1], 0);
    assert_eq!(state.bitfield[2], 1);
    assert_eq!(state.bitfield[3], 0);
}
