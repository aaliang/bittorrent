extern crate bencode;

use std::env;
use std::collections::HashMap;
use bencode::{deserialize_file, Bencode, TypedMethods};

#[derive(Debug)]
struct Metadata {
    announce: String,
}

trait MetadataDict {
    fn to_metadata (&self) -> Option<Metadata>;
}

impl MetadataDict for HashMap<String, Bencode> {
    /// Extracts information from this HashMap into a Metadata instance, if valid. Currently if it
    /// is invalid, it will just throw a runtime exception
    fn to_metadata (&self) -> Option<Metadata> {
        let announce = self.get_string("announce").unwrap();
        Some(Metadata {
            announce: announce.clone(),
        })
    }
}

fn main () {
    let path = env::args().nth(1).unwrap_or_else(||panic!("no path to torrent provided"));
    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));
    let metadata = match content.first() {
        Some(&Bencode::Dict(ref x)) => x.to_metadata(),
        _ => panic!("no valid information in torrent file")
    };

    println!("{:?}", metadata.unwrap());
}
