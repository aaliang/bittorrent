use std::collections::HashMap;
use crypto::sha1::Sha1;
use crypto::digest::Digest;
use bencode::{Bencode, TypedMethods, BencodeToString};

#[derive(Clone, Debug)]
pub struct SingleFileInfo {
    length: i64,
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
    files: Vec<FileInfo>
}

#[derive(Clone, Debug)]
pub enum FileMode {
    SingleFile(SingleFileInfo),
    MultiFile(MultiFileInfo)
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub announce: String,
    pub info_hash: [u8; 20],
    name: String,
    piece_length: i64,
    pieces: String,
    mode_info: FileMode,
}

impl Metadata {
    pub fn get_total_length (&self) -> u32 {
        let len = match self.mode_info {
            FileMode::SingleFile(ref sf) => sf.length,
            FileMode::MultiFile(ref mf) => mf.files.iter().fold(0, |a:i64, b:&FileInfo| a + b.length)
        };
        len as u32
    }
}

fn to_file_list (list: &Vec<Bencode>) -> Option<Vec<FileInfo>> {
    //TODO: figure out how exception handling works
    Some(list.iter().map(|item| {
        match item {
            &Bencode::Dict(ref hm) => {
                let path_list_bencode = hm.get_list("path")
                                          .unwrap()
                                          .iter()
                                          .map(|x| match x {
                                                    &Bencode::String(ref path) => path.to_string(),
                                                    _ => panic!("unexpected type")
                                          }).collect::<Vec<String>>();
                FileInfo {
                    length: hm.get_int("length").unwrap_or_else(|| panic!("no length in file")),
                    md5sum: hm.get_owned_string("md5sum"),
                    path: path_list_bencode
                }
            },
            _ => panic!("not a bencode list of dicts")
        }
    }).collect::<Vec<FileInfo>>())
}

pub trait MetadataDict {
    fn to_metadata (&self) -> Option<Metadata>;
}

impl MetadataDict for HashMap<String, Bencode> {
    /// Extracts information from this HashMap into a Metadata instance, if valid. Currently if it
    /// is invalid, it will just throw a runtime exception
    fn to_metadata (&self) -> Option<Metadata> {
        let announce = self.get_string("announce").unwrap_or_else(||panic!("no key found for announce"));
        let info_dict = self.get_dict("info").unwrap_or_else(||panic!("no key found for info")).to_owned();
        let mut sha = Sha1::new();
        sha.input_str(&Bencode::Dict(info_dict.clone()).to_bencode_string());
        let mut info_hash:[u8; 20] = [0; 20];
        let result = sha.result(&mut info_hash);

        let mode_info = match info_dict.get_list("files") {
            Some(flist) => {
                FileMode::MultiFile(MultiFileInfo {
                    files: to_file_list(flist).unwrap()
                })
            },
            None => FileMode::SingleFile(SingleFileInfo {
                length: info_dict.get_int("length").unwrap_or_else(||panic!("no key found for length")),
                md5sum: info_dict.get_owned_string("md5sum")})

        };

        //for now only handle single file mode
        Some(Metadata {
            announce: announce.clone(),
            info_hash: info_hash,
            piece_length: info_dict.get_int("piece length").unwrap_or_else(||panic!("no key found for piece length")),
            pieces: info_dict.get_string("pieces").unwrap().to_string(),
            name: info_dict.get_string("name").unwrap_or_else(||panic!("no key found for name")).to_string(),
            mode_info: mode_info
        })
    }
}
