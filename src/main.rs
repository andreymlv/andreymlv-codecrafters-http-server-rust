use anyhow::Result;
use nom::branch::alt;
use nom::bytes::complete::take_till;
use nom::character::complete::{alpha0, alpha1, char};
use nom::combinator::value;
use nom::multi::many0;
use nom::sequence::separated_pair;
use nom::{bytes::complete::tag, IResult};
use std::{
    // collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    str,
};

#[derive(Debug, Clone)]
enum Method {
    GET,
    POST,
}

// #[derive(Debug)]
// struct StartLine {
//     method: Method,
//     path: String,
//     version: String,
// }

// #[derive(Debug)]
// struct Request {
//     start_line: StartLine,
//     // headers: HashMap<String, String>,
//     // body: Option<String>,
// }

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream)?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_client(mut stream: std::net::TcpStream) -> Result<()> {
    let mut request = [0u8; 1024];
    let read = stream.read(&mut request)?;
    let request = str::from_utf8(&request[..read])?;
    let (rest, _) = parse_method(&request).unwrap();
    let (_, path) = parse_path(&rest).unwrap();
    if path == "/" {
        let respond = b"HTTP/1.1 200 OK\r\n\r\n";
        stream.write_all(respond)?;
    } else {
        let respond = b"HTTP/1.1 404 Not Found\r\n\r\n";
        stream.write_all(respond)?;
    }
    Ok(())
}

fn parse_method(input: &str) -> IResult<&str, Method> {
    alt((
        value(Method::GET, tag("GET ")),
        value(Method::POST, tag("POST ")),
    ))(input)
}

fn parse_path(input: &str) -> IResult<&str, &str> {
    take_till(|c| c == ' ')(input)
}
