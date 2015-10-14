extern crate bencode;
extern crate rand;
extern crate bittorrent;
extern crate hyper;

use std::env;
use std::io::Read;
use bencode::{deserialize_file, Bencode};
use rand::{Rng, thread_rng};
use bittorrent::querystring::QueryString;
use bittorrent::metadata::{MetadataDict, Metadata};
use hyper::Client;
use hyper::header::Connection;

const PEER_ID_LENGTH:usize = 20;
const PEER_ID_PREFIX:&'static str = "ABT:";

fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = thread_rng().gen_ascii_chars()
                           .take(rand_length)
                           .collect::<String>();

    prefix.to_string() + &rand
}

fn init (metadata: Metadata, listen_port: u32, bytes_dled: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let bytes_left = metadata.get_total_length() - bytes_dled;
    let req_addr = metadata.announce + "?" + &QueryString::from(vec![
                                                                ("info_hash", metadata.info_hash),
                                                                ("peer_id", peer_id),
                                                                ("port", listen_port.to_string()),
                                                                ("uploaded", 0.to_string()),
                                                                ("downloaded", bytes_dled.to_string()),
                                                                ("left", bytes_left.to_string()),
                                                                ("compact", 1.to_string()),
                                                                ("event", "started".to_string())
                                                                ]).query_string();

    let mut client = Client::new();
    let mut res = client.get(&req_addr)
                        .header(Connection::close())
                        .send().unwrap();

    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();


    println!("Response: {:?}", body.iter().map(|x| *x as char).collect::<String>());
}

fn main () {
    let path = env::args().nth(1)
                          .unwrap_or_else(||panic!("no path to torrent provided"));

    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));

    let metadata = match content.first() {
        Some(&Bencode::Dict(ref dict)) => dict.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    //println!("{:?}", metadata);

    init(metadata, 6888, 0);
}
