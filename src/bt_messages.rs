use std::io::Read;
use std::io::Result;

pub enum Message {
    KeepAlive,

}

///len_prefix is big endian
///might want to use traits instead of returning an enum... haven't decided yet. would save a match
pub fn decode_message <T> (len_prefix: &[u8; 4], stream: &mut T) -> Message where T:Read {
    let i: u32 = (
        len_prefix[3] as u32
        | ((len_prefix[2] as u32) << 8)
        | ((len_prefix[1] as u32) << 16)
        | ((len_prefix[0] as u32) << 24));
    match i {
        0 => Message::KeepAlive,
        _ => {
            let mut id_buf = [0; 1];
            let _ = stream.read(&mut id_buf);
            let id = id_buf[0];

            println!("temp");
            Message::KeepAlive
        }
    }
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
