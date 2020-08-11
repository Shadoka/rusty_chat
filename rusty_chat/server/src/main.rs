use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use crossbeam_channel as channel;
use crossbeam_channel::{Sender, Receiver};
use common::{LoginRequest, ChatRoom, User, ChatMode, MasterSelectionResult};
use rand::{thread_rng, Rng};

extern crate bincode;
extern crate rand;
extern crate crossbeam_channel;

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
        Some(r) => create_and_add_user(r.name, stream.peer_addr().unwrap().ip().to_string(), &users),
        None => create_and_add_user(String::from("anon"), stream.local_addr().unwrap().to_string(), &users)
    };

    let (sender, receiver) = channel::unbounded();
	attach_sender_to_user(&users, user_id, sender);
    
    let receiver_thread = thread::spawn({
        let listen_stream_clone = stream.try_clone().unwrap();
		move || {
			listen_on_channel(receiver, listen_stream_clone);
		}
	});

    while match receive_chat_mode(&mut stream) {
        Some(m) => {
            if m == ChatMode::DIRECT {
                direct_mode(&mut stream, &users, user_id)
            } else if m == ChatMode::WAIT {
                false
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

    receiver_thread.join().unwrap();
    
    remove_user(user_id, &users);
    println!("terminating connection with {}", stream.peer_addr().unwrap().to_string());
}

// TODO: https://stackoverflow.com/questions/26126683/how-to-match-trait-implementors
// TODO: Need to work in a chat request for example, probably close request too
fn listen_on_channel(receiver: Receiver<MasterSelectionResult>, mut stream: TcpStream) {
	while match receiver.recv() {
		Ok(selection) => {
			send_master_selection(&selection, &mut stream);
			false
		},
		Err(e) => {
			println!("error receiving master selection on channel: {}", e);
			false
		}
	} {}
}

fn send_master_selection(selection: &MasterSelectionResult, stream: &mut TcpStream) {
	let serialized = bincode::serialize(selection).unwrap();
	stream.write(&serialized).unwrap();
}

fn attach_sender_to_user(users: &Arc<Mutex<Vec<User>>>, id: u8, sender: Sender<MasterSelectionResult>) {
	let mut user_vec = users.lock().unwrap();
	match user_vec.iter_mut().find(|u| u.id == id) {
		Some(user) => user.sender = Some(sender),
		None => ()
	}
}

fn direct_mode(stream: &mut TcpStream, users: &Arc<Mutex<Vec<User>>>, own_user_id: u8) -> bool {
    // we have to wait for atleast another user
    // TODO: send updates, for as long as we are not chatting
    while check_for_users(&users) {
        thread::sleep(time::Duration::from_millis(1000));
    }
    let user_names = get_user_names(&users);
    send_string_vec(user_names, stream);

    let user_name_buffer = vec!(0; User::NAME_SIZE);
    let to_chat_with = receive_string(stream, user_name_buffer);
    let own_name = get_name_by_id(own_user_id, &users).unwrap();
    let other_name = match to_chat_with {
        Some(name) => name,
        None => String::from("")
    };
    
    if &other_name == "" {
    	println!("no name submitted");
    	return true
    }
    println!("{} wants to chat with {}", own_name, other_name);
    
    let other_id = get_id_by_name(&other_name, &users).unwrap();
    let mut ids = Vec::new();
    ids.push(own_user_id);
    ids.push(other_id);
    let master_id = choose_master(ids);
    let master_ip = get_address_by_id(own_user_id, &users).unwrap();

    println!("master_id: {}, master_ip={}", master_id, &master_ip);

    let other_sender = get_sender_by_id(&users, other_id).expect("cannot communicate with other client");
	let mut other_selection_result = MasterSelectionResult{chat_partner_name: own_name, target_ip: master_ip.clone(), is_own_ip: false};
	if master_id == other_id {
		other_selection_result.is_own_ip = true;
	}
    other_sender.send(other_selection_result).unwrap();
    
    println!("result sent to other party");
    
    let mut selection_result = MasterSelectionResult{chat_partner_name: other_name, target_ip: master_ip, is_own_ip: false};
    if master_id == own_user_id {
        selection_result.is_own_ip = true;
    }

    // it's neccessary to go via the channel to terminate our own receiver thread
    let own_sender = get_sender_by_id(&users, own_user_id).expect("cant obtain own sender channel");
    own_sender.send(selection_result).unwrap();

    false
}

fn get_address_by_id(id: u8, users: &Arc<Mutex<Vec<User>>>) -> Option<String> {
    let user_vec = users.lock().unwrap();
    match user_vec.iter().find(|u| u.id == id) {
        Some(user) => Some(user.ip_address.clone()),
        None => None
    }
}

fn get_sender_by_id(users: &Arc<Mutex<Vec<User>>>, id: u8) -> Option<Sender<MasterSelectionResult>> {
    let user_vec = users.lock().unwrap();
    match user_vec.iter().find(|u| u.id == id) {
        Some(user) =>  {
            user.get_sender()
        },
        None => None
    }
}

fn get_id_by_name(name: &String, users: &Arc<Mutex<Vec<User>>>) -> Option<u8> {
	let user_vec = users.lock().unwrap();
	match user_vec.iter().find(|u| &u.name == name) {
		Some(user) => Some(user.id),
		None => None
	}
}

fn choose_master(ids: Vec<u8>) -> u8 {
	let mut rng = thread_rng();
	let selection_id: usize = rng.gen_range(0, ids.len());
	ids[selection_id]
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

fn check_for_users(users: &Arc<Mutex<Vec<User>>>) -> bool {
    let user_vec = users.lock().unwrap();
    user_vec.len() <= 1
}

fn get_name_by_id(id: u8, users: &Arc<Mutex<Vec<User>>>) -> Option<String> {
    let user_vec = users.lock().unwrap();
    match user_vec.iter().find(|u| u.id == id) {
        Some(user) => Some(user.name.clone()),
        None => None
    }
}

fn remove_user(id: u8, users: &Arc<Mutex<Vec<User>>>) -> Option<User> {
    let mut user_vec = users.lock().unwrap();
    for i in 0..user_vec.len() {
        if user_vec[i].id == id {
            return Some(user_vec.swap_remove(i))
        }
    }
    None
}

fn create_and_add_user(user_name: String, ip_address: String, users: &Arc<Mutex<Vec<User>>>) -> u8 {
    let mut user_vec = users.lock().unwrap();
    let user_id = user_vec.len() as u8;
    let user = User{id: user_id, name: user_name, ip_address: ip_address, sender: None};
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
