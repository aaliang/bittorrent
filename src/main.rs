extern crate bencode;

use std::env;
use bencode::deserialize_file;

fn main () {
    let path = env::args().nth(1).unwrap();
    let cont = deserialize_file(path).unwrap();
    println!("{:?}", cont);
}
