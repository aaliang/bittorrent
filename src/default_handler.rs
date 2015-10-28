use bt_messages::Message;
use buffered_reader::BufferedReader;
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::io::Write;
use std::cmp::Ordering;
const BLOCK_LENGTH:usize = 16384; //block length in bytes

#[derive(PartialEq, Debug, Clone)]
pub struct Position {
    index: usize,
    offset: usize
}

impl Position {
    pub fn new (index: usize, offset: usize) -> Position {
        Position {
            index: index,
            offset: offset
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp (&self, rhs: &Position) -> Option<Ordering> {
         if self.index < rhs.index {
            Some(Ordering::Less)
         } else if self.index > rhs.index {
            Some(Ordering::Greater)
         } else {
            if self.offset < rhs.offset {
                Some(Ordering::Less)
            } else if self.offset > rhs.offset {
                Some(Ordering::Greater)
            } else {
                Some(Ordering::Equal)
            }
         }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Piece {
    start: Position,
    end: Position
}

impl Piece {
    pub fn new (start: Position, end: Position) -> Piece {
        Piece {
            start: start,
            end: end
        }
    }

    //start is inclusive, end is exclusive
    pub fn from (piece_length: usize, index: usize, offset: usize, bytes: usize) -> Piece {
        let num_whole_pieces = bytes/piece_length;
        let rem_offset = (offset + bytes) % piece_length;

        let start = Position {
            index: index,
            offset: offset
        };
        let end = Position {
            index: index + num_whole_pieces,
            offset: rem_offset
        };

        Piece{start: start, end: end}
    }

}


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
    pub owned: Vec<u8>,
    //outgoing requests
    pub request_map: Vec<u8>,

    s_request_map: Vec<Piece>,

    piece_length: usize
}

impl DefaultHandler {
    pub fn new (piece_length: usize) -> DefaultHandler {
        DefaultHandler {
            gpc: vec![],
            owned: vec![],
            request_map: vec![],
            s_request_map: vec![],
            piece_length: piece_length
        }
    }

    pub fn convert_bitfield_to_piece_vec (bitfield: &[u8]) -> Vec<Piece> {
        let mut vec = Vec::new();
        let mut a_start = None;
        for (bitmap_byte_num, byte) in bitfield.iter().enumerate() {
            let mut bitmap_offset = 0;
            let mut remainder = byte.to_owned(); 
            loop {
                match remainder.leading_zeros() {
                    0 => (),
                    x => {
                        let n = if x > 8 - bitmap_offset { 8 -bitmap_offset} else {x};
                        bitmap_offset += n;

                        match a_start {
                            Some(_) => {
                                let end = Position::new((bitmap_byte_num as u32 * 8 + bitmap_offset - n as u32) as usize, 0);
                                vec.push(Piece::new(a_start.unwrap(), end));
                                a_start = None;
                            },
                            None => {}
                        }

                        remainder = remainder << n;
                    }
                };
                match (!remainder).leading_zeros() { //leading 1's after shifting
                    0 => (),
                    n => {
                        match a_start {
                            Some(_) => {/*do nothing*/},
                            None => {
                                a_start = Some(Position::new((bitmap_byte_num as u32 * 8 + bitmap_offset as u32) as usize, 0));
                            }
                        }

                        bitmap_offset += n;
                        println!("br: {}", remainder);
                        remainder = remainder << n;
                        println!("{}, shifted {}", remainder, n);
                    }
                };
                if bitmap_offset == 8 {
                    bitmap_offset = 0;
                    break;
                };
            }
        }

        match a_start {
            Some(_) => {
                vec.push((
                    Piece::new(a_start.unwrap(), 
                    Position::new(bitfield.len() * 8, 0))));
            },
            _ => ()
        };
        vec
    }

    ///attempts to compact the piece indexed by {index} with elements to its left and right
    #[inline]
    pub fn compact_if_possible(arr: &mut Vec<Piece>, index: usize) {
        let res = {
            let ref el = arr[index];
            match ((arr.get(index-1), arr.get(index+1))) {
                (Some(ref left), Some(ref right)) if left.end == el.start && el.end == right.start => {
                    Some((index-1, index+1, Piece::new(left.start.clone(), right.end.clone())))},
                (Some(ref left), _) if left.end == el.start => {
                    Some((index-1, index, Piece::new(left.start.clone(), el.end.clone())))},
                (_, Some(ref right)) if el.end == right.start => {
                    Some((index, index+1, Piece::new(el.start.clone(), right.end.clone())))}
                _ => None
            }
        };
        match res {
            Some((start_index, end_index, compacted_piece)) => {
                for (n, i) in (start_index..end_index+1).enumerate() {
                    arr.remove(n-i);
                }
                arr.insert(start_index, compacted_piece);
            },
            _ => ()
        }
    }

    #[inline]
    ///returns the index at which the chunk was inserted into the vector
    //pub fn add_to_boundary_vec(arr: &mut Vec<Piece>) -> usize {
    pub fn add_to_boundary_vec(arr: &mut Vec<Piece>, new_block: Piece) -> usize {
        //let new_block = DefaultHandler::get_block_boundaries(piece_length, index, offset, bytes);
        if arr.len() == 0 || new_block.start >= arr.last().unwrap().end {
            arr.push(new_block);
            arr.len() - 1
        } else if new_block.end <= arr.first().unwrap().start {
            arr.insert(0, new_block);
            0
        } else {
            let (mut win_left, mut win_right) = (0, arr.len());
            while (win_left < win_right) { //should probably just use loop {}
                let arr_index = (win_left+win_right)/2;
                let something = {
                    let block = &arr[arr_index];
                    let el_left = &arr[arr_index - 1];
                    let el_right = arr.get(arr_index + 1);
                    if new_block.start >= block.end {
                        match el_right {
                            a @ None | a @ Some(_) if new_block.end <= a.unwrap().start => {
                                Some(arr_index+1)
                            },
                            _ => {
                                win_left = arr_index + 1;
                                None
                            }
                        }
                    }
                    else if new_block.end <= block.start {
                            if new_block.start >= el_left.end {
                                Some(arr_index)
                            } else {
                                win_right = arr_index - 1;
                                None
                            }
                    }
                    else { panic!("this is bad")}
                };

                match something {
                    Some(i) => {
                        arr.insert(i, new_block);
                        return i
                    },
                    _ => ()
                }
            }
            //if (win_left > win_right) {
            panic!("this is also bad");
            //}
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

    /// Returns the index of the rarest piece that isn't owned or currently being requested.
    /// TODO: Approximations may yield optimized results
    #[inline]
    pub fn rarest (&self) -> Option<usize> {
        //there's probably a faster way. doing this naively for the sake of forward progress
        let mut most_rare = (None, u16::max_value());
        for (index, byte) in self.unclaimed_fields().iter().enumerate() {
            for i in 0..8 { //cast up so i don't have to deal with overflows
                let n = 1 & (((*byte as u16) << i) >> (8-i));
                if n == 1 {
                    let true_index = index*8+1;
                    let population = self.gpc[true_index];
                    let (_, mr_pop) = most_rare;
                    if population < mr_pop {
                        most_rare = (Some(true_index), population)
                    }
                }
            }
        }
        let (index, _) = most_rare;
        index
    }

    /// Returns the index of the rarest piece that is owned by the peer and isn't both owned
    /// and currently being requested
    /// TODO: Approximations may yield optimized results
    #[inline]
    pub fn rarest_wrt_peer (&self, peer_bitfield: &Vec<u8>) -> Option<usize> {
        //there's probably a faster way. doing this naively for the sake of forward progress
        let mut most_rare = (None, u16::max_value());
        let eligible = and_slice_vbr_len(&self.unclaimed_fields(), &peer_bitfield);
        for (index, byte) in eligible.iter().enumerate() {
            for i in 0..8 { //cast up so i don't have to deal with overflows
                let n = 1 & (((*byte as u16) << i) >> (8-i));
                //println!("n: {}", n);
                if n == 1 {
                    let true_index = index*8+1;
                    let population = self.gpc[true_index];
                    let (_, mr_pop) = most_rare;
                    if population < mr_pop {
                        most_rare = (Some(true_index), population)
                    }
                }
            }
        }
        let (index, _) = most_rare;
        index
    }

    /// returns a complete bitfield of pieces that aren't owned or being requested
    /// this is done almost as strictly as possible - and might be a little of a waste as it isn't
    /// really necessary to get a complete picture to get a request chunk
    /// additionally the definitions of owned and request_map are not strict yet - currently they
    /// are growable, and impelementers should take note of that
    #[inline]
    pub fn unclaimed_fields (&self) -> Vec<u8> {
        nand_slice_vbr_len(&self.owned, &self.request_map)
    }

    #[inline]
    pub fn req (&mut self, peer_bitfield: &Vec<u8>) {
        let index = self.rarest_wrt_peer(peer_bitfield);
        println!("REQ: {:?}", index);
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
                self.req(&peer.state.bitfield);
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
                        self.gpc_incr(index*8+i, n);
                    }
                }
                peer.state.set_bitfield(bitfield);

                /*
                let candidates = self.unclaimed_fields();
                let request = peer.get_request(&candidates);
                */

                self.req(&peer.state.bitfield);
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

#[derive(Debug)]
pub struct State {
    //are we choked by them?
    pub us_choked: bool,
    //are we interested in them?
    pub us_interested: bool,
    //are they choked by us?
    pub is_choked: bool,
    //are they interested in us?
    pub is_interested: bool,
    //the intention is that eventually we will support growable files. so going with vector
    pub bitfield: Vec<u8>
}

impl State {
    pub fn new () -> State {
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

    pub fn set_bitfield (&mut self, bitfield: Vec<u8>) {
        self.bitfield = bitfield;
    }

    pub fn set_have (&mut self, index: usize) {
        set_have_bitfield(&mut self.bitfield, index);
    }

    pub fn set_us_choked (&mut self, us_choked: bool) {
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

/// Zip-maps a generic func over two byte slices with variable lengths
#[inline]
pub fn apply_bitwise_slice_vbr_len <F, T:Clone> (lhs: &[T], rhs: &[T], default: T, func: F) -> Vec<T>
    where F: Fn((&T, &T)) -> T {
    if lhs.len() == rhs.len() {
        bitwise_byte_slice(lhs, rhs, func)
    } else {
        if lhs.len() < rhs.len() {
            //let mut a = (&rhs[..lhs.len()]).to_owned();
            let mut a = lhs.to_owned();
            a.extend((0..rhs.len()-lhs.len()).map(|_| default.clone()));
            bitwise_byte_slice(&a, rhs, func)
        } else {
            let mut b = rhs.to_owned();
            b.extend((0..lhs.len()-rhs.len()).map(|_| default.clone()));
            bitwise_byte_slice(lhs, &b, func)
        }
    }
}

/// Zip-maps a generic func (intended bitwise) over two byte slices
#[inline]
pub fn bitwise_byte_slice <F, T> (lhs: &[T], rhs: &[T], func: F) -> Vec<T>
    where F: Fn((&T, &T)) -> T {
    println!("llen: {}, rlen: {}", lhs.len(), rhs.len());
    assert!(lhs.len() == rhs.len());
    lhs.iter().zip(rhs)
              .map(func)
              .collect::<Vec<T>>()
}

#[inline]
pub fn nand_slice_vbr_len (lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    apply_bitwise_slice_vbr_len(lhs, rhs, 0, |(a, b)| !a & !b)
}

#[inline]
pub fn and_slice_vbr_len(lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    println!("lhs: {:?}", lhs);
    println!("rhs: {:?}", rhs);
    apply_bitwise_slice_vbr_len(lhs, rhs, 255, |(a, b)| a & b)
}
