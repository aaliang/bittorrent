#![allow(unused_imports, unused_must_use, dead_code)]
extern crate combine;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::str;
use combine::{parser, between, many, many1, digit, char, Parser, ParserExt};
use combine::primitives::{State, Stream, ParseResult, Consumed};
use combine::combinator::FnParser;

//my own bencode stuff!
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Bencode {
    Int(i64),
    ByteString(Vec<u8>),
    List(Vec<Bencode>),
    Dict(HashMap<String, Bencode>)
}

//for options over Bencode Shapes
pub trait BencodeVecOption {
    /// Unwraps self if it wraps a dictionary
    fn to_singleton_dict (&self) -> Option<HashMap<String, Bencode>>;
}

impl BencodeVecOption for Option<Vec<Bencode>> {
    fn to_singleton_dict (&self) -> Option <HashMap<String, Bencode>> {
        match self {
            &Some(ref a) => match (a.first(), a.len()) {
                (Some(&Bencode::Dict(ref b)), 1) => Some(b.to_owned()),
                _ => None
            },
            _ => None
        }
    }
}

pub trait BencodeToString {
    fn to_bencode_string (&self) -> Vec<u8>;
}

//yes, we could just use the offsets from the parsed file. but we aren't keeping it around, nor
//are the parsers returning offsets right now (and as it's only needed for computing the
//hashsum... and computing this on the fly is comparatively easier than navigating through
//the sigils of combine
//
//might be worth adapting at the end as its modular, probably not worth it though)
impl BencodeToString for Bencode {
    fn to_bencode_string (&self) -> Vec<u8> {
        match *self { //oh god... what did i do...
            Bencode::Int(ref int) => {
                let mut vec: Vec<u8> = Vec::new();
                vec.push('i' as u8);
                for a_char in int.to_string().chars() {
                    vec.push(a_char as u8);
                }
                vec.push('e' as u8);
                vec
            }
            Bencode::ByteString(ref string) => {
                //this is horrible and ugly because of a string/ascii oversight
                let mut vec: Vec<u8> = Vec::new();
                for a_char in string.len().to_string().chars() {
                    vec.push(a_char as u8);
                }
                vec.push(':' as u8);
                for byte in string.iter() {
                    vec.push(*byte as u8);
                }
                vec
            },
            Bencode::List(ref list) => {
                list.iter().flat_map(|x| x.to_bencode_string()).collect::<Vec<u8>>()
            },
            Bencode::Dict(ref dict) => {
                let mut vec: Vec<u8> = Vec::new();
                let mut kvs: Vec<(&String, &Bencode)> = dict.iter().collect();
                kvs.sort_by(|a, b| a.0.cmp(&b.0));
                vec.push('d' as u8);
                for (key_name, val) in kvs {
                    for a_char in key_name.len().to_string().chars() {
                        vec.push(a_char as u8);
                    }
                    vec.push(':' as u8);
                    for byte in key_name.chars() {
                        vec.push(byte as u8);
                    }
                    // vec.push(key_name.to_string());
                    for byte in val.to_bencode_string().iter() {
                        vec.push(*byte as u8);
                    }
                }
                vec.push('e' as u8);
                vec
            }
        }
    }
}

/// Opens a file and returns its contents as vector of bytes
/// throws an exception for now if ENOENT
pub fn open_file <P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut fd = File::open(path).unwrap();
    let mut buffer:Vec<u8> = Vec::new();
    let _ = fd.read_to_end(&mut buffer);
    buffer
}

/// Deserializes a bencoded file
pub fn deserialize_file<P: AsRef<Path>>(path: P) -> Option<Vec<Bencode>> {
    deserialize(&open_file(path))
}

