extern crate bencode;

use std::env;
use std::collections::HashMap;
use bencode::{deserialize_file, Bencode};

#[derive(Debug)]
struct Metadata {
    announce: String,
}

trait ToMetadata {
    fn to_metadata (&self) -> Option<Metadata>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_string(&self, key: &str) -> Option<String>;
}

impl ToMetadata for HashMap<String, Bencode> {
    fn to_metadata (&self) -> Option<Metadata> {
        let announce = self.get_string("announce").unwrap();
        Some(Metadata {
            announce: announce,
        })
    }

    fn get_int (&self, key: &str) -> Option <i64> {
        match self.get(key) {
            Some(&Bencode::Int(a)) => Some(a),
            _ => None
        }
    }

    fn get_string (&self, key: &str) -> Option <String> {
        match self.get(key) {
            Some(&Bencode::String(ref a)) => Some(a.clone()),
            _ => None
        }
    }

}

fn main () {
    let path = env::args().nth(1).unwrap();
    let content = deserialize_file(path).unwrap();
    let metadata = match *content.first().unwrap() {
        Bencode::Dict(ref x) => {
             x.to_metadata()
        },
        _ => panic!("something")
    };

    println!("{:?}", metadata.unwrap());
}
