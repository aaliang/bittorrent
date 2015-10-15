extern crate bencode;
extern crate rand;
extern crate bittorrent;
extern crate hyper;

use std::env;
use std::io::Read;
use std::collections::HashMap;
use bencode::{deserialize, deserialize_file, Bencode, TypedMethods, BencodeVecOption};
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

fn ping_tracker (announce: String, args: Vec<(&str, String)>) -> Option<HashMap<String, Bencode>> {
    let req_addr = announce + "?" + &QueryString::from(args).query_string();
    let client = Client::new();
    let mut res = client.get(&req_addr)
                        .header(Connection::close())
                        .send().unwrap();
    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    println!("dict: {:?}", body.iter().map(|a| *a as char).collect::<Vec<char>>());
    println!("{}", body.iter().map(|a| *a as char).collect::<String>());

    deserialize(body).to_singleton_dict()
}

fn get_peers <T> (tracker_response: &T) -> Vec<(String, u32)> where T:TypedMethods {
    let peers = tracker_response.get_owned_string("peers").unwrap();
    //for now keep the bottom unused value, it's for ipv6. which maybe will be addressed
    let peers6 = tracker_response.get_owned_string("peers6").unwrap();
    let peers_bytes = peers.chars().map(|x| x as u8).collect::<Vec<u8>>();

    (0..peers_bytes.len()/6).map(|x| {
        let ip_start = x * 6;
        let ip_end = ip_start + 4;
        //returns a 2-ple of addresses (as string) and port (as u32)
        ((&peers_bytes[ip_start..ip_end]).iter()
                                            .map(|y| y.to_string())
                                            .collect::<Vec<String>>()
                                            .join("."),
            peers_bytes[ip_end] as u32 + peers_bytes[ip_end+1] as u32)
    }).collect::<Vec<(String, u32)>>()
}

fn init (metadata: Metadata, listen_port: u32, bytes_dled: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let bytes_left = metadata.get_total_length() - bytes_dled;

    let response = ping_tracker(metadata.announce, vec![
                                ("info_hash", metadata.info_hash),
                                ("peer_id", peer_id),
                                ("port", listen_port.to_string()),
                                ("uploaded", 0.to_string()),
                                ("downloaded", bytes_dled.to_string()),
                                ("left", bytes_left.to_string()),
                                ("compact", 1.to_string()),
                                ("event", "started".to_string())
                                ]);

    let tracker_resp = match response {
        Some(a) => a,
        None => panic!("no valid bencode response from tracker")
    };

    let peers = get_peers(&tracker_resp);

    println!("{:?}", peers);
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
