use std::thread;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use common::{LoginRequest, ChatRoom, User};

extern crate bincode;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();

    let mut room_vec: Vec<ChatRoom> = Vec::new();
    create_chat_room(String::from("Lobby"), &mut room_vec);

    let rooms: Arc<Mutex<Vec<ChatRoom>>> = Arc::new(Mutex::new(room_vec));
    let users: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(Vec::new()));

    println!("server listening on port 3333");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new connection: {}", stream.peer_addr().unwrap());
                thread::spawn({ 
                    let room_clone = Arc::clone(&rooms);
                    let users_clone = Arc::clone(&users);
                    move || {
                        handle_client(stream, room_clone, users_clone);  
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

fn handle_client(mut stream: TcpStream, rooms: Arc<Mutex<Vec<ChatRoom>>>, users: Arc<Mutex<Vec<User>>>) {
    let request = receive_login_request(&mut stream);
    let user_id = match request {
        Some(r) => create_and_add_user(r.name, &users),
        None => create_and_add_user(String::from("anon"), &users)
    };

    let room_names = get_room_info(&rooms);
    send_room_info(room_names, &mut stream);

    let room_name = receive_room_name(&mut stream);
    match room_name {
        Some(name) => println!("user wants to join {}", name),
        None => println!("no room name available")
    }

    // TODO: create the room, attach user id to it
    // maybe first implement bidirectional chatting between two users?

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

fn create_and_add_user(user_name: String, users: &Arc<Mutex<Vec<User>>>) -> u8 {
    let mut user_vec = users.lock().unwrap();
    let user_id = user_vec.len() as u8;
    let user = User{id: user_id, name: user_name};
    user_vec.push(user);
    user_id
}

fn create_chat_room(room_name: String, room_vec: &mut Vec<ChatRoom>) {
    let room_id = room_vec.len() as u8;
    room_vec.push(ChatRoom{id: room_id, current_user: 0, name: room_name});
}

fn receive_room_name(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = vec!(0; ChatRoom::NAME_SIZE);
    match stream.read(&mut buffer) {
        Ok(size) => {
            if size == 0 {
                None
            } else {
                let name: String = bincode::deserialize(&buffer).unwrap();
                Some(name)
            }
        },
        Err(e) => {
            println!("error reading the room name from client {}: {}", stream.peer_addr().unwrap(), e);
            None
        }
    }
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
