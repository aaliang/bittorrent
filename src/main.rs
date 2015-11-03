extern crate bencode;
extern crate bittorrent;
extern crate time;

use std::{env, thread};
use std::thread::{JoinHandle};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::ops::{Deref, DerefMut};
use bittorrent::metadata::{Metadata, MetadataDict};
use bencode::{Bencode, deserialize_file};
use bittorrent::bt_messages::Message;
use bittorrent::tracker::{get_http_tracker_peers, PEER_ID_PREFIX};
use bittorrent::peer::{connect_to_peer, gen_rand_peer_id, Peer, SendPeerMessage};
use bittorrent::default_handler::{Handler, DefaultHandler, GlobalState, Spin};

// Sets up a sink pool. it functions similarly to an Actor
/// atm, rust doesn't support HKTs
/// TODO: the Handler now stores state... so some assumptions no longer hold
///
fn init (global_arc: Arc<Mutex<GlobalState>>, mut handler: DefaultHandler) -> (Sender<(Message, Arc<RwLock<Peer>>)>, JoinHandle<()>) {
    let (tx, rx) = channel();
    let sink = thread::spawn(move|| {
        loop {
            let (message, cell): (Message, Arc<RwLock<Peer>>) = rx.recv().unwrap();

            let mut gs_guard = (&global_arc).lock().unwrap();
            {
                let mut peer_mut_guard = cell.deref().write().unwrap();
                let mut peer = peer_mut_guard.deref_mut();
                let _ = handler.handle(&message, peer, &mut gs_guard);
            }

            //acquiring the gs lock can get very expensive especially in early stage when many
            //messages flood in. while we have it, try to get more messages. if we can't nbd -
            //we'll wait for messages when the loop comes back around
            loop {
                match rx.try_recv() {
                    Ok(a) => {
                        let (ref msg, ref peer_arc) = a;
                        let mut peer_mg = peer_arc.deref().write().unwrap();

                        let mut peer_g = peer_mg.deref_mut();
                        let _ = handler.handle(msg, peer_g, &mut gs_guard);
                    }
                    _ => break
                }
            }
        }
    });
    (tx, sink)
}

/// Sets up a transmission based on a single torrent
fn init_torrent (tx: &Sender<(Message, Arc<RwLock<Peer>>)>, metadata: &Metadata, listen_port: u32, bytes_dled: u32, global_arc: Arc<Mutex<GlobalState>>) {
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
        let ga = global_arc.clone();

        thread::spawn(move || {
            match connect_to_peer(peer, &child_meta, &peer_id) {
                Ok((peer_id, mut reader)) => {
                    let peer_id_str = peer_id.iter().map(|x| *x as char).collect::<String>();
                    let mut peer = Peer::new(peer_id_str);
                    peer.state.set_us_interested(true);

                    let peer_cell = RwLock::new(peer);
                    let arc = Arc::new(peer_cell);
                    let mut pstream = reader.clone_stream();

                    pstream.send_message(Message::Interested);

                    { //add to the global peer list
                        let _ga = ga.clone();
                        let mut _y = (&_ga).lock().unwrap();
                        let mut _x = _y.deref_mut();
                        _x.add_new_peer(arc.clone(), pstream, peer_id.to_owned());
                    } //release da lock

                    loop {
                        //we can't just block read in a loop - we'll never have a chance to send out
                        //outgoing messages over TCP
                        match reader.wait_for_message() {
                            Ok(message) => tx.send((message, arc.clone())),
                            Err(e) => {
                                println!("error waiting for message: {:?}", e);
                                let _ga = ga.clone();
                                let mut gstate =  _ga.deref().lock().unwrap();

                                gstate.remove_peer(&peer_id);

                                //TODO: need to signal the handler thread that the client has
                                //disconnected
                                break;
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

    let global_state = GlobalState::new(&metadata);
    let global_arc = Arc::new(Mutex::new(global_state));

    let (tx, sink) = init(global_arc.clone(), DefaultHandler);

    //for now initialize torrents inline with main
    init_torrent(&tx, &metadata, 6887, 0, global_arc.clone());

    let spin_thread = thread::spawn(move || {
        loop {
            {
                let gs = global_arc.clone();

                let mut guard = (&gs).lock().unwrap();
                (&mut guard).deref_mut().spin();
            }

            thread::sleep_ms(1000);
        }
    });

    //block until the sink shuts down
    let _ = sink.join();
    //test();
}

/*fn test() {
    use bittorrent::chunk::Piece;
    let bitfield = vec![];
    let vec = Piece::convert_bitfield_to_piece_vec(&bitfield);

    let piece = Piece::from(10, 1, 0, 10);
    println!("{:?}", vec);
}*/
