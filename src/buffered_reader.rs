use std::io::Read;
use std::io::Result;
use std::io::{Error, ErrorKind};
use std::net::TcpStream;
use bt_messages::{Message, try_decode};

pub struct BufferedReader <T> where T: Read {
    readable: T,
    buffer: Vec<u8>
}

impl <T> BufferedReader <T> where T:Read {
    pub fn new (readable: T, buffer: Vec<u8>) -> BufferedReader <T> {
        BufferedReader {
            readable: readable,
            buffer: buffer
        }
    }

    pub fn wait_for_message(&mut self) -> Result<Message> {
        loop {
            let mut i_buff = [0; 512];
            match self.readable.read(&mut i_buff) {
                Ok(0) => return Err(Error::new(ErrorKind::Other, "graceful disconnect")),
                Ok(bytes_read) => {
                    self.buffer.extend(i_buff[0..bytes_read].iter());
                    if self.buffer.len() >= 4 {
                        //try_decode calls read which is a blocking operation, should probably
                        //combine bt_messages with this
                        match try_decode(&self.buffer) {
                            None => continue,
                            Some((protocol_message, bytes_consumed)) => {
                                //TODO: might be able to use self.spare as a slice
                                self.buffer = (&self.buffer[bytes_consumed..]).to_owned();
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

impl BufferedReader <TcpStream> {
    pub fn clone_stream (&self) -> TcpStream {
        self.readable.try_clone().unwrap()
    }
}
