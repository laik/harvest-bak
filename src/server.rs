use super::Result;
use crate::db::{Database, Row};
use std::io::{Error, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

pub const END_WITH: [u8; 2] = [';' as u8, '\n' as u8];

pub enum Op {
    Add,
    Update,
    Delete,
}

pub struct Request {
    op: Op,
    row: Row,
}

pub enum Status {
    Ok,
    Err,
}

pub struct Response {
    status: Status,
    message: String,
}

pub struct RpcServer {
    socket_addr: String,
    database: Arc<Database>,
}

impl RpcServer {
    pub fn new(socket_addr: String, database: Arc<Database>) -> Self {
        Self {
            socket_addr,
            database,
        }
    }

    fn handle_client(mut stream: TcpStream) {
        let mut data = [0 as u8; 20]; // using 20 byte buffer
        let mut all_data = Vec::<u8>::new();
        loop {
            match stream.read(&mut data) {
                Ok(size) => {
                    all_data.append(&mut data[0..size].to_vec());
                    if !data[0..size].ends_with(&END_WITH) {
                        continue;
                    }
                    println!("write {:?}", String::from_utf8(all_data.clone()));
                    stream.write(all_data.as_slice()).unwrap();
                    all_data.clear();
                }
                Err(_) => {
                    println!(
                        "An error occurred, terminating connection with {}",
                        stream.peer_addr().unwrap()
                    );
                    stream.shutdown(Shutdown::Both).unwrap();
                    break;
                }
            }
        }
    }

    pub fn listen(&self) -> Result<()> {
        return match TcpListener::bind(&self.socket_addr) {
            Ok(listener) => {
                println!("RpcServer listen on {}", self.socket_addr);
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            println!("New connection: {}", stream.peer_addr().unwrap());
                            thread::spawn(move || Self::handle_client(stream));
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        };
    }
}
