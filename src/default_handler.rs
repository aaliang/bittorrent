use bt_messages::Message;
use buffered_reader::BufferedReader;
use chunk::{Position, Piece};
use peer::{Peer, SendPeerMessage};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use std::ops::{Deref, DerefMut};
use rand::{Rng, thread_rng};

const BLOCK_LENGTH:usize = 16384; //block length in bytes

pub struct GlobalState {
    gpc: Vec<u16>,
    pub owned: Vec<u8>,
    pub owned_pieces: Vec<Piece>,
    pub request_map: Vec<u8>,
    pub exclude: Vec<Piece>,
    s_request_map: Vec<u8>,
    piece_length: usize,
    peer_list: Vec<(Arc<RwLock<Peer>>, TcpStream)>
}

impl GlobalState {
    pub fn new (piece_length: usize) -> GlobalState {
        GlobalState {
            gpc: vec![],
            owned: vec![],
            owned_pieces: vec![],
            exclude: vec![],
            request_map: vec![],
            s_request_map: vec![],
            peer_list: vec![],
            piece_length: piece_length
        }
    }

    pub fn add_new_peer (&mut self, peer: Arc<RwLock<Peer>>, stream: TcpStream) {
        self.peer_list.push((peer, stream));
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
                        most_rare = (Some(true_index), population);
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

pub trait Spin {
    fn spin (&mut self);
}

impl Spin for GlobalState {
    fn spin (&mut self) {

        //NOTE: this shuffles the peer_list
        thread_rng().shuffle(&mut self.peer_list);
        for tup in self.peer_list.iter() {
            let (ref rw_lock_peer, _) = *tup;
            {
                let peer = match rw_lock_peer.try_read() { 
                    Ok(a) => a,
                    Err(_) => continue//do nothing. it's locked
                };
                //TODO: owned_pieces is not sufficient. it should be the union of owned_pieces and
                //requests
                let want = Piece::complement(&peer.deref().state.pieces, &self.exclude);
                let req_piece = match want.len() {
                    0 => continue,
                    _ => slice_piece(&want, &self.piece_length, &BLOCK_LENGTH)
                };

                println!("{:?}", req_piece);

                let index = Piece::add_to_boundary_vec(&mut self.exclude, req_piece);
                Piece::compact_if_possible(&mut self.exclude, index);
            }
        }
    }
}

fn slice_piece (pieces: &[Piece], piece_length: &usize, block_size: &usize) -> Piece {
    let &Piece {
        start: ref start,
        end: ref end
    } = pieces.first().unwrap();

    let piece = Piece::from(piece_length.to_owned(), start.index, start.offset, block_size.to_owned());

    if end < &piece.end {
        Piece::new(start.clone(), end.clone())
    } else {
        piece
    }
}

pub struct DefaultHandler;

/// Handles messages. This is a cheap way to force reactive style
pub trait Handler {
    type MessageType;
    fn handle(&mut self, message: Self::MessageType, peer: &mut Peer, global_state: &mut GlobalState);
}

/// The default algorithm
impl Handler for DefaultHandler {
    type MessageType = Message;
    #[inline]
    fn handle (&mut self, message: Message, peer: &mut Peer, global: &mut GlobalState) {
        println!("{:?}", message);
        match message {
            Message::Have{piece_index: index} => {
                let i = index as usize;
                global.gpc_incr(i, 1);
                //peer.state.set_have(i);
                let piece = Piece::from(BLOCK_LENGTH, i, 0, BLOCK_LENGTH);
                let i_index = Piece::add_to_boundary_vec(&mut global.owned_pieces, piece);
                Piece::compact_if_possible(&mut global.owned_pieces, i_index);
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
                        global.gpc_incr(index*8+i, n);
                    }
                }

                peer.state.set_pieces_from_bitfield(&bitfield);
                //peer.state.set_bitfield(bitfield);
            },
            _ => {
            }
        };
    }
}

/// Zip-maps a generic func over two byte slices with variable lengths
#[inline]
pub fn apply_bitwise_slice_vbr_len <F, T:Clone> (lhs: &[T], rhs: &[T], default: T, func: F) -> Vec<T>
    where F: Fn((&T, &T)) -> T {
    if lhs.len() == rhs.len() {
        bitwise_byte_slice(lhs, rhs, func)
    } else {
        if lhs.len() < rhs.len() {
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
    apply_bitwise_slice_vbr_len(lhs, rhs, 255, |(a, b)| a & b)
}
