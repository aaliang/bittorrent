use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    println!("new conn");
    stream.write(b"Hello World\r\n").unwrap();
}

fn main () {
    let listener = TcpListener::bind("0.0.0.0:3451").unwrap();
    println!("listening");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move|| {
                    handle_client(stream)
                });
            }
            Err(e) => println!("connection failed!")
        }
    }
}
