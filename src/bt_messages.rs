use std::io::Read;
use std::io::Result;

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

///len_prefix is big endian
///might want to use traits instead of returning an enum... haven't decided yet. would save a match
pub fn decode_message <T> (len_prefix: &[u8; 4], stream: &mut T) -> Message where T:Read {
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
fn u8_2_to_u16 (bytes: &[u8; 2]) -> u16 {
    (bytes[1] as u16 | (bytes[0] as u16) << 8)
}

fn u8_4_to_u32 (bytes: &[u8; 4]) -> u32 {
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
