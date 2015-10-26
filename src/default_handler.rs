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

//TODO: state probably shouldn't be stored here in the handler... eventually move it back in main. for each
//torrent
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

    /// Increases the value of gpc[piece_index] by n
    #[inline]
    pub fn gpc_incr (&mut self, piece_index: usize, n: u16) {
        //starting to regret making the bitfield variable in size... maybe i can preallocate. will come back and re-eval
        let len = self.gpc.len();
        if piece_index >= len {
            self.gpc.extend((0..piece_index+1 - len).map(|_| 0));
        }
        self.gpc[piece_index] += n;
    }

    /// returns a complete bitfield of pieces that aren't owned or being requested
    /// this is done almost as strictly as possible - and might be a little of a waste as it isn't
    /// really necessary to get a complete picture to get a request chunk
    /// additionally the definitions of owned and request_map are not strict yet - currently they
    /// are growable, and impelementers should take note of that
    pub fn unclaimed_fields (&self) -> Vec<u8> {
        if self.owned.len() == self.request_map.len() {
            nand_slice(&self.owned, &self.request_map)
        } else {
            if self.owned.len() < self.request_map.len() {
                let mut vec = nand_slice(&self.request_map[..self.owned.len()], &self.owned);
                vec.extend((0..self.request_map.len()-self.owned.len()).map(|_| 255));
                vec
            } else {
                let mut vec = nand_slice(&self.request_map, &self.owned[..self.request_map.len()]);
                vec.extend((0..self.owned.len()-self.request_map.len()).map(|_| 255));
                vec
            }
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
                let i = index as usize;
                self.gpc_incr(i, 1);
                peer.state.set_have(i);
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
            Message::Bitfield(bitfield) => {
                for (index, byte) in bitfield.iter().enumerate() {
                    for i in 0..8 { //cast up so i don't have to deal with overflows
                        let n = 1 & (((*byte as u16) << i) >> (8-i));
                        self.gpc_incr(index*8+i, n & 255);
                    }
                }
                peer.state.set_bitfield(bitfield);

                let candidates = self.unclaimed_fields();
                let request = peer.get_request(&candidates);
                println!("{:?}", request);

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
        let _ = self.stream.write_all(&as_bytes);
    }
}

trait RequestGenerator {
    /// Given a bitfield of eligible candidates, returns if possible the (index, begin, and length)
    /// corresponding to the fields in Message::Request
    fn get_request(&self, candidate_field: &[u8]) -> Option<(u32, u32, u32)>;
}

impl RequestGenerator for Peer {
    fn get_request(&self, candidate_field: &[u8]) -> Option<(u32, u32, u32)> {
        Some((1, 1, 1))
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

#[inline]
fn set_have_bitfield (bitfield: &mut Vec<u8>, index: usize) {
    let chunk_index = index/8;
    let chunk_posit = index % 8;
    let chunk_mask = 128 >> chunk_posit;

    //bounds check needs to be here because the bitfield is a variable size - which we want in
    //the future
    if chunk_index+1 > bitfield.len() {
        bitfield.extend((0..chunk_index).map(|_| 0));
    }

    bitfield[chunk_index] = bitfield[chunk_index] | chunk_mask;
}


#[inline]
pub fn nand_slice (lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    assert!(lhs.len() == rhs.len());
    lhs.iter().zip(rhs)
              .map(|(a, b)| !a & !b)
              .collect::<Vec<u8>>()
}

#[test]
fn test_nand_slice() {
    let a = vec![0, 0];
    let b = vec![0, 1];
    let c = nand_slice(&a, &b);

    assert_eq!(c, vec![255, 254]);
}

#[test]
fn test_unclaimed_fields() {
    let mut handler = DefaultHandler::new();

    handler.owned = vec![0, 0, 0];
    handler.request_map = vec![1, 0];

    let c = handler.unclaimed_fields();
    assert_eq!(c, vec![254, 255, 255]);
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
