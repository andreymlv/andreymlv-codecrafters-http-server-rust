use std::{
    io::{Read, Write},
    net::TcpListener,
    vec,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: std::net::TcpStream) {
    println!("accepted new connection");
    let mut buf = vec![];
    stream.read(&mut buf).unwrap();
    let respond = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(respond.as_bytes()).unwrap();
}
