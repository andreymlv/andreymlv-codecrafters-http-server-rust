use anyhow::Result;
use nom::branch::alt;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::multi::many1;
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{bytes::complete::tag, IResult};
use std::{
    io::{Read, Write},
    net::TcpListener,
    str,
};

#[derive(Debug)]
struct Request<'a> {
    method: &'a [u8],
    uri: &'a [u8],
    version: &'a [u8],
}

#[derive(Debug)]
struct Header<'a> {
    name: &'a [u8],
    value: Vec<&'a [u8]>,
}

fn is_token(c: u8) -> bool {
    match c {
        128..=255 => false,
        0..=31 => false,
        b'(' => false,
        b')' => false,
        b'<' => false,
        b'>' => false,
        b'@' => false,
        b',' => false,
        b';' => false,
        b':' => false,
        b'\\' => false,
        b'"' => false,
        b'/' => false,
        b'[' => false,
        b']' => false,
        b'?' => false,
        b'=' => false,
        b'{' => false,
        b'}' => false,
        b' ' => false,
        _ => true,
    }
}

fn not_line_ending(c: u8) -> bool {
    c != b'\r' && c != b'\n'
}

fn is_space(c: u8) -> bool {
    c == b' '
}

fn is_not_space(c: u8) -> bool {
    c != b' '
}
fn is_horizontal_space(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

fn is_version(c: u8) -> bool {
    c >= b'0' && c <= b'9' || c == b'.'
}

fn line_ending(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag("\r\n"), tag("\n")))(input)
}

fn request_line<'a>(input: &'a [u8]) -> IResult<&'a [u8], Request<'a>> {
    let (input, method) = take_while1(is_token)(input)?;
    let (input, _) = take_while1(is_space)(input)?;
    let (input, url) = take_while1(is_not_space)(input)?;
    let (input, _) = take_while1(is_space)(input)?;
    let (input, version) = http_version(input)?;
    let (input, _) = line_ending(input)?;
    Ok((
        input,
        (Request {
            method,
            uri: url,
            version,
        }),
    ))
}

fn http_version(input: &[u8]) -> IResult<&[u8], &[u8]> {
    preceded(tag("HTTP/"), take_while1(is_version))(input)
}

fn message_header_value(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(
        take_while1(is_horizontal_space),
        take_while1(not_line_ending),
        line_ending,
    )(input)
}

fn message_header<'a>(input: &'a [u8]) -> IResult<&'a [u8], Header<'a>> {
    let (rest, name) = take_while1(is_token)(input)?;
    let (rest, _) = char(':')(rest)?;
    let (rest, values) = many1(message_header_value)(rest)?;
    Ok((
        rest,
        Header {
            name,
            value: values,
        },
    ))
}

fn request<'a>(input: &'a [u8]) -> IResult<&'a [u8], (Request<'a>, Vec<Header<'a>>)> {
    terminated(pair(request_line, many1(message_header)), line_ending)(input)
}

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
    let mut request = [0u8; 2048];
    let read = stream.read(&mut request)?;
    if read >= request.len() {
        stream.write_all(b"HTTP/1.1 414 URI Too Long\r\n\r\n")?;
        return Ok(());
    }
    let (_, request) = request_line(&request[..read]).unwrap();
    let path = str::from_utf8(request.uri)?;
    if path.starts_with("/echo/") {
        let echo = &path[6..];
        let len = echo.len();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {len}\r\n\r\n{echo}"
        );
        stream.write_all(response.as_bytes())?;
    } else if path == "/" {
        let response = b"HTTP/1.1 200 OK\r\n\r\n";
        stream.write_all(response)?;
    } else {
        let response = b"HTTP/1.1 404 Not Found\r\n\r\n";
        stream.write_all(response)?;
    }
    Ok(())
}
