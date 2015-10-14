extern crate bencode;
extern crate rand;
extern crate bittorrent;

use std::env;
use bencode::{deserialize_file, Bencode};
use rand::{Rng, thread_rng};
use bittorrent::querystring::QueryString;
use bittorrent::metadata::{MetadataDict, Metadata};

const PEER_ID_LENGTH:usize = 20;
const PEER_ID_PREFIX:&'static str = "ABT:";

fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = thread_rng().gen_ascii_chars()
                           .take(rand_length)
                           .collect::<String>();

    prefix.to_string() + &rand
}

fn init (metadata: Metadata, listen_port: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let req_params = QueryString::from(vec![
                                           ("info_hash", metadata.info_hash),
                                           ("peer_id", peer_id),
                                           ("port", listen_port.to_string()),
                                           ("uploaded", 0.to_string()),
                                           ("downloaded", 0.to_string())
                                           ]).query_string();
    println!("params: {}", req_params);
}

fn main () {
    let path = env::args()
                    .nth(1)
                    .unwrap_or_else(||panic!("no path to torrent provided"));

    let content = deserialize_file(path)
                    .unwrap_or_else(||panic!("unable to parse bencoded metadata"));

    let metadata = match content.first() {
        Some(&Bencode::Dict(ref dict)) => dict.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    println!("{:?}", metadata);

    init(metadata, 6888);
}
