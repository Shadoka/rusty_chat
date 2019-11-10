use std::thread;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use common::{LoginRequest, ChatRoom};

extern crate bincode;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
    let rooms: Arc<Mutex<Vec<ChatRoom>>> = Arc::new(Mutex::new(Vec::new()));
    println!("server listening on port 3333");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new connection: {}", stream.peer_addr().unwrap());
                thread::spawn({ 
                    let clone = Arc::clone(&rooms);
                    move || {
                        handle_client(stream, clone);  
                    }
                });
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    drop(listener);
}

fn handle_client(mut stream: TcpStream, rooms: Arc<Mutex<Vec<ChatRoom>>>) {
    let request = receive_login_request(&mut stream);
    // TODO: unsuccessful should probably exit here
    match request {
        Some(r) => println!("user {} logged in", r.name),
        None => println!("unsuccessful login attempt") 
    };

    let room_names = get_room_info(&rooms);
    send_room_info(room_names, &mut stream);

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

fn send_room_info(room_names: Vec<String>, stream: &mut TcpStream) {
    let serialized_names = bincode::serialize(&room_names).unwrap();
    stream.write(&serialized_names).unwrap();
}

fn get_room_info(rooms: &Arc<Mutex<Vec<ChatRoom>>>) -> Vec<String> {
    let room_vec = rooms.lock().unwrap();
    let mut room_names: Vec<String> = Vec::new();
    for room in room_vec.iter() {
        room_names.push(String::from(room.name.as_str()));
    }
    room_names
}

fn receive_login_request(stream: &mut TcpStream) -> Option<LoginRequest> {
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
