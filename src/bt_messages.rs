use std::mem::transmute;

#[derive(Debug)]
pub enum Message {
    //peer messages according to protocol
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
    //internally used messages
    Handshake,
    Unhandled,
}

//this is kind of lazy. but it also makes reading/writing Messages symmetrical...
impl Message {
    fn to_byte_array (&self) -> Vec<u8> {
        match self {
            &Message::KeepAlive => {
                vec![0, 0, 0, 0]
            }
            &Message::Choke => {
                vec![0, 0, 0, 1, 0]
            },
            &Message::Unchoke => {
                vec![0, 0, 0, 1, 1]
            },
            &Message::Interested => {
                vec![0, 0, 0, 1, 2]
            },
            &Message::Have{piece_index: p} => {
                let r: [u8; 4] = unsafe {transmute(p.to_be())};
                vec![0, 0, 0, 5, r[0], r[1], r[2], r[3]]
            },
            _ => {
                vec![]
            }

        }
    }
}

/// Tries to decode a message according to the bittorrent protocol from a slice of bytes
/// If it is not complete, it will return None
/// If successful it will return a tuple enveloping the message in deserialized form and the number
/// of bytes that it consumed. n.b. done immutably - the caller will need to advance the pointer
///
/// # Preconditions
/// ```assert(bytes.len() > 3)```
///
pub fn try_decode (bytes: &[u8]) -> Option<(Message, usize)> {
    //println!("tcp buffer: {:?}", bytes);
    //yes there are some magic numbers floating around in here... but they're byte manipulations
    let rest = &bytes[4..];
    match u8_4_to_u32(&bytes[0..4]) as usize {
        0 => Some((Message::KeepAlive, 4)),
        len => { //len is inclusive of the id byte
            let message_type = match rest.first() {
                None => return None,
                Some(a) => a
            };
            let message = match *message_type {
                _ if len > rest.len() => return None, //the entire envelope is not here yet
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

            //successfully consume the len + the 4 byte length value
            Some((message, len + 4))

        }
    }
}

//this is relatively unsafe
fn u8_2_to_u16 (bytes: &[u8]) -> u16 {
    (bytes[1] as u16 | (bytes[0] as u16) << 8)
}

fn u8_4_to_u32 (bytes: &[u8]) -> u32 {
    (bytes[3] as u32
        | ((bytes[2] as u32) << 8)
        | ((bytes[1] as u32) << 16)
        | ((bytes[0] as u32) << 24))
}

#[test]
fn test_have_message () {
    let a_message = Message::Have{piece_index: 400};

    let a = a_message.to_byte_array();

    assert_eq!(a, vec![0, 0, 0, 5, 0, 1, 0, 0]);

}

/*#[test]
fn test_decode () {
    use std::io::{Result, Read};

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
    try_decode(&buf);
}*/
