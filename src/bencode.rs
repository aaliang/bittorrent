#![allow(unused_imports, unused_must_use, unused_variables, dead_code)]

extern crate combine;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::str;
use combine::{spaces, parser, between, many, many1, digit, char, any, string, token, Parser, ParserExt, ParseError};
use combine::primitives::{State, Stream, ParseResult};
use combine::combinator::{Between, Token, FnParser};

//my own bencode stuff!
//
pub enum Bencode {
    Number(i64),
    String(Vec<u8>)
}

fn open_file <P: AsRef<Path>>(path: P) -> Vec<u8> {
    let mut fd = File::open(path).unwrap();
    let mut buffer:Vec<u8> = Vec::new();
    fd.read_to_end(&mut buffer);
    buffer
}

enum Expr {
    Integer(i64)
}

//TODO: this does not handle negatives, yo
fn bencode_integer<I>(input: State<I>) -> ParseResult<i64, I> where I:Stream<Item=char> {
    let (open, close) = (char('i'), char('e'));
    let mut int = between(open, close, many1::<String, _>(digit())).map(|x| {
        x.parse::<i64>().unwrap()
    });
    int.parse_state(input)
}

fn bencode_string<I>(input: State<I>) -> ParseResult<i32, I> where I:Stream<Item=char> {
    bencode_string_length_prefix(input)
}

fn bencode_string_length_prefix<I>(input: State<I>) -> ParseResult<i32, I> where I:Stream<Item=char> {
    let many_digit = many1::<String, _>(digit());
    let mut get_len = (many_digit, char(':')).map(|(length, _)| length.parse::<i32>().unwrap());
    get_len.parse_state(input)
}

fn main () {
    let file_contents = open_file("t-test.torrent");

    match str::from_utf8(&file_contents) {
        Ok(v) => println!("file: {}", v),
        Err(e) => panic!("Not a UTF-8 String: {}", e)
    }

    //let result = parser(bencode_integer).parse("i5e");
    let result = parser(bencode_string).parse("3:sss");

    match result {
        Ok((a, _)) => println!("found {}", a),
        Err(e) => println!("{}", e)
    }
}
