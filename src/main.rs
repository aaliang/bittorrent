extern crate bencode;

use std::env;
use std::collections::HashMap;
use bencode::{deserialize_file, Bencode};

#[derive(Debug)]
struct Metadata {
    announce: String,
}

trait MetadataDict {
    fn to_metadata (&self) -> Option<Metadata>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_string(&self, key: &str) -> Option<&String>;
    fn get_dict(&self, key: &str) -> Option<&HashMap<String, Bencode>>;
    fn get_list(&self, key: &str) -> Option<&Vec<Bencode>>;
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

    fn get_int (&self, key: &str) -> Option <i64> {
        match self.get(key) {
            Some(&Bencode::Int(a)) => Some(a),
            _ => None
        }
    }

    fn get_string (&self, key: &str) -> Option <&String> {
        match self.get(key) {
            Some(&Bencode::String(ref a)) => Some(a),
            _ => None
        }
    }

    fn get_dict (&self, key: &str) -> Option <&HashMap<String, Bencode>> {
        match self.get(key) {
            Some(&Bencode::Dict(ref a)) => Some(a),
            _ => None
        }
    }

    fn get_list (&self, key: &str) -> Option <&Vec<Bencode>> {
        match self.get(key) {
            Some(&Bencode::List(ref a)) => Some(a),
            _ => None
        }
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
