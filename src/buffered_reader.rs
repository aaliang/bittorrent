use std::io::Read;
use std::io::Result;
use bt_messages::{Message, try_decode};

struct BufferedReader <'a, T> where T: Read {
    readable: T,
    spare: &'a [u8]
}

impl <'a, T> BufferedReader <'a, T> where T:Read {

    fn new (readable: T, spare: &[u8]) -> BufferedReader <T> {
        BufferedReader {
            readable: readable,
            spare: spare
        }
    }

    fn wait_for_message(&mut self) -> Result<Message> {
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
