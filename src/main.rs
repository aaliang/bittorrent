extern crate bencode;
extern crate rand;
extern crate bittorrent;

use std::{env, thread};
use std::sync::mpsc::channel;
use bencode::{deserialize_file, Bencode};
use bittorrent::metadata::{MetadataDict, Metadata};
use bittorrent::bt_messages::Message;
use bittorrent::tracker::{get_http_tracker_peers, PEER_ID_PREFIX, PEER_ID_LENGTH};
use bittorrent::peer::{connect_to_peer, gen_rand_peer_id};

/// Handles messages. This is a cheap way to force reactive style
trait Handler {
    type MessageType;
    fn handle(message: Self::MessageType);
}

struct DefaultHandler;
/// The default algorithm
impl Handler for DefaultHandler {
    type MessageType = Message;
    fn handle (message: Message) {
        match message {
            _ => println!("{:?}", message)
        }
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
            //for now handle all messages on a single thread. this is similar to the actor pattern
            DefaultHandler::handle(rx.recv().unwrap());
        }
    });

    for peer in peers {
        let child_meta = metadata.clone();
        let peer_id = peer_id.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            match connect_to_peer(peer, &child_meta, &peer_id) {
                Ok((ref peer_id, ref mut reader)) => {
                    loop {
                        match reader.wait_for_message() {
                            Ok(message) => {
                                tx.send((message));
                            },
                            e @ Err(_) => {
                                println!("error waiting for message");
                            }
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
}
