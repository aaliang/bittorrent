use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddrV4};
use rand::{Rng, thread_rng};
use metadata::Metadata;
use buffered_reader::BufferedReader;
use bt_messages::Message;
use tracker::{Address, PEER_ID_LENGTH};
use chunk::Piece;

/// Contains functionality required to setup and exchange messages with a peer

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: String,
    pub state: State
}

impl Peer {
    pub fn new (id:String) -> Peer {
        Peer {
            id: id,
            state: State::new()
        }
    }
}

pub trait SendPeerMessage:Write {
    fn send_message(&mut self, message:Message) {
        let as_bytes = message.to_byte_array();
        let _ = self.write_all(&as_bytes);
    }
}

impl SendPeerMessage for TcpStream {}

#[derive(Debug, Clone)]
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
    pub bitfield: Vec<u8>,

    pub pieces: Vec<Piece>
}

impl State {
    pub fn new () -> State {
        State {
            us_choked: true,
            us_interested: false,
            is_choked: true,
            is_interested: false,
            bitfield: vec![],
            pieces: vec![]
        }
    }

    pub fn set_us_interested (&mut self, us_interested: bool) {
        self.us_interested = us_interested;
    }

    pub fn set_bitfield (&mut self, bitfield: Vec<u8>) {
        self.bitfield = bitfield;
    }

    pub fn set_pieces_from_bitfield (&mut self, bitfield: &[u8]) {
        self.pieces = Piece::convert_bitfield_to_piece_vec(bitfield);
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
    let bitfield_len = bitfield.len();
    if chunk_index+1 > bitfield_len {
        bitfield.extend((0..chunk_index+1-bitfield_len).map(|_| 0));
    }

    bitfield[chunk_index] = bitfield[chunk_index] | chunk_mask;
}

pub fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = thread_rng().gen_ascii_chars()
                           .take(rand_length)
                           .collect::<String>();

    prefix.to_string() + &rand
}

pub fn decode_handshake(resp: &[u8]) -> (&[u8], &[u8], &[u8], &[u8], &[u8]){
    //i realize this is the goofiest looking block ever... but you cant really destructure a
    //vector so i'm sticking with the tuples for now. maybe i'll make it look normal later
    let (pstrlen, a0) = {
        let (l, r) = resp.split_at(1 as usize);
        (l[0], r)
    };
    let (protocol, a1) = a0.split_at(pstrlen as usize);
    let (reserved, a2) = a1.split_at(8);
    let (info_hash, a3) = a2.split_at(20);
    let (peer_id, remainder) = a3.split_at(20);

    (protocol, reserved, info_hash, peer_id, remainder)
}

//this seems overly verbose (the signature)
pub fn connect_to_peer (address: Address, metadata: &Metadata, peer_id: &String) -> Result<(Vec<u8>, BufferedReader<TcpStream>), String> {
    println!("connecting to {:?}", address);
    let (ip, port) = match address {
        Address::TCP(ip_address, port) => (ip_address, port)
    };

    let mut stream = match TcpStream::connect(SocketAddrV4::new(ip, port)) {
        Ok(tcp_stream) => tcp_stream,
        Err(_) => return Err(format!("unable to connect to peer {:?}", ip))
    };

    println!("connected to {:?}", address);

    let _ = stream.write_all(&to_handshake("BitTorrent protocol", &metadata.info_hash, peer_id));
    let _ = stream.flush();

    //for now enforce a maximum handshake size of 512 bytes
    let mut buffer = [0; 512];
    match stream.read(&mut buffer) {
        Ok(0) => Err(format!("invalid handshake from peer")),
        Ok(bytes_read) => {
            let (protocol, _, info_hash, peer_id, rest) = decode_handshake(&buffer[0..bytes_read]);
            match (protocol, info_hash) {
                (b"BitTorrent protocol", i_h) if i_h == metadata.info_hash => {
                    Ok((peer_id.to_owned(), BufferedReader::new(stream, rest.to_vec())))
                },
                _ => Err(format!("invalid peer handshake"))
            }
        },
        Err(_) => Err(format!("unable to read from peer {:?}", ip))
    }
}


/// The peer handshake message, according to protocol
///
fn to_handshake (pstr:&str, info_hash: &[u8; 20], peer_id: &String) -> Vec<u8> {
    let reserved = [0u8; 8];
    let pstr_bytes = pstr.to_string().into_bytes();
    let a = [pstr_bytes.len() as u8];
    let b = pstr_bytes;
    let c = reserved;
    let d = info_hash;
    let e = peer_id.clone().into_bytes();

    [a.iter(),
     b.iter(),
     c.iter(),
     d.iter(),
     e.iter()].iter().flat_map(|y| {
         y.to_owned().map(|x| *x).collect::<Vec<u8>>()
     }).collect::<Vec<u8>>()
}

