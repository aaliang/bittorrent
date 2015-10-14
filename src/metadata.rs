use std::collections::HashMap;
use crypto::sha1::Sha1;
use crypto::digest::Digest;
use bencode::{Bencode, TypedMethods, BencodeToString};

#[derive(Clone, Debug)]
pub struct SingleFileInfo {
    length: i64,
    name: String,
    md5sum: Option<String>
}

#[derive(Clone, Debug)]
pub struct FileInfo {
    length: i64,
    md5sum: Option<String>,
    path: Vec<String>
}

#[derive(Clone, Debug)]
pub struct MultiFileInfo {
    name: String,
    files: Vec<FileInfo>
}

#[derive(Clone, Debug)]
pub enum FileMode {
    SingleFile(SingleFileInfo),
    MultiFile(MultiFileInfo)
}

#[derive(Debug, Clone)]
pub struct Metadata {
    announce: String,
    pub info_hash: String,
    piece_length: i64,
    pieces: String,
    mode_info: FileMode,
}

pub trait MetadataDict {
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