use std::net::{Ipv4Addr};
use std::collections::HashMap;
use std::io::Read;
use hyper::Client;
use hyper::header::Connection;
use bencode::{deserialize, Bencode, BencodeVecOption, TypedMethods};
use metadata::{Metadata};
use querystring::QueryString;

/// Contains functionality required to connect and parse tracker responses

pub const PEER_ID_LENGTH:usize = 20;
pub const PEER_ID_PREFIX:&'static str = "-TR1000-";

//Address doesn't exactly belong here
#[derive(Debug)]
pub enum Address {
    TCP(Ipv4Addr, u16)
}

pub fn get_http_tracker_peers (peer_id: &String, metadata: &Metadata, listen_port:u32, bytes_dled: u32) -> Option<Vec<Address>> {
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
                                ("num_want", 15.to_string())
                                ]);

    match response {
        Some(resp) => Some(get_peers(&resp)),
        None => panic!("no valid bencode response from tracker")
    }
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

    deserialize(&body).to_singleton_dict()
}

/// Gets peer addresses from a received tracker response. These are just Ipv4 addresses currently
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
