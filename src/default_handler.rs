use bt_messages::Message;
use buffered_reader::BufferedReader;
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::io::Write;

const BLOCK_LENGTH:usize = 16384; //block length in bytes

/// Handles messages. This is a cheap way to force reactive style
pub trait Handler {
    type MessageType;
    fn handle(&mut self, message: Self::MessageType, peer: &mut Peer);
}


pub struct DefaultHandler {
    //the global piece count
    gpc: Vec<u16>,
    //pieces owned by self. (as a bitfield)
    owned: Vec<u8>,
    //outgoing requests
    request_map: Vec<u8>
}

impl DefaultHandler {
    pub fn new () -> DefaultHandler {
        DefaultHandler {
            gpc: vec![],
            owned: vec![],
            request_map: vec![]
        }
    }
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
                peer.state.set_us_choked(true);
            },
            Message::Unchoke => {
                peer.state.set_us_choked(false);
            },
            Message::Interested => {
                peer.state.set_us_interested(true);
            },
            _ => {
            }
        };
    }
}

pub struct Peer {
    pub id: String,
    stream: TcpStream,
    pub state: State
}

impl Peer {
    pub fn new (id:String, stream: TcpStream) -> Peer {
        Peer {
            id: id,
            stream: stream,
            state: State::new()
        }
    }

    pub fn send_message (&mut self, message: Message) {
        let as_bytes = message.to_byte_array();
        self.stream.write_all(&as_bytes);
    }
}

#[derive(Debug)]
pub struct State {
    //are we choked by them?
    us_choked: bool,
    //are we interested in them?
    us_interested: bool,
    //are they choked by us?
    is_choked: bool,
    //are they interested in us?
    is_interested: bool,
    //the intention is that eventually we will support growable files. so going with vector
    bitfield: Vec<u8>
}

impl State {
    fn new () -> State {
        State {
            us_choked: true,
            us_interested: false,
            is_choked: true,
            is_interested: false,
            bitfield: vec![]
        }
    }

    pub fn set_us_interested (&mut self, us_interested: bool) {
        self.us_interested = us_interested;
    }

    fn set_bitfield (&mut self, bitfield: Vec<u8>) {
        self.bitfield = bitfield;
    }

    fn set_have (&mut self, index: usize) {
        set_have_bitfield(&mut self.bitfield, index);
    }

    fn set_us_choked (&mut self, us_choked: bool) {
        self.us_choked = us_choked;
    }
}

fn set_have_bitfield (bitfield: &mut Vec<u8>, index: usize) {
    //lets say i have index 500 -> how do i bitwise over a u8 array?
    let chunk_index = index/8;
    let chunk_posit = index % 8;
    let chunk_mask = 128 >> chunk_posit;

    //bounds check needs to be here because the bitfield is a variable size - which we want in
    //the future
    if chunk_index+1 > bitfield.len() {
        bitfield.extend((0..chunk_index).map(|_| 0 as u8));
    }

    bitfield[chunk_index] = bitfield[chunk_index] | chunk_mask;
}

#[test]
fn test_set_have_singleton_bitfield() {
    let mut state = State::new();

    state.set_bitfield(vec![0]);
    state.set_have(2);

    assert_eq!(state.bitfield[0], 32);
}

#[test]
fn test_set_have_longer_bitfiled() {
    let mut state = State::new();

    state.set_bitfield(vec![0, 0, 0, 0]);
    state.set_have(23);

    assert_eq!(state.bitfield[0], 0);
    assert_eq!(state.bitfield[1], 0);
    assert_eq!(state.bitfield[2], 1);
    assert_eq!(state.bitfield[3], 0);
}

#[test]
fn test_set_have_out_of_bounds() {
    let mut state = State::new();

    state.set_bitfield(vec![0, 1]);
    state.set_have(31);

    assert_eq!(state.bitfield[0], 0);
    assert_eq!(state.bitfield[1], 1);
    assert_eq!(state.bitfield[2], 0);
    assert_eq!(state.bitfield[3], 1);
}