/// Takes an input (vector of bytes) and returns the deserialized form as a vector of Bencode(d)
/// objects. Wrapped in an Option.
///
/// there's probably an argument that this should be a Result as opposed to an Option, as you
/// could potentially be losing error pertinent information in an Option. not going to change it over
/// right now
pub fn deserialize (byte_vector: &[u8]) -> Option<Vec<Bencode>> {
    //hack to coerce ascii byte values to rust array sliceof char (UTF-8). necessary to avoid substantial
    //writing of existing combine parser builtins
    let as_string: Vec<char> = byte_vector.iter().map(|x| *x as char).collect();
    match many::<Vec<Bencode>, _>(bencode_any()).parse(&as_string[..]) {
        Ok((result, _)) => Some(result),
        Err(_) => None
    }
}

/// Provides typesafe getters for a collection
pub trait TypedMethods {
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_string(&self, key: &str) -> Option<&Vec<u8>>;
    fn get_owned_string(&self, key: &str) -> Option<Vec<u8>>;
    fn get_dict(&self, key: &str) -> Option<&HashMap<String, Bencode>>;
    fn get_list(&self, key: &str) -> Option<&Vec<Bencode>>;
}

impl TypedMethods for HashMap<String, Bencode> {
    fn get_int (&self, key: &str) -> Option <i64> {
        match self.get(key) {
            Some(&Bencode::Int(a)) => Some(a),
            _ => None
        }
    }

    //yeah... that oversight is killing me. its actually a byte vector
    fn get_string (&self, key: &str) -> Option <&Vec<u8>> {
        match self.get(key) {
            Some(&Bencode::ByteString(ref a)) => Some(a),
            _ => None
        }
    }

    fn get_owned_string(&self, key: &str) -> Option <Vec<u8>> {
        match self.get(key) {
            Some(&Bencode::ByteString(ref a)) => Some(a.to_owned()),
            _ => None
        }
    }

    fn get_dict (&self, key: &str) -> Option <&HashMap<String, Bencode>> {
        match self.get(key) {
            Some(&Bencode::Dict(ref a)) => Some(a),
            _ => None
        }
    }

    fn get_list (&self, key: &str) -> Option <&Vec<Bencode>> {
        match self.get(key) {
            Some(&Bencode::List(ref a)) => Some(a),
            _ => None
        }
    }
}

fn bencode_integer<I>(input: State<I>) -> ParseResult<i64, I> where I: Stream<Item=char> {
    let (open, close) = (char('i'), char('e'));
    let mut int = between(open, close, many1::<String, _>(digit())).map(|x| {
        x.parse::<i64>().unwrap()
    });
    int.parse_state(input)
}

fn bencode_string<I>(input: State<I>) -> ParseResult<Vec<u8>, I> where I: Stream<Item=char> {
    let (len, input_) = try!(bencode_string_length_prefix(input));
    input_.combine(|input__| take(len).parse_state(input__))
}

fn bencode_string_length_prefix<I>(input: State<I>) -> ParseResult<i32, I> where I:Stream<Item=char> {
    let many_digit = many1::<String, _>(digit());
    let mut get_len = (many_digit, char(':')).map(|(length, _)| length.parse::<i32>().unwrap());
    get_len.parse_state(input)
}

fn bencode_list<I>(input: State<I>) -> ParseResult<Vec<Bencode>, I> where I: Stream<Item=char> {
    let (open, close) = (char('l'), char('e'));
    let list_contents = many::<Vec<Bencode>, _>(bencode_any());
    let mut list = between(open, close, list_contents);
    list.parse_state(input)
}

fn bencode_dict<I>(input: State<I>) -> ParseResult<HashMap<String, Bencode>, I> where I: Stream<Item=char> {
    let (open, close) = (char('d'), char('e'));
    let pairs = (parser(bencode_string), bencode_any());
    let mut dict = between(open, close, many(pairs)).map(|entries:Vec<(Vec<u8>, Bencode)>|{
        let mut hash_map = HashMap::new();
        for (k, v) in entries {
            // this is a dangerous assumption
            let key_as_string = str::from_utf8(&k).unwrap().to_string();
            hash_map.insert(key_as_string, v);
        }
        hash_map
    });
    dict.parse_state(input)
}

