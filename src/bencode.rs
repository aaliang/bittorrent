//#![allow(unused_imports, unused_must_use, unused_variables, dead_code)]

extern crate combine;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::env;
use combine::{parser, between, many, many1, digit, char, Parser, ParserExt};
use combine::primitives::{State, Stream, ParseResult, Consumed};
use combine::combinator::FnParser;

//my own bencode stuff!
#[derive(Debug, Eq, PartialEq)]
enum Bencode {
    Int(i64),
    String(String),
    List(Vec<Bencode>),
    Dict(HashMap<String, Bencode>)
}

fn bencode_integer<I>(input: State<I>) -> ParseResult<i64, I> where I: Stream<Item=char> {
    let (open, close) = (char('i'), char('e'));
    let mut int = between(open, close, many1::<String, _>(digit())).map(|x| {
        x.parse::<i64>().unwrap()
    });
    int.parse_state(input)
}

fn bencode_string<I>(input: State<I>) -> ParseResult<String, I> where I: Stream<Item=char> {
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
    let mut dict = between(open, close, many(pairs)).map(|entries:Vec<(String, Bencode)>|{
        let mut hash_map = HashMap::new();
        for (k, v) in entries {
            hash_map.insert(k, v);
        }
        hash_map
    });
    dict.parse_state(input)
}

fn bencode_any<I>() -> FnParser<I, fn (State<I>) -> ParseResult<Bencode, I>> where I: Stream<Item=char> {
    fn bencode_any_<I>(input: State<I>) -> ParseResult<Bencode, I> where I: Stream<Item=char> {
        parser(bencode_integer).map(Bencode::Int)
            .or(parser(bencode_string).map(Bencode::String))
            .or(parser(bencode_list).map(Bencode::List))
            .or(parser(bencode_dict).map(Bencode::Dict)).parse_state(input)
    }
    parser(bencode_any_)
}

fn take <I> (num: i32) -> SizedBuffer<I> where I: Stream<Item=char> {
    SizedBuffer(num, PhantomData)
}

#[derive(Clone)]
pub struct SizedBuffer <I>(i32, PhantomData<I>);
impl <I> Parser for SizedBuffer<I> where I: Stream<Item=char> {
    type Input = I;
    type Output = String;
    fn parse_lazy(&mut self, mut input: State<I>) -> ParseResult<String, I> {
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
        Ok((vec_buf.into_iter().collect(), Consumed::Consumed(input)))
    }
}

fn open_file <P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut fd = File::open(path).unwrap();
    let mut buffer:Vec<u8> = Vec::new();
    let _ = fd.read_to_end(&mut buffer);
    buffer
}

//there's probably an argument that this should be a Result as opposed to an Option, as you
//could potentially be losing error pertinent information in an Option. not going to change it over
//right now
fn deserialize (byte_vector: Vec<u8>) -> Option<Vec<Bencode>> {
    //hack to coerce ascii byte values to rust array sliceof char (UTF-8). necessary to avoid substantial
    //writing of existing combine parser builtins
    let as_string: Vec<char> = byte_vector.iter().map(|x| *x as char).collect();
    match many::<Vec<Bencode>, _>(bencode_any()).parse(&as_string[..]) {
        Ok((result, _)) => Some(result),
        Err(_) => None
    }
}

fn deserialize_file<P: AsRef<Path>>(path: P) -> Option<Vec<Bencode>> {
    deserialize(open_file(path))
}

fn main () {
    let path = env::args().nth(1).unwrap();
    let cont = deserialize_file(path).unwrap();
    println!("{:?}", cont);
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
    assert_eq!(homogenous_str_list, Ok((vec![Bencode::String("abc".to_string()), Bencode::String("defgh".to_string())], "")));

    let hetero_list = parser(bencode_list).parse("li32e3:abce");
    assert_eq!(hetero_list, Ok((vec![Bencode::Int(32), Bencode::String("abc".to_string())], "")));
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
            Bencode::String("abc".to_string()),
            Bencode::String("defg".to_string())
            ]),
        Bencode::Dict(my_map)
    ]);
}
