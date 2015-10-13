extern crate bencode;
extern crate crypto;

use std::env;
use std::collections::HashMap;
use bencode::{deserialize_file, Bencode, TypedMethods, BencodeToString};
use crypto::sha1::Sha1;
use crypto::digest::Digest;

#[derive(Debug)]
struct Metadata {
    announce: String,
    info_hash: String
}

trait MetadataDict {
    fn to_metadata (&self) -> Option<Metadata>;
}

impl MetadataDict for HashMap<String, Bencode> {
    /// Extracts information from this HashMap into a Metadata instance, if valid. Currently if it
    /// is invalid, it will just throw a runtime exception
    fn to_metadata (&self) -> Option<Metadata> {
        let announce = self.get_string("announce").unwrap();
        let info_raw = self.get("info").unwrap().to_bencode_string();
        let mut sha = Sha1::new();
        sha.input_str(&info_raw);

        let info_hash = sha.result_str();
        Some(Metadata {
            announce: announce.clone(),
            info_hash: info_hash.to_string()
        })
    }
}

fn connect_to_tracker(announce: String) {

}

fn main () {
    let path = env::args().nth(1).unwrap_or_else(||panic!("no path to torrent provided"));
    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));
    let metadata = match content.first() {
        Some(&Bencode::Dict(ref x)) => x.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    println!("{:?}", metadata);
}
