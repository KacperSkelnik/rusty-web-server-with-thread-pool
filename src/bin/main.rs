use core::result::Result;
use simple_web_app::ThreadPool;
use std::fs;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::TcpListener;
use std::net::{Incoming, TcpStream};

fn get_request_handler() -> Result<String, Error> {
    fs::read_to_string("public/index.html").map(|content| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            content.len(),
            content
        )
    })
}

fn post_request_handler() -> Result<String, Error> {
    fs::read_to_string("public/index.html").map(|content| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            content.len(),
            content
        )
    })
}

fn router(buffer: &[u8; 1024]) -> Result<String, Error> {
    if buffer.starts_with(b"GET / HTTP/1.1\r\n") {
        get_request_handler()
    } else if buffer.starts_with(b"POST / HTTP/1.1\r\n") {
        post_request_handler()
    } else {
        println!("Request {}", String::from_utf8_lossy(&buffer[..]));
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn handle_request(mut stream: &TcpStream) -> Result<(), Error> {
    let mut buffer = [0; 1024];

    stream
        .read(&mut buffer)
        .and_then(|_| router(&buffer))
        .and_then(|response| stream.write(response.as_bytes()))
        .and_then(|_| stream.flush())
}

fn main() {
    let address: &str = "127.0.0.1:7878";
    let thread_pool: ThreadPool = ThreadPool::new(4);

    let use_request_handler =
        |incoming: Incoming| -> Result<(), Error> {
            println!("Connection established!");

            incoming
                .map(|stream_result| {
                    Ok(thread_pool
                        .execute(|| stream_result.and_then(|stream| handle_request(&stream))))
                })
                .collect()
        };

    TcpListener::bind(address)
        .and_then(|listener| use_request_handler(listener.incoming()))
        .unwrap_or_else(|e| println!("Error occurred: {}", e))
}
