#[macro_use]
extern crate log;

mod db;
mod tp;

use db::{Command, Database};
use env_logger;
use tp::ThreadPool;

use std::{
    io::prelude::*,
    io::BufReader,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
};

fn main() {
    env_logger::init();
    debug!("Starting up...");

    let pool = ThreadPool::new(24);

    let addr = "127.0.0.1:7878";
    let listener = TcpListener::bind(addr).unwrap();
    info!("Listening in {}", addr);

    let storage = Arc::new(Mutex::new(Database::new()));

    for stream in listener.incoming() {
        let storage = storage.clone();

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
                        storage
                            .lock()
                            .unwrap()
                            .get()
                            .unwrap_or_else(|| "<empty>".into()),
                    ),
                    Ok(Command::Pub(s)) => {
                        storage.lock().unwrap().store(s);
                        send_reply(&mut stream, "<done>");
                    }
                    Err(e) => send_reply(&mut stream, format!("<error: {:?}>", e)),
                }
            }
        });
    }

    // This will not work
    pool.shutdown();
}

// No need to really touch this function
fn send_reply<'a, S: Into<String>>(stream: &mut TcpStream, msg: S) {
    // Sometimes we break and we don't care
    let _ = stream.write(msg.into().as_bytes());
    let _ = stream.write("\r\n".as_bytes());
}
