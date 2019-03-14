mod db;
mod tp;

use db::{Command, Database};
use tp::ThreadPool;

use std::{
    io::prelude::*,
    io::BufReader,
    net::{TcpListener, TcpStream},
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    // Store data
    let storage = Database::new();

    // Creates a threadpool with 4 connection workers
    let mut pool = ThreadPool::new(4);

    // This is an infinite iterator
    for stream in listener.incoming() {
        pool.queue(move || {
            let mut stream = stream.unwrap();

            loop {
                let mut read_buffer = String::new();
                let mut buffered_stream = BufReader::new(&stream);
                if let Err(_) = buffered_stream.read_line(&mut read_buffer) {
                    break;
                }

                let cmd = db::parse(&read_buffer);

                match cmd {
                    Ok(Command::Get) => send_reply(
                        &mut stream,
                        storage.get().unwrap_or_else(|| "<empty>".into()),
                    ),
                    Ok(Command::Pub(s)) => {
                        storage.store(s);
                        send_reply(&mut stream, "<done>");
                    }
                    Err(e) => send_reply(&mut stream, format!("<error: {:?}>", e)),
                }
            }
        });
    }
}

// No need to really touch this function
fn send_reply<'a, S: Into<String>>(stream: &mut TcpStream, msg: S) {
    // Sometimes we break and we don't care
    let _ = stream.write(msg.into().as_bytes());
    let _ = stream.write("\r\n".as_bytes());
}
