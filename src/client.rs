use std::io::prelude::*;
use std::net::TcpStream;
use std::str;

fn main () {
    let mut stream = TcpStream::connect("127.0.0.1:3451").unwrap();

    //let _ = stream.write(&[1]);
    //
    let mut buffer = &mut[0; 128];
    let _ = stream.read(buffer);
    let s = match str::from_utf8(buffer) {
        Ok(v) => v,
        Err(e) => panic!("Not a UTF-8 String: {}", e)
    };

    println!("{}", s);
}
