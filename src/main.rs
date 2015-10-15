extern crate bencode;
extern crate rand;
extern crate bittorrent;
extern crate hyper;

use std::env;
use std::io::Read;
use bencode::{deserialize, deserialize_file, Bencode, TypedMethods};
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

    //println!("{}", req_addr);

    let client = Client::new();
    let mut res = client.get(&req_addr)
                        .header(Connection::close())
                        .send().unwrap();

    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    println!("dict: {:?}", body.iter().map(|a| *a as char).collect::<Vec<char>>());

    println!("{}", body.iter().map(|a| *a as char).collect::<String>());
    let tracker_response = deserialize(body).unwrap();
    let tracker_resp = match tracker_response.first() {
        Some(&Bencode::Dict(ref a)) => a,
        _ => panic!("unable to parse dictionary from tracker response!")
    };
    let peers = tracker_resp.get_owned_string("peers").unwrap();
    let peers6 = tracker_resp.get_owned_string("peers6").unwrap();

    let peers_bytes = peers.chars().map(|x| x as u8).collect::<Vec<u8>>();

    let addresses = (0..peers_bytes.len()/6).map(|x| {
        let ip_start = x * 6;
        let ip_end = ip_start + 4;
        //returns a 2-ple of addresses (as string) and port (as u32)
        ((&peers_bytes[ip_start..ip_end]).iter()
                                            .map(|y| y.to_string())
                                            .collect::<Vec<String>>()
                                            .join("."),
            peers_bytes[ip_end] as u32 + peers_bytes[ip_end+1] as u32)
    }).collect::<Vec<(String, u32)>>();

    println!("{:?}", addresses);
    println!("Response: {:?}", tracker_response);
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
