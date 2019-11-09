use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use common::LoginRequest;

extern crate bincode;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
    println!("server listening on port 3333");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move || {
                    handle_client(stream);
                });
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    drop(listener);
}

fn handle_client(mut stream: TcpStream) {
    let request = read_login_request(&mut stream);

    // TODO: unsuccessful should probably exit here
    match request {
        Some(r) => println!("user {} logged in", r.name),
        None => println!("unsuccessful login attempt") 
    };

    let mut data = [0 as u8; 1024];
    while match stream.read(&mut data) {
        Ok(size) => {
            if size == 0 {
                println!("client {} quit", stream.peer_addr().unwrap());
                false
            } else {
                stream.write(&data[0..size]).unwrap();
                true
            }
        },
        Err(_) => {
            println!("error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}

fn read_login_request(stream: &mut TcpStream) -> Option<LoginRequest> {
    let mut read_data = vec!(0; LoginRequest::SIZE);
    match stream.read(&mut read_data) {
        Ok(size) => {
            if size == 0 {
                None
            } else {
                let request: LoginRequest = bincode::deserialize(&read_data).unwrap();
                Some(request)
            }
        },
        Err(e) => {
            println!("error reading the login request from client {}: {}", stream.peer_addr().unwrap(), e);
            None
        }
    }
}
