extern crate bencode;
extern crate rand;
extern crate bittorrent;

use std::{env, thread};
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddrV4};
use std::sync::mpsc::channel;
use bencode::{deserialize_file, Bencode};
use rand::{Rng, thread_rng};
use bittorrent::metadata::{MetadataDict, Metadata};
use bittorrent::buffered_reader::BufferedReader;
use bittorrent::tracker::{Address, get_http_tracker_peers, PEER_ID_PREFIX, PEER_ID_LENGTH};

fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = thread_rng().gen_ascii_chars()
                           .take(rand_length)
                           .collect::<String>();

    prefix.to_string() + &rand
}

/// The peer handshake message, according to protocol
///
fn to_handshake (pstr:&str, info_hash: &[u8; 20], peer_id: &String) -> Vec<u8> {
    let reserved = [0u8; 8];
    let pstr_bytes = pstr.to_string().into_bytes();
    let a = [pstr_bytes.len() as u8];
    let b = pstr_bytes;
    let c = reserved;
    let d = info_hash;
    let e = peer_id.clone().into_bytes();

    [a.iter(),
     b.iter(),
     c.iter(),
     d.iter(),
     e.iter()].iter().flat_map(|y| {
         y.to_owned().map(|x| *x).collect::<Vec<u8>>()
     }).collect::<Vec<u8>>()
}

fn decode_handshake(resp: &[u8]) -> (&[u8], &[u8], &[u8], &[u8], &[u8]){
    //i realize this is the goofiest looking block ever... but you cant really destructure a
    //vector so i'm sticking with the tuples for now. maybe i'll make it look normal later
    let (pstrlen, a0) = {
        let (l, r) = resp.split_at(1 as usize);
        (l[0], r)
    };
    let (protocol, a1) = a0.split_at(pstrlen as usize);
    let (reserved, a2) = a1.split_at(8);
    let (info_hash, a3) = a2.split_at(20);
    let (peer_id, remainder) = a3.split_at(20);

    (protocol, reserved, info_hash, peer_id, remainder)
}

//this seems overly verbose (the signature)
fn connect_to_peer (address: Address, metadata: &Metadata, peer_id: &String) -> Result<(Vec<u8>, BufferedReader<TcpStream>), String> {
    println!("connecting to {:?}", address);
    let (ip, port) = match address {
        Address::TCP(ip_address, port) => (ip_address, port)
    };

    let mut stream = match TcpStream::connect(SocketAddrV4::new(ip, port)) {
        Ok(tcp_stream) => tcp_stream,
        Err(_) => return Err(format!("unable to connect to peer {:?}", ip))
    };

    println!("connected to {:?}", address);

    let _ = stream.write_all(&to_handshake("BitTorrent protocol", &metadata.info_hash, peer_id));
    let _ = stream.flush();

    //for now enforce a maximum handshake size of 512 bytes
    let mut buffer = [0; 512];
    match stream.read(&mut buffer) {
        Ok(0) => Err(format!("invalid handshake from peer")),
        Ok(bytes_read) => {
            let (protocol, _, info_hash, peer_id, rest) = decode_handshake(&buffer[0..bytes_read]);
            match (protocol, info_hash) {
                (b"BitTorrent protocol", i_h) if i_h == metadata.info_hash => {
                    Ok((peer_id.to_owned(), BufferedReader::new(stream, rest.to_vec())))
                },
                _ => Err(format!("invalid peer handshake"))
            }
        },
        Err(_) => Err(format!("unable to read from peer {:?}", ip))
    }
}

fn init (metadata: &Metadata, listen_port: u32, bytes_dled: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let peers = match get_http_tracker_peers(&peer_id, metadata, listen_port, bytes_dled) {
        Some(peers) => peers,
        _ => panic!("cannot get peers from tracker")
    };

    println!("got {} peers", peers.len());

    let (tx, rx) = channel();

    //TODO: the sink initialization should be done outside of init() as one sink can realistically
    //handle multiple torrents relatively trivially
    //all threads will send messages asynchronously to the sink. tbh the loop could be shoved onto the main
    //thread, but for the sake of modularity lets dedicated one for the sink
    let sink = thread::spawn(move || {
        loop {
            let res = rx.recv().unwrap();
            println!("{:?}", res);
        }
    });

    for peer in peers {
        let child_meta = metadata.clone();
        let peer_id = peer_id.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            match connect_to_peer(peer, &child_meta, &peer_id) {
                Ok((ref peer_id, ref mut reader)) => {
                    match reader.wait_for_message() {
                        Ok(message) => {
                            tx.send((message));
                        },
                        e @ Err(_) => {
                            println!("error waiting for message");
                        }
                    }
                },
                Err(e) => {
                    println!("{:?}", e);
                }
            };
        });
    }

    //block until the sink shuts down
    let _ = sink.join();
}

fn start_torrenting () {
    let path = env::args().nth(1)
                          .unwrap_or_else(||panic!("no path to torrent provided"));

    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));

    assert_eq!(content.len(), 1);

    let metadata = match content.first() {
        Some(&Bencode::Dict(ref dict)) => dict.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    init(&metadata, 6887, 0);
}

fn main () {
    start_torrenting();
    //test();
}
