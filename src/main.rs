extern crate bencode;
extern crate crypto;
extern crate rand;
extern crate url;

use std::env;
use std::collections::HashMap;
use bencode::{deserialize_file, Bencode, TypedMethods, BencodeToString};
use crypto::sha1::Sha1;
use crypto::digest::Digest;
use rand::{random, Rng};
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

const PEER_ID_LENGTH:usize = 20;
const PEER_ID_PREFIX:&'static str = "ABT:";

fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = rand::thread_rng()
        .gen_ascii_chars()
        .take(rand_length)
        .collect::<String>();

    prefix.to_string() + &rand
}

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

fn get_tracker_response_sync (announce: String) {

}

struct TrackerRequest <'a> {
    params: HashMap<&'a str, String>
}

impl <'a> TrackerRequest <'a> {
    fn new () -> TrackerRequest <'a> {
        TrackerRequest {params: HashMap::new()}
    }

    fn from_params (params: Vec<(&'a str, String)>) -> TrackerRequest <'a> {
        let mut hm = TrackerRequest::new();
        hm.add_params(params);
        hm
    }

    fn add_param (&mut self, name: &'a str, val: String) {
        self.params.insert(name, utf8_percent_encode(&val, DEFAULT_ENCODE_SET));
    }

    fn add_params (&mut self, params: Vec<(&'a str, String)>) {
        for (key, val) in params {
            self.params.insert(key, utf8_percent_encode(&val, DEFAULT_ENCODE_SET));
        }
    }
}

fn main () {
    let path = env::args().nth(1).unwrap_or_else(||panic!("no path to torrent provided"));
    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));
    let metadata = match content.first() {
        Some(&Bencode::Dict(ref x)) => x.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);

    let mut req_params = TrackerRequest::from_params(vec![
                                                     ("info_hash", metadata.info_hash),
                                                     ("peer_id", peer_id)
                                                     ]);

}
