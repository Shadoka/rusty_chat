use std::thread;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use common::{LoginRequest, ChatRoom, User, ChatMode};

extern crate bincode;

// TODO: receiving 0 bytes in any receive* method means the client quit on us -> do something sensible
// TODO: find logging crate
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

    while match receive_chat_mode(&mut stream) {
        Some(m) => {
            if m == ChatMode::DIRECT {
                direct_mode(&mut stream, &users, user_id)
            } else {
                room_mode(&mut stream, &rooms)
            }
        },
        None => {
            println!("no mode selected, terminating connection");
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
}

fn direct_mode(stream: &mut TcpStream, users: &Arc<Mutex<Vec<User>>>, own_user_id: u8) -> bool {
    let user_names = get_user_names(&users);
    send_string_vec(user_names, stream);

    let user_name_buffer = vec!(0; User::NAME_SIZE);
    let to_chat_with = receive_string(stream, user_name_buffer);
    let own_name = get_name_by_id(own_user_id, &users).unwrap();
    match to_chat_with {
        Some(name) => println!("{} wants to chat with {}", name, own_name),
        None => println!("no name submitted")
    }
    true
}

fn room_mode(stream: &mut TcpStream, rooms: &Arc<Mutex<Vec<ChatRoom>>>) -> bool {
    let room_names = get_room_info(&rooms);
    send_string_vec(room_names, stream);

    let room_name_buffer = vec!(0; ChatRoom::NAME_SIZE);
    let room_name = receive_string(stream, room_name_buffer);
    match room_name {
        Some(name) => println!("user wants to join {}", name),
        None => println!("no room name available")
    }
    true
}

fn get_name_by_id(id: u8, users: &Arc<Mutex<Vec<User>>>) -> Option<String> {
    let user_vec = users.lock().unwrap();
    match user_vec.iter().find(|u| u.id == id) {
        Some(user) => Some(user.name.clone()),
        None => None
    }
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

// TODO: i bet i can generify the return type
fn receive_string(stream: &mut TcpStream, mut buffer: Vec<u8>) -> Option<String> {
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

fn receive_chat_mode(stream: &mut TcpStream) -> Option<ChatMode> {
    let mut buffer = vec!(0; ChatMode::SIZE);
    match stream.read(&mut buffer) {
        Ok(size) => {
            if size == 0 {
                None
            } else {
                let mode: ChatMode = bincode::deserialize(&buffer).unwrap();
                Some(mode)
            }
        },
        Err(e) => {
            println!("error reading the chat mode from the client {}: {}", stream.peer_addr().unwrap(), e);
            None
        }
    }
}

fn send_string_vec(room_names: Vec<String>, stream: &mut TcpStream) {
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

fn get_user_names(users: &Arc<Mutex<Vec<User>>>) -> Vec<String> {
    let user_vec = users.lock().unwrap();
    let mut user_names: Vec<String> = Vec::new();
    for user in user_vec.iter() {
        user_names.push(String::from(user.name.as_str()))
    }
    user_names
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
