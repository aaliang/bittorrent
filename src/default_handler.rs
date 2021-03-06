extern crate time;

use bt_messages::{Message, try_decode};
use buffered_reader::BufferedReader;
use chunk::{Position, Piece};
use peer::{Peer, SendPeerMessage};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use std::ops::{Deref, DerefMut};
use rand::{Rng, thread_rng};
use metadata::Metadata;
const BLOCK_LENGTH:usize = 16384; //block length in bytes

pub struct GlobalState {
    gpc: Vec<u16>,
    pub owned: Vec<u8>,
    pub owned_pieces: Vec<Piece>,
    pub request_map: Vec<u8>,
    pub requests: Vec<(Piece, i64)>,
    s_request_map: Vec<u8>,
    piece_length: usize,
    pieces_hash: Vec<u8>,
    peer_list: Vec<(Arc<RwLock<Peer>>, TcpStream, i64, Vec<u8>)>
}

impl GlobalState {
    pub fn new (metadata: &Metadata) -> GlobalState {
        GlobalState {
            gpc: vec![],
            owned: vec![],
            owned_pieces: vec![],
            request_map: vec![],
            requests: vec![],
            s_request_map: vec![],
            peer_list: vec![],
            piece_length: metadata.piece_length as usize,
            pieces_hash: metadata.pieces.clone()
        }
    }

    pub fn add_new_peer (&mut self, peer: Arc<RwLock<Peer>>, stream: TcpStream, peer_id: Vec<u8>) {
        let last_checkin = time::get_time().sec;
        self.peer_list.push((peer, stream, last_checkin, peer_id));
    }

    pub fn remove_peer(&mut self, id: &[u8]) {
        self.peer_list.retain(|x| {
            &x.3[..] != id
        });
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
}

pub trait Spin {
    fn spin (&mut self);
}

const WANT_LIMIT:usize = 1500;
const EXPIRE_FACTOR:usize = 2;
const MIN_TIMEOUT:usize = 15;

impl Spin for GlobalState {
    fn spin (&mut self) {
        //NOTE: this shuffles the peer_list
        println!("{} peers", self.peer_list.len());
        thread_rng().shuffle(&mut self.peer_list);

        let mut exclude = self.owned_pieces.clone();

        for &(ref request, _) in self.requests.iter() {
            Piece::add_to_boundary_vec(&mut exclude, request.clone());
        }

        for tup in self.peer_list.iter_mut() {
            let (ref rw_lock_peer, ref mut peer_socket, ref mut timestamp, _) = *tup;
            {
                let now = time::get_time().sec;
                if timestamp < &mut(now - 120) {
                    peer_socket.send_message(Message::KeepAlive);
                    *timestamp = now;
                }

                if self.requests.len() < WANT_LIMIT {
                     let peer = match rw_lock_peer.try_read() {
                        Ok(a) => a,
                        Err(_) => continue//do nothing. it's locked
                    };
                    let want = Piece::complement(&peer.deref().state.pieces, &exclude);
                    let req_piece = match want.len() {
                        0 => continue,
                        _ => slice_piece(&want, &self.piece_length, &BLOCK_LENGTH)
                    };

                    self.requests.push((req_piece.clone(), time::get_time().sec));

                    match Piece::add_to_boundary_vec(&mut exclude, req_piece.clone()) {
                        Ok(index) => {
                            Piece::compact_if_possible(&mut exclude, index);

                            let message = Message::Request{
                                index: req_piece.start.index as u32,
                                begin: req_piece.start.offset as u32,
                                length: req_piece.num_bytes(&self.piece_length) as u32/*BLOCK_LENGTH as u32*/
                            };

                            peer_socket.send_message(message);

                        },
                        Err(e) => {
                            println!("owned: {:?}, requests: {:?}", self.owned_pieces, self.requests);
                            panic!(e);
                        }
                    }
                }
            }
        }

        if self.requests.len() >= WANT_LIMIT {

            let mut num_to_expire = WANT_LIMIT/EXPIRE_FACTOR;
            let now = time::get_time().sec;
            self.requests.retain(|&(ref x, ref y)| {
                if *y < now - 18 && num_to_expire > 0 {
                    num_to_expire -= 1;
                    false
                }
                else {
                    true
                }
            });
        }
    }
}

fn slice_piece (pieces: &[Piece], piece_length: &usize, block_size: &usize) -> Piece {
    let &Piece {
        ref start,
        ref end
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
    fn handle(&mut self, message: &Self::MessageType, peer: &mut Peer, global_state: &mut GlobalState);
}

/// The default algorithm
impl Handler for DefaultHandler {
    type MessageType = Message;
    #[inline]
    fn handle (&mut self, message: &Message, peer: &mut Peer, global: &mut GlobalState) {
        println!("{:?}", message);
        match message {
            &Message::Have{piece_index: index} => {
                let i = index as usize;
                global.gpc_incr(i, 1);
                //peer.state.set_have(i);
                let piece = Piece::from(global.piece_length, i, 0, global.piece_length);
                match Piece::add_to_boundary_vec(&mut peer.state.pieces, piece) {
                    Ok(i_index) => Piece::compact_if_possible(&mut peer.state.pieces, i_index),
                    Err(e) => {
                        panic!(e);
                    }
                }

            },
            &Message::Choke => {
                peer.state.set_us_choked(true);
            },
            &Message::Unchoke => {
                peer.state.set_us_choked(false);
            },
            &Message::Interested => {
                peer.state.set_us_interested(true);
            },
            &Message::Bitfield(ref bitfield) => {
                for (index, byte) in bitfield.iter().enumerate() {
                    for i in 0..8 { //cast up so i don't have to deal with overflows
                        let n = 1 & (((*byte as u16) << i) >> (8-i));
                        global.gpc_incr(index*8+i, n);
                    }
                }
                peer.state.set_pieces_from_bitfield(&bitfield);
                //peer.state.set_bitfield(bitfield);
            },
            _ => {}
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
