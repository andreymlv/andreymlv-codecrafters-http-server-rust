use std::{net::TcpListener, io::{Read, Write}, vec};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                let mut buf = vec![];
                stream.read_to_end(&mut buf).unwrap();
                let respond = "HTTP/1.1 200 OK\r\n\r\n";
                stream.write_all(respond.as_bytes()).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
