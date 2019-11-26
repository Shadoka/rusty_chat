use std::net::{TcpStream, Shutdown};
use std::io::{Read, Write, BufRead};
use std::str::from_utf8;
use common::{LoginRequest, ChatRoom, ChatMode};

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

            get_and_send_chat_mode(&mut stream);

            while get_user_input(String::from(">>")).read_string != "exit" {
                
            }
            close_connection(&mut stream);
        },
        Err(e) => {
            println!("failed to connect: {}", e);
        }
    }
}

fn choose_room(stream: &mut TcpStream) {
    let room_name = match receive_room_names(stream) {
        Some(names) => {
            print_room_names(&names);
            select_or_create_room()
        },
        None => {
            select_or_create_room()
        }
    };
    send_room_name(room_name, stream);
}

fn send_room_name(name: String, stream: &mut TcpStream) {
    let serialized_name = bincode::serialize(&name).unwrap();
    match stream.write(&serialized_name) {
        Ok(size) => println!("room name transmitted, wrote {} bytes", size),
        Err(e) => println!("error transmitting the room name: {}", e)
    }
}

fn print_room_names(names: &Vec<String>) {
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

fn get_and_send_chat_mode(stream: &mut TcpStream) {
    let mode = get_chat_mode();
    let serialized = bincode::serialize(&mode).unwrap();
    match stream.write(&serialized) {
        Ok(size) => println!("chat mode transmitted, wrote {} bytes", size),
        Err(e) => println!("error transmitting the chat mode: {}", e)
    }
}

fn get_chat_mode() -> ChatMode {
    let mut mode: ChatMode = ChatMode::DIRECT;
    while match get_user_input(String::from("(1) direct chat, (2) chat rooms")).read_string.as_ref() {
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

fn receive_room_names(stream: &mut TcpStream) -> Option<Vec<String>> {
    let mut buffer = vec!(0; 20 * ChatRoom::NAME_SIZE);
    match stream.read(&mut buffer) {
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
