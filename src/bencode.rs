#![allow(unused_imports, unused_must_use, unused_variables, dead_code)]

extern crate combine;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::str;
use std::marker::PhantomData;
use combine::{spaces, parser, between, many, many1, digit, char, any, string, token, Parser, ParserExt, ParseError};
use combine::primitives::{State, Stream, ParseResult, Consumed};
use combine::combinator::{Between, Token, FnParser, satisfy};

//my own bencode stuff!
//
fn open_file <P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut fd = File::open(path).unwrap();
    let mut buffer:Vec<u8> = Vec::new();
    fd.read_to_end(&mut buffer);
    buffer
}

#[derive(Debug, Eq, PartialEq)]
enum Bencode {
    Int(i64),
    String(String),
    List(Vec<Bencode>)
}

//TODO: this does not handle negatives, yo
fn bencode_integer<I>(input: State<I>) -> ParseResult<Bencode, I> where I:Stream<Item=char> {
    let (open, close) = (char('i'), char('e'));
    let mut int = between(open, close, many1::<String, _>(digit())).map(|x| {
        Bencode::Int(x.parse::<i64>().unwrap())
    });
    int.parse_state(input)
}

fn bencode_string<I>(input: State<I>) -> ParseResult<Bencode, I> where I:Stream<Item=char> {
    let (len, input_) = try!(bencode_string_length_prefix(input));
    input_.combine(|input__| take(len).map(|x| Bencode::String(x)).parse_state(input__))
}

fn bencode_string_length_prefix<I>(input: State<I>) -> ParseResult<i32, I> where I:Stream<Item=char> {
    let many_digit = many1::<String, _>(digit());
    let mut get_len = (many_digit, char(':')).map(|(length, _)| length.parse::<i32>().unwrap());
    get_len.parse_state(input)
}

//for now only handle i64s
fn bencode_list<I>(input: State<I>) -> ParseResult<Bencode, I> where I:Stream<Item=char> {
    let (open, close) = (char('l'), char('e'));
    let list_contents = many(parser(bencode_integer).or(parser(bencode_string)));
    let mut list = between(open, close, list_contents).map(|x| Bencode::List(x));
    list.parse_state(input)
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

fn main () {
    let file_contents = open_file("t-test.torrent");

    match str::from_utf8(&file_contents) {
        Ok(v) => println!("file: {}", v),
        Err(e) => panic!("Not a UTF-8 String: {}", e)
    }
}

#[test]
fn test_integer() {
    let result = parser(bencode_integer).parse("i57e");
    assert_eq!(result, Ok((Bencode::Int(57),"")));
}

#[test]
fn test_string() {
    let result = parser(bencode_string).parse("5:abcde");
    assert_eq!(result, Ok((Bencode::String("abcde".to_string()), "")));;
}

#[test]
fn test_list() {
    let homogenous_int_list = parser(bencode_list).parse("li57ei32ee");
    assert_eq!(homogenous_int_list, Ok((Bencode::List(vec![Bencode::Int(57), Bencode::Int(32)]), "")));

    let homogenous_str_list = parser(bencode_list).parse("l3:abc5:defghe");
    assert_eq!(homogenous_str_list, Ok((Bencode::List(vec![Bencode::String("abc".to_string()), Bencode::String("defgh".to_string())]), "")));

    let hetero_list = parser(bencode_list).parse("li32e3:abce");
    assert_eq!(hetero_list, Ok((Bencode::List(vec![Bencode::Int(32), Bencode::String("abc".to_string())]), "")));
}

