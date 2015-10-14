extern crate bencode;
extern crate crypto;
extern crate rand;
extern crate url;

mod querystring;

use std::env;
use std::collections::HashMap;
use bencode::{deserialize_file, Bencode, TypedMethods, BencodeToString};
use crypto::sha1::Sha1;
use crypto::digest::Digest;
use rand::{Rng, thread_rng};

use querystring::QueryString;

const PEER_ID_LENGTH:usize = 20;
const PEER_ID_PREFIX:&'static str = "ABT:";

#[derive(Clone, Debug)]
struct SingleFileInfo {
    length: i64,
    name: String,
    md5sum: Option<String>
}

#[derive(Clone, Debug)]
struct FileInfo {
    length: i64,
    md5sum: Option<String>,
    path: Vec<String>
}

#[derive(Clone, Debug)]
struct MultiFileInfo {
    name: String,
    files: Vec<FileInfo>
}

#[derive(Clone, Debug)]
enum FileMode {
    SingleFile(SingleFileInfo),
    MultiFile(MultiFileInfo)
}

#[derive(Debug, Clone)]
struct Metadata {
    announce: String,
    info_hash: String,
    piece_length: i64,
    pieces: String,
    mode_info: FileMode,
}

trait MetadataDict {
    fn to_metadata (&self) -> Option<Metadata>;
}

impl MetadataDict for HashMap<String, Bencode> {
    /// Extracts information from this HashMap into a Metadata instance, if valid. Currently if it
    /// is invalid, it will just throw a runtime exception
    fn to_metadata (&self) -> Option<Metadata> {
        let announce = self.get_string("announce").unwrap();
        let info_dict = self.get_dict("info").unwrap().to_owned();
        let mut sha = Sha1::new();

        sha.input_str(&Bencode::Dict(info_dict.clone()).to_bencode_string());

        //for now only handle single file mode
        Some(Metadata {
            announce: announce.clone(),
            info_hash: sha.result_str().to_string(),
            piece_length: info_dict.get_int("piece length").unwrap(),
            pieces: info_dict.get_string("pieces").unwrap().to_string(),
            mode_info: FileMode::SingleFile(SingleFileInfo {
                length: info_dict.get_int("length").unwrap(),
                name: info_dict.get_string("name").unwrap().to_string(),
                md5sum: info_dict.get_owned_string("md5sum")
            })
        })
    }
}

fn gen_rand_peer_id (prefix: &str) -> String {
    let rand_length = PEER_ID_LENGTH - prefix.len();
    let rand = thread_rng().gen_ascii_chars()
                           .take(rand_length)
                           .collect::<String>();

    prefix.to_string() + &rand
}

fn init (metadata: Metadata, listen_port: u32) {
    let peer_id = gen_rand_peer_id(PEER_ID_PREFIX);
    let req_params = QueryString::from(vec![
                                           ("info_hash", metadata.info_hash),
                                           ("peer_id", peer_id),
                                           ("port", listen_port.to_string()),
                                           ("uploaded", 0.to_string()),
                                           ("downloaded", 0.to_string()) 
                                           ]).query_string();
    println!("params: {}", req_params);
}

fn main () {
    let path = env::args()
                    .nth(1)
                    .unwrap_or_else(||panic!("no path to torrent provided"));

    let content = deserialize_file(path)
                    .unwrap_or_else(||panic!("unable to parse bencoded metadata"));

    let metadata = match content.first() {
        Some(&Bencode::Dict(ref x)) => x.to_metadata(),
        _ => panic!("no valid information in torrent file")
    }.unwrap();

    let m = metadata.clone();
    init(metadata, 6888);
}
