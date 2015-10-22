use std::io::Read;
use std::io::Result;

#[derive(Debug)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {piece_index: u32},
    Bitfield(Vec<u8>),
    Request {index: u32, begin: u32, length: u32},
    Piece {index: u32, begin: u32, block: Vec<u8>},
    Cancel {index: u32, begin: u32, length: u32},
    Port(u16),
    Unhandled
}

/// Tries to decode a message according to the bittorrent protocol from a slice of bytes
/// If it is not complete, it will return None
///
/// # Preconditions
/// ```assert(bytes.len() > 3)```
///
pub fn try_decode (bytes: &[u8]) -> Option<Message> {
    let rest = &bytes[4..];
    match u8_4_to_u32(&bytes[0..4]) {
        0 => Some(Message::KeepAlive),
        len => { //len is inclusive of the id byte
            let rest_len = rest.len();
            let message_type = match rest.first() {
                None => return None,
                Some(a) => a
            };
            let message = match *message_type {
                _ if len > rest.len() as u32 => return None, //the entire envelope is not here yet
                0 => Message::Choke,
                1 => Message::Unchoke,
                2 => Message::Interested,
                3 => Message::NotInterested,
                4 => Message::Have{piece_index: u8_4_to_u32(&rest[1..5])},
                5 => Message::Bitfield((&rest[1..len as usize]).to_owned()),
                6 => {
                    let index = u8_4_to_u32(&rest[1..5]);
                    let begin = u8_4_to_u32(&rest[5..9]);
                    let length = u8_4_to_u32(&rest[9..13]);
                    Message::Request{index: index, begin: begin, length: length}
                },
                7 => {
                    let index = u8_4_to_u32(&rest[1..5]);
                    let begin = u8_4_to_u32(&rest[5..9]);
                    let block = (&rest[9..len as usize]).to_owned();
                    Message::Piece{index: index, begin: begin, block: block}
                },
                8  => {
                    let index = u8_4_to_u32(&rest[1..5]);
                    let begin = u8_4_to_u32(&rest[5..9]);
                    let length = u8_4_to_u32(&rest[9..13]);
                    Message::Cancel{index: index, begin: begin, length: length}
                },
                9 => Message::Port(u8_2_to_u16(&rest[1..3])),
                _ => return None
            };

            Some(message)

        }
    }
}
///len_prefix is big endian
///might want to use traits instead of returning an enum... haven't decided yet. would save a match
pub fn decode_message <T> (len_prefix: &[u8], stream: &mut T) -> Message where T:Read {
    let i = u8_4_to_u32(len_prefix);
    match i {
        0 => Message::KeepAlive,
        len => {
            let mut id_buf = [0; 1];
            let _ = stream.read(&mut id_buf);
            match id_buf[0] {
                0 => Message::Choke,
                1 => Message::Unchoke,
                2 => Message::Interested,
                3 => Message::NotInterested,
                4 => {
                    let piece_index = read_word(stream);
                    Message::Have{piece_index: piece_index}
                },
                5 => {
                    let bitfield_length = len - 1;
                    Message::Bitfield(read_out_variable(stream, bitfield_length as u64))
                },
                6 => {
                    //so much for referential transparency... ah my eyes!
                    let index = read_word(stream);
                    let begin = read_word(stream);
                    let length = read_word(stream);
                    Message::Request{index: index, begin: begin, length: length}
                },
                7 => {
                    let block_length = len - 9;
                    //TODO: for the sake of semi-immutability maybe it would be prettier if we
                    //consume from the stream in one operation, then partition the slice up
                    //appropriately after
                    let index = read_word(stream);
                    let begin = read_word(stream);
                    let block = read_out_variable(stream, block_length as u64);
                    Message::Piece{index: index, begin: begin, block: block}
                },
                8 => {
                    let index = read_word(stream);
                    let begin = read_word(stream);
                    let length = read_word(stream);
                    Message::Cancel{index: index, begin: begin, length: length}
                },
                9 =>{
                    let mut buf = [0; 2];
                    stream.read(&mut buf);
                    let port = u8_2_to_u16(&buf);
                    Message::Port(port)
                }
                _ => Message::Unhandled
            }

        }
    }
}


//the following two definitions feel clunky. can probably genericize over num bytes somehow
fn u8_2_to_u16 (bytes: &[u8]) -> u16 {
    (bytes[1] as u16 | (bytes[0] as u16) << 8)
}

fn u8_4_to_u32 (bytes: &[u8]) -> u32 {
    (bytes[3] as u32
        | ((bytes[2] as u32) << 8)
        | ((bytes[1] as u32) << 16)
        | ((bytes[0] as u32) << 24))
}

//reads into a fixed length u32
fn read_word <T> (stream: &mut T) -> u32 where T:Read {
    let mut buf = [0; 4];
    stream.read(&mut buf);
    u8_4_to_u32(&buf)
}

fn read_out_variable <T> (stream: &mut T, num_bytes: u64) -> Vec<u8> where T:Read {
    let mut buf = Vec::new();
    stream.take(num_bytes).read(&mut buf);
    buf
}

pub fn test () {
    struct MockStream;

    impl Read for MockStream {
        fn read (&mut self, buf: &mut [u8]) -> Result<usize> {
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
            buf[3] = 0;
            Ok(4)
        }
    }

    let mut stream = MockStream;
    let mut buf = [1; 4];
    stream.read(&mut buf);
    decode_message(&buf, &mut stream);
}

#[test]
fn test_decode () {

    struct MockStream;

    impl Read for MockStream {
        fn read (&mut self, buf: &mut [u8]) -> Result<usize> {
            buf[0] = 0;
            buf[1] = 0;
            buf[2] = 0;
            buf[3] = 0;
            Ok(4)
        }
    }

    let mut stream = MockStream;
    let mut buf = [1; 4];
    stream.read(&mut buf);
    decode_message(&buf, stream);
}
