use std::net::{TcpStream, Shutdown};
use std::io::{Read, Write, BufRead};
use std::str::from_utf8;
use common::{LoginRequest};

const BUFFER_SIZE: usize = 1024;

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
            transmit_user(user, &mut stream);
            let mut input = get_user_input(String::from(">>"));
            while input.read_string != "exit" {
                stream.write(&*input.user_input).unwrap();
                println!("message sent, awaiting reply");
                let mut data = [0 as u8; BUFFER_SIZE];
                match stream.read(&mut data) {
                    Ok(_) => {
                        let mut received_data = data.to_vec();
                        received_data.truncate(input.length);
                        if received_data == input.user_input {
                            println!("reply is ok");
                        } else {
                            let text = from_utf8(&*received_data).unwrap();
                            println!("invalid reply: {}", text);
                        }
                    },
                    Err(e) => {
                    println!("failed to receive data: {}", e);
                    }
                }
                input = get_user_input(String::from(">>"));
            }
            close_connection(&mut stream);
        },
        Err(e) => {
            println!("failed to connect: {}", e);
        }
    }
}

fn get_user() -> LoginRequest {
    let input = get_user_input(String::from("user name:"));
    LoginRequest{name: input.read_string}
}

fn transmit_user(user: LoginRequest, stream: &mut TcpStream) {
    let serialized = user.to_bytes();
    match stream.write(&serialized){
        Ok(size) => println!("wrote {} bytes", size),
        Err(e) => println!("failed to transmit name: {}", e)
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
