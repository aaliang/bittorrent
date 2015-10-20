extern crate bencode;
extern crate rand;
extern crate bittorrent;
extern crate hyper;

use std::{env, str, thread};
use std::io::{Read, Write};
use std::collections::HashMap;
use std::sync::Arc;
use std::net::{Ipv4Addr, TcpStream, SocketAddrV4};
use bencode::{deserialize, deserialize_file, Bencode, TypedMethods, BencodeVecOption};
use rand::{Rng, thread_rng};
use bittorrent::querystring::QueryString;
use bittorrent::metadata::{MetadataDict, Metadata};
use hyper::Client;
use hyper::header::Connection;

const PEER_ID_LENGTH:usize = 20;
const PEER_ID_PREFIX:&'static str = "-TR1000-";

#[derive(Debug)]
enum Address {
    TCP(Ipv4Addr, u16)
}

fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = thread_rng().gen_ascii_chars()
                           .take(rand_length)
                           .collect::<String>();

    prefix.to_string() + &rand
}

fn ping_tracker (announce: &String, args: Vec<(&str, String)>) -> Option<HashMap<String, Bencode>> {
    let req_addr = announce.to_string() + "?" + &QueryString::from(args).query_string();
    println!("pinging tracker {}", req_addr);
    let client = Client::new();
    let mut res = client.get(&req_addr)
                        .header(Connection::close())
                        .send().unwrap();
    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    deserialize(body).to_singleton_dict()
}

fn get_peers <T> (tracker_response: &T) -> Vec<Address> where T:TypedMethods {
    let peers = tracker_response.get_owned_string("peers").unwrap_or_else(||panic!("no peers found"));
    //for now keep the bottom unused value, it's for ipv6. which maybe will be addressed
    (0..peers.len()/6).map(|x| {
        let ip_start = x * 6;
        let ip_end = ip_start + 4;
        let ip_bytes = &peers[ip_start..ip_end];
        let ip = Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);
        let port = (peers[ip_end] as u16)*256 + peers[ip_end+1] as u16;
        Address::TCP(ip, port)
    }).collect::<Vec<Address>>()
}

/// The peer handshake message, according to protocol
///
fn to_handshake (pstr:&str, info_hash: &[u8; 20], peer_id: &String) -> Vec<u8> {
    let reserved = [0u8; 8];
    let pstr_bytes = pstr.to_string().into_bytes();
    let a = [pstr_bytes.len() as u8];
    let b = pstr_bytes;
    let c = reserved;

    println!("info_hash: {:?}", info_hash);
    println!("peer_id: {}", peer_id);
    let d = info_hash.clone();
    let e = peer_id.clone().into_bytes();

    let hs = [a.iter(),
     b.iter(),
     c.iter(),
     d.iter(),
     e.iter()].iter().flat_map(|y| {
         y.to_owned().map(|x| *x).collect::<Vec<u8>>()
     }).collect::<Vec<u8>>();

    println!("hs.len(): {}", hs.len());
    println!("{:?}", hs);
    hs
}

fn connect_to_peer(address:Address, metadata: &Metadata, peer_id: &String) {
    println!("connecting to {:?}", address);
    let (ip, port) = match address {
        Address::TCP(ip_address, port) => (ip_address, port)
    };

    let mut stream = match TcpStream::connect(SocketAddrV4::new(ip, port)) {
        Ok(tcp_stream) => tcp_stream,
        _ => panic!("unable to connect to socket")
    };

    println!("connected to {:?}", address);

    let _ = stream.write_all(&to_handshake("BitTorrent protocol", &metadata.info_hash, peer_id));

    stream.flush();

    let mut buffer = Vec::new();
    match stream.read_to_end(&mut buffer) {
        Ok(bytes_read) => println!("bytes consumed: {}", bytes_read),
        Err(a) => println!("{}", a)
    }

    println!("res: {:?}", buffer);
}

fn init (metadata: &Metadata, listen_port: u32, bytes_dled: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let bytes_left = metadata.get_total_length() - bytes_dled;

    let info_hash_escaped = QueryString::encode_component(&metadata.info_hash);
    let response = ping_tracker(&metadata.announce, vec![
                                ("info_hash", info_hash_escaped),
                                ("peer_id", peer_id.clone()),
                                ("port", listen_port.to_string()),
                                ("uploaded", 0.to_string()),
                                ("downloaded", bytes_dled.to_string()),
                                ("left", bytes_left.to_string()),
                                ("compact", 1.to_string()),
                                ("event", "started".to_string()),
                                ("num_want", "15".to_string())
                                ]);

    let tracker_resp = match response {
        Some(a) => a,
        None => panic!("no valid bencode response from tracker")
    };

    println!("tracker resp: {:?}", tracker_resp);

    let peers = get_peers(&tracker_resp);
    println!("{} peers", peers.len());
    let handshake_out = to_handshake("BitTorrent protocol", &metadata.info_hash, &peer_id);

    let mut children = vec![];
    for peer in peers {
        let child_meta = metadata.clone();
        let peer_id = peer_id.clone();
        children.push(thread::spawn(move || {
            connect_to_peer(peer, &child_meta, &peer_id)
        }));
        //connect_to_peer(peer, &metadata, &peer_id);
    }

    for child in children {
        let _ = child.join();
    }
}

fn main () {
    let path = env::args().nth(1)
                          .unwrap_or_else(||panic!("no path to torrent provided"));

    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));

    assert_eq!(content.len(), 1);

    let metadata = match content.first() {
        Some(&Bencode::Dict(ref dict)) => {
            //println!("{:?}", dict);
            dict.to_metadata()
        },
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    //println!("{:?}", metadata);
    init(&metadata, 6887, 0);
}
