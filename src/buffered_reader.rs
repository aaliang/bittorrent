#![feature(split_off)]

use std::io::Read;
use std::io::Result;
use bt_messages::{Message, try_decode};

pub struct BufferedReader <T> where T: Read {
    readable: T,
    spare: Vec<u8>
}

impl <T> BufferedReader <T> where T:Read {
    pub fn new (readable: T, spare: Vec<u8>) -> BufferedReader <T> {
        BufferedReader {
            readable: readable,
            spare: spare
        }
    }

    pub fn wait_for_message(&mut self) -> Result<Message> {
        let mut buffer:Vec<u8> = Vec::new();
        loop {
            let mut i_buff = [0; 512];
            match self.readable.read(&mut i_buff) {
                Ok(0) => continue,
                Ok(bytes_read) => {
                    buffer.extend(i_buff[0..bytes_read].iter());
                    if buffer.len() >= 4 {
                        //try_decode calls read which is a blocking operation, should probably
                        //combine bt_messages with this
                        match try_decode(&buffer) {
                            None => continue,
                            Some((protocol_message, bytes_consumed)) => {
                                //TODO: might be able to use self.spare as a slice
                                self.spare = (&self.spare[bytes_consumed..]).to_owned();
                                return Ok(protocol_message)
                            }
                        }
                    }
                },
                Err(err) => return Err(err)
            };
        }
    }
}
