use std::net::{TcpStream, Shutdown};
use std::io::{Read, Write, BufRead};
use std::str::from_utf8;
use common::{LoginRequest, ChatRoom, ChatMode, User, MasterSelectionResult};

extern crate bincode;

struct Input {
    user_input: Vec<u8>,
    read_string: String,
    length: usize
}

fn main() {
    let user = get_user();
    match TcpStream::connect("localhost:3333") {
        Ok(mut stream) => {
            println!("connected to port 3333");

            send_request(user, &mut stream);

            let mode = get_and_send_chat_mode(&mut stream);

            if mode == ChatMode::DIRECT {
                direct_mode(&mut stream);
            } else {

            }

            while get_user_input(String::from(">>")).read_string != "exit" {
                
            }
            close_connection(&mut stream);
        },
        Err(e) => {
            println!("failed to connect: {}", e);
        }
    }
}

fn direct_mode(stream: &mut TcpStream) {
    let mut buffer = vec!(0; 20 * User::NAME_SIZE);
    let chat_partner = match receive_string_vec(stream, &mut buffer){
        Some(names) => {
            print_string_vec(&names);
            select_chat_partner()
        },
        None => {
            String::from("")
        }
    };
    send_string(chat_partner.clone(), stream);

    let master_selection = receive_master_selection(stream).expect("no selection submitted");
    // TODO: disconnect from server
    if master_selection.is_own_ip {
        // TODO: spinup own server socket & wait for incoming connection
    } else {
        // TODO: open connection in new thread & start writing
    }
}

// TODO: switch to numbers + 'exit' to go back
fn select_chat_partner() -> String {
    let input = get_user_input(String::from("Select chat partner: "));
    return input.read_string;
}

fn choose_room(stream: &mut TcpStream) {
    let mut buffer = vec!(0; 20 * ChatRoom::NAME_SIZE);
    let room_name = match receive_string_vec(stream, &mut buffer) {
        Some(names) => {
            print_string_vec(&names);
            select_or_create_room()
        },
        None => {
            select_or_create_room()
        }
    };
    send_string(room_name, stream);
}

fn send_string(name: String, stream: &mut TcpStream) {
    let serialized_name = bincode::serialize(&name).unwrap();
    match stream.write(&serialized_name) {
        Ok(size) => println!("room name transmitted, wrote {} bytes", size),
        Err(e) => println!("error transmitting the room name: {}", e)
    }
}

// TODO: Better print it enumerated and pick with numbers
fn print_string_vec(names: &Vec<String>) {
    for name in names {
        println!("{}", name);
    }
}

fn select_or_create_room() -> String {
    let input_vec = read_line_vector();
    // annoying - i still dont really get the difference between / neccessity for str + String
    let fake_string = from_utf8(&input_vec).unwrap();
    String::from(fake_string)
}

fn get_and_send_chat_mode(stream: &mut TcpStream) -> ChatMode {
    let mode = get_chat_mode();
    let serialized = bincode::serialize(&mode).unwrap();
    match stream.write(&serialized) {
        Ok(size) => println!("chat mode transmitted, wrote {} bytes", size),
        Err(e) => println!("error transmitting the chat mode: {}", e)
    }
    mode
}

fn get_chat_mode() -> ChatMode {
    let mut mode: ChatMode = ChatMode::DIRECT;
    while match get_user_input(String::from("(1) direct chat, (2) chat rooms: ")).read_string.as_ref() {
        "1" => {
            mode = ChatMode::DIRECT;
            false
        },
        "2" => {
            mode = ChatMode::ROOM;
            false
        },
        _ => true
    } {}
    mode
}

fn receive_master_selection(stream: &mut TcpStream) -> Option<MasterSelectionResult> {
    let mut buffer = vec!(0; MasterSelectionResult::SIZE);
    match stream.read(&mut buffer) {
        Ok(size) => {
            if size == 0 {
                None
            } else {
                let result: MasterSelectionResult = bincode::deserialize(&buffer).unwrap();
                Some(result)
            }
        },
        Err(e) => {
            println!("error receiving master selection result: {}", e);
            None
        }
    }
}

fn receive_string_vec(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> Option<Vec<String>> {
    match stream.read(buffer) {
        Ok(size) => {
            if size == 0 {
                None
            } else {
                let names: Vec<String> = bincode::deserialize(&buffer).unwrap();
                Some(names)
            }
        },
        Err(e) => {
            println!("error receiving room names: {}", e);
            None
        }
    }
}

fn get_user() -> LoginRequest {
    let input = get_user_input(String::from("user name:"));
    LoginRequest{name: input.read_string}
}

fn send_request(user: LoginRequest, stream: &mut TcpStream) {
    let serialized = user.to_bytes();
    match stream.write(&serialized){
        Ok(size) => println!("wrote {} bytes", size),
        Err(e) => println!("failed to transmit user name: {}", e)
    };
}

fn close_connection(connection: &mut TcpStream) {
    match connection.shutdown(Shutdown::Both) {
        Ok(_) => {
            println!("client terminated");
        },
        Err(e) => {
            println!("failed to properly close connection to server: {}", e);
        }
    }
}

fn get_user_input(prompt_text: String) -> Input {
    print!("{}", prompt_text);
    std::io::stdout().flush().unwrap();
    let input_data = read_line_vector();
    let input_string = from_utf8(&input_data).unwrap().to_string();
    let input_length = input_string.len();
    Input{user_input: input_data, read_string: input_string, length: input_length}
}

fn read_line_vector() -> Vec<u8> {
    let mut input = Vec::new();
    std::io::stdin().lock().read_until(0xA, &mut input).unwrap();
    // remove (windows?) line break
    if input.len() >= 2 {
        input.truncate(input.len() - 2);
    }
    input
}
