extern crate bencode;
extern crate rand;
extern crate bittorrent;

use std::{env, thread};
use std::thread::JoinHandle;
use std::sync::mpsc::{channel, Sender};
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
    #[inline]
    fn handle (message: Message) {
        match message {
            _ => println!("{:?}", message)
        }
    }
}

/// Sets up a sink pool. it functions as an Actor
fn init () -> (Sender<Message>, JoinHandle<()>) {
    let (tx, rx) = channel();
    let sink = thread::spawn(move|| {
        loop {
            DefaultHandler::handle(rx.recv().unwrap())
        }
    });
    (tx, sink)
}

/// Sets up a transmission based on a single torrent
fn init_torrent (tx: &Sender<Message>, metadata: &Metadata, listen_port: u32, bytes_dled: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let peers = match get_http_tracker_peers(&peer_id, metadata, listen_port, bytes_dled) {
        Some(peers) => peers,
        _ => panic!("cannot get peers from tracker")
    };

    println!("got {} peers", peers.len());

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
                            Err(_) => {
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
}

fn main () {
    let path = env::args().nth(1)
                          .unwrap_or_else(||panic!("no path to torrent provided"));

    let content = deserialize_file(path).unwrap_or_else(||panic!("unable to parse bencoded metadata"));

    assert_eq!(content.len(), 1);

    let metadata = match content.first() {
        Some(&Bencode::Dict(ref dict)) => dict.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    let (tx, sink) = init();

    //for now initialize torrents inline with main
    init_torrent(&tx, &metadata, 6887, 0);

    //block until the sink shuts down
    let _ = sink.join();
}