fn bencode_any<I>() -> FnParser<I, fn (State<I>) -> ParseResult<Bencode, I>> where I: Stream<Item=char> {
    fn bencode_any_<I>(input: State<I>) -> ParseResult<Bencode, I> where I: Stream<Item=char> {
        parser(bencode_integer).map(Bencode::Int)
            .or(parser(bencode_string).map(Bencode::ByteString))
            .or(parser(bencode_list).map(Bencode::List))
            .or(parser(bencode_dict).map(Bencode::Dict)).parse_state(input)
    }
    parser(bencode_any_)
}

fn take <I> (num: i32) -> SizedBuffer<I> where I: Stream<Item=char> {
    SizedBuffer(num, PhantomData)
}

#[derive(Clone)]
struct SizedBuffer <I>(i32, PhantomData<I>);
impl <I> Parser for SizedBuffer<I> where I: Stream<Item=char> {
    type Input = I;
    type Output = Vec<u8>;
    fn parse_lazy(&mut self, mut input: State<I>) -> ParseResult<Vec<u8>, I> {
        let start = input.position;
        let mut vec_buf:Vec<char> = Vec::new();
        for _ in 0..self.0 {
            match input.uncons() {
                Ok((other, rest)) => {
                    vec_buf.push(other);
                    input = rest.into_inner();
                }
                Err(error) => {
                    return error.combine(|mut error| {
                        error.position = start;
                        Err(Consumed::Consumed(error))
                    })
                }
            };
        }
        Ok((vec_buf.clone().iter().map(|x| *x as u8).collect::<Vec<u8>>(), Consumed::Consumed(input)))
    }
}

#[test]
fn test_integer() {
    let result = parser(bencode_integer).parse("i57e");
    assert_eq!(result, Ok((57,"")));
}

#[test]
fn test_string() {
    let result = parser(bencode_string).parse("5:abcde");
    assert_eq!(result, Ok(("abcde".to_string(), "")));;
}

#[test]
fn test_list() {
    let homogenous_int_list = parser(bencode_list).parse("li57ei32ee");
    assert_eq!(homogenous_int_list, Ok((vec![Bencode::Int(57), Bencode::Int(32)], "")));

    let homogenous_str_list = parser(bencode_list).parse("l3:abc5:defghe");
    assert_eq!(homogenous_str_list, Ok((vec![Bencode::ByteString("abc".to_string()), Bencode::ByteString("defgh".to_string())], "")));

    let hetero_list = parser(bencode_list).parse("li32e3:abce");
    assert_eq!(hetero_list, Ok((vec![Bencode::Int(32), Bencode::ByteString("abc".to_string())], "")));
}

#[test]
fn test_dict() {
    let (dict_result, _) = parser(bencode_dict).parse("d3:abci4e4:andyli5eee").unwrap();
    assert_eq!(*dict_result.get("abc").unwrap(), Bencode::Int(4));
    assert_eq!(*dict_result.get("andy").unwrap(), Bencode::List(vec![Bencode::Int(5)]));
    assert_eq!(dict_result.len(), 2);
}

#[test]
fn test_many() {
    let input_string = "i1ei2ei3ei10el3:abc4:defged1:ai10ee";
    let (result, _) = many::<Vec<Bencode>, _>(bencode_any()).parse(input_string).unwrap();
    let mut my_map = HashMap::new();
    my_map.insert("a".to_string(), Bencode::Int(10));
    assert_eq!(result, vec![
        Bencode::Int(1),
        Bencode::Int(2),
        Bencode::Int(3),
        Bencode::Int(10),
        Bencode::List(vec![
            Bencode::ByteString("abc".to_string()),
            Bencode::ByteString("defg".to_string())
            ]),
        Bencode::Dict(my_map)
    ]);
}
