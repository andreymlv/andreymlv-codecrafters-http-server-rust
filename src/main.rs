use anyhow::{Error, Result};
use nom::branch::alt;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::multi::many1;
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{bytes::complete::tag, IResult};
use std::env::args;
use std::path::Path;
use std::str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct Request<'a> {
    method: &'a [u8],
    uri: &'a [u8],
    _version: &'a [u8],
}

#[derive(Debug)]
struct Header<'a> {
    name: &'a [u8],
    value: Vec<&'a [u8]>,
}

fn is_token(c: u8) -> bool {
    !matches!(c, 128..=255 | 0..=31 | b'(' | b')' | b'<' | b'>' | b'@' | b',' | b';' | b':' | b'\\' | b'"' | b'/' | b'[' | b']' | b'?' | b'=' | b'{' | b'}' | b' ')
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
    c.is_ascii_digit() || c == b'.'
}

fn line_ending(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((tag("\r\n"), tag("\n")))(input)
}

fn request_line(input: &[u8]) -> IResult<&[u8], Request<'_>> {
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
            _version: version,
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

fn message_header(input: &[u8]) -> IResult<&[u8], Header<'_>> {
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

fn request(input: &[u8]) -> IResult<&[u8], (Request<'_>, Vec<Header<'_>>)> {
    terminated(pair(request_line, many1(message_header)), line_ending)(input)
}

#[derive(Debug, Clone)]
struct Config {
    directory: Option<String>,
}

async fn run(stream: TcpStream, config: Config) -> Result<(), Error> {
    tokio::spawn(async move {
        handle_client(stream, config).await.unwrap();
    })
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut directory: Option<String> = None;
    if args().len() > 1 {
        if std::env::args().nth(1).expect("no pattern given") == "--directory" {
            directory = Some(args().nth(2).expect("no pattern given"));
        } else {
            panic!()
        }
    }
    let config = Config { directory };
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(run(stream, config.clone()));
    }
}

async fn handle_client(mut stream: TcpStream, config: Config) -> Result<()> {
    let mut buffer = [0u8; 2048];
    let read = stream.read(&mut buffer).await?;
    if read >= buffer.len() {
        stream
            .write_all(b"HTTP/1.1 414 URI Too Long\r\n\r\n")
            .await?;
        return Ok(());
    }
    let (_, (request, headers)) = request(&buffer[..read])
        .unwrap_or((&buffer, (request_line(&buffer[..read]).unwrap().1, vec![])));
    let path = str::from_utf8(request.uri)?;
    if request.method == b"GET" {
        if path.starts_with("/echo/") {
            let echo = path.strip_prefix("/echo/").unwrap();
            let len = echo.len();
            let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {len}\r\n\r\n{echo}"
        );
            stream.write_all(response.as_bytes()).await?;
        } else if path.starts_with("/files/") {
            let file = config.directory.unwrap() + path.strip_prefix("/files").unwrap();
            let file_path = Path::new(&file);
            if file_path.exists() {
                let contents = tokio::fs::read_to_string(file_path).await?;
                let len = contents.len();
                let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {len}\r\n\r\n{contents}"
        );
                stream.write_all(response.as_bytes()).await?;
            } else {
                let response = b"HTTP/1.1 404 Not Found\r\n\r\n";
                stream.write_all(response).await?;
            }
        } else if path.starts_with("/user-agent") {
            for header in headers {
                if header.name == b"User-Agent" {
                    let agent = str::from_utf8(header.value[0])?;
                    let len = agent.len();
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {len}\r\n\r\n{agent}");
                    stream.write_all(response.as_bytes()).await?;
                }
            }
        } else if path == "/" {
            let response = b"HTTP/1.1 200 OK\r\n\r\n";
            stream.write_all(response).await?;
        } else {
            let response = b"HTTP/1.1 404 Not Found\r\n\r\n";
            stream.write_all(response).await?;
        }
    }
    Ok(())
}
