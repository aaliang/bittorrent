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
                        let option = try_decode(&buffer);
                    }
                },
                Err(err) => return Err(err)
            };
        }
        Ok(Message::Choke)
    }
}
