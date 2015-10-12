extern crate combine;

mod bencode;

use std::env;

fn main () {
    let path = env::args().nth(1).unwrap();
    let cont = bencode::deserialize_file(path).unwrap();
    println!("{:?}", cont);
}
