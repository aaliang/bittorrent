extern crate bencode;
extern crate bittorrent;

use std::{env, thread};
use std::thread::JoinHandle;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::net::TcpStream;
use std::ops::{Deref, DerefMut};
use bencode::{deserialize_file, Bencode};
use bittorrent::metadata::{MetadataDict, Metadata};
use bittorrent::bt_messages::Message;
use bittorrent::buffered_reader::BufferedReader;
use bittorrent::tracker::{get_http_tracker_peers, PEER_ID_PREFIX};
use bittorrent::peer::{connect_to_peer, gen_rand_peer_id};
use bittorrent::default_handler::{Handler, DefaultHandler, Peer, Action};

// Sets up a sink pool. it functions similarly to an Actor
/// atm, rust doesn't support HKTs
fn init <'a> (mut handler: DefaultHandler) -> (Sender<(Message, Arc<Mutex<Peer>>)>, JoinHandle<()>) {
    let (tx, rx) = channel();
    let sink = thread::spawn(move|| {
        loop {
            let (message, cell): (Message, Arc<Mutex<Peer>>) = rx.recv().unwrap();
            let mut peer_mut_guard = cell.deref().lock().unwrap();
            let mut peer = peer_mut_guard.deref_mut();

            let action = handler.handle(message, peer);

            //peer.chan.send(action);
        }
    });
    (tx, sink)
}

/// Sets up a transmission based on a single torrent
fn init_torrent (tx: &Sender<(Message, Arc<Mutex<Peer>>)>, metadata: &Metadata, listen_port: u32, bytes_dled: u32) {
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
                Ok((peer_id, mut reader)) => {
                    let peer_id_str = peer_id.iter().map(|x| *x as char).collect::<String>();
                    //requests need a response - use a second channel to accomplish this
                    //TODO - actually this is most likely no longer true. keep it around for now
                    //in case i change my mind
                    let (btx, brx) = channel();
                    let arc = Arc::new(Mutex::new(Peer::new(peer_id_str, btx, reader.clone_stream())));
                    loop {
                        //we can't just block read in a loop - we'll never have a chance to send out
                        //outgoing messages over TCP
                        match reader.wait_for_message() {
                            Ok(message) => {
                                let _ = tx.send((message, arc.clone()));
                            },
                            Err(_) => {
                                println!("error waiting for message");
                            }
                        };
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

    let (tx, sink) = init(DefaultHandler::new());

    //for now initialize torrents inline with main
    init_torrent(&tx, &metadata, 6887, 0);

    //block until the sink shuts down
    let _ = sink.join();
}
