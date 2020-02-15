use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use common::{LoginRequest, ChatRoom, ChatMode, User, MasterSelectionResult, Message};
use crossbeam_channel::{Sender, Receiver};

extern crate bincode;
extern crate crossbeam_channel;
extern crate console;

mod ui;

enum ClientState {
    PreLogin,
    ModeSelection,
    
}

enum InternMessage {
    SystemMessage(String),
    ChatMessage(MessageInfo)
}

struct MessageInfo {
    message_writer: String,
    message: String
}

fn main() {
    let term = ui::create_ui();
    term.clear_screen_and_reset_cursor();

    let threadsafe_term = Arc::new(Mutex::new(term));

    let user = get_user(&threadsafe_term);
    match TcpStream::connect("localhost:3333") {
        Ok(mut stream) => {
            ui::write_sys_message(&threadsafe_term, "connected to port 3333");

            send_request(user, &mut stream, &threadsafe_term);

            let mode = get_and_send_chat_mode(&mut stream, &threadsafe_term);

            if mode == ChatMode::DIRECT {
                direct_mode(&mut stream, threadsafe_term);
            } else if mode == ChatMode::WAIT {
                wait_mode(&mut stream, threadsafe_term);
            } else {

            }
        },
        Err(e) => {
            ui::write_err_message(&threadsafe_term, format!("failed to connect: {}", e).as_str())
        }
    }
}

fn wait_mode(stream: &mut TcpStream, term: Arc<Mutex<ui::UI>>) {
    let master_selection = receive_master_selection(stream).expect("no selection submitted");
    close_connection(stream, &term);
    drop(stream);

    if master_selection.is_own_ip {
        start_direct_server(master_selection.chat_partner_name, term);
    } else {
        ui::write_info_message(&term, "waiting 5 seconds before connecting to elected master server");
        thread::sleep(time::Duration::from_millis(5000));
        connect_to_master(master_selection.target_ip, master_selection.chat_partner_name, term);
    }
}

fn direct_mode(stream: &mut TcpStream, term: Arc<Mutex<ui::UI>>) {
    let mut buffer = vec!(0; 20 * User::NAME_SIZE);
    // TODO: whats up with that empty name?
    let chat_partner = match receive_string_vec(stream, &mut buffer){
        Some(names) => {
            print_string_vec(&names);
            select_chat_partner(&term)
        },
        None => {
            String::from("")
        }
    };
    send_string(chat_partner.clone(), stream, &term);

    // TODO: handle bad case
    let master_selection = receive_master_selection(stream).expect("no selection submitted");
    close_connection(stream, &term);
    drop(stream);

    if master_selection.is_own_ip {
        start_direct_server(chat_partner, term);
    } else {
        ui::write_info_message(&term, "waiting 5 seconds before connecting to elected master server");
        thread::sleep(time::Duration::from_millis(5000));
        connect_to_master(master_selection.target_ip, chat_partner, term);
    }
}

fn connect_to_master(master_ip: String, chat_partner: String, term: Arc<Mutex<ui::UI>>) {
    let server_address = master_ip + ":3334";
    match TcpStream::connect(server_address) {
        Ok(mut stream) => {
            let (snd, rcv): (Sender<InternMessage>, Receiver<InternMessage>) = crossbeam_channel::unbounded();
            let term_clone = term.clone();
            thread::spawn(move || {
                prepare_print_loop_simple(rcv, chat_partner, term_clone)
            });
            user_input_loop(snd, &mut stream, &term);
        },
        Err(e) => {
            ui::write_err_message(&term, format!("failed to connect: {}", e).as_str())
        }
    }
}

fn start_direct_server(chat_partner: String, term: Arc<Mutex<ui::UI>>) {
    let (snd, rcv): (Sender<InternMessage>, Receiver<InternMessage>) = crossbeam_channel::unbounded();
    let term_clone = term.clone();
    let partner_clone = chat_partner.clone();
    thread::spawn(move || {
        prepare_print_loop_master(rcv, partner_clone, term_clone)
    });
    master_server_direct(snd, chat_partner, &term);
}

fn master_server_direct(sender: Sender<InternMessage>, chat_partner: String, term: &Arc<Mutex<ui::UI>>) {
    let listener = TcpListener::bind("0.0.0.0:3334").unwrap();
    // TODO: do i really wait for multiple connections here?
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let reader_clone = sender.clone();
                // TODO: we uhhh.. dont have our name :D
                // let mut message = MessageInfo{message_writer: String::from("myself"), message: String::from("")};

                let message = InternMessage::SystemMessage(String::from("/connected"));
                sender.send(message).unwrap();

                let mut read_stream = stream.try_clone().unwrap();
                let partner_clone = chat_partner.clone();
                thread::spawn(move || {
                    read_incoming_messages(reader_clone, &mut read_stream, partner_clone)
                });

                let mut write_stream = stream.try_clone().unwrap();
                let input_clone = sender.clone();
                user_input_loop(input_clone, &mut write_stream, term);
            },
            Err(e) => ui::write_err_message(&term, &format!("Error: {}", e))
        }
    }
}

// TODO: do the real input loop here
fn user_input_loop(sender: Sender<InternMessage>, stream: &mut TcpStream, term: &Arc<Mutex<ui::UI>>) {
    while match ui::read_line(term, ">>") {
        input => {
            let mut continue_chat = true;
            if &input == "/exit" {
                let info = MessageInfo{message_writer: String::from("myself"), message: String::from("/exit")};
                sender.send(InternMessage::ChatMessage(info)).unwrap();
                ui::reset_input_line(&term);
                close_connection(stream, &term);

                continue_chat = false;
            } else {
                let message_to_send = Message{message: input.clone()};
                send_message(message_to_send, stream);

                let info = MessageInfo{message_writer: String::from("myself"), message: input};
                sender.send(InternMessage::ChatMessage(info)).unwrap();
                ui::reset_input_line(&term);
            }
            continue_chat
        }
    } {}
}

fn send_message(msg: Message, stream: &mut TcpStream) {
    let serialized = bincode::serialize(&msg).unwrap();
    stream.write(&serialized).expect("catastrophic failure");
}

fn read_incoming_messages(sender: Sender<InternMessage>, stream: &mut TcpStream, chat_partner: String) {
    let mut buffer = vec!(0; Message::SIZE);
    while match stream.read(&mut buffer) {
        Ok(size) => {
            let mut continue_reading = true;
            if size == 0 {
                // TODO: chat partner quit, send it to print loop
                continue_reading = false;
            } else {
                let msg = deserialize_message(&buffer);
                let info = MessageInfo{message: msg.message, message_writer: chat_partner.clone()};
                sender.send(InternMessage::ChatMessage(info)).unwrap();
            }
            continue_reading
        },
        Err(_) => {
            // TODO: send info to print loop
            false
        }
    } {}
}

fn deserialize_message(buffer: &[u8]) -> Message {
    bincode::deserialize(buffer).unwrap()
}

fn prepare_print_loop_simple(receiver: Receiver<InternMessage>, chat_partner: String, term: Arc<Mutex<ui::UI>>) {
    ui::write_sys_message(&term, &format!("connected to {}!", chat_partner));
    print_messages_to_ui(receiver, term);
}

fn prepare_print_loop_master(receiver: Receiver<InternMessage>, chat_partner: String, term: Arc<Mutex<ui::UI>>) {
    ui::write_sys_message(&term, "waiting for chat partner to connect...");
    let con_success = match receiver.recv() {
        Ok(msg) => {
            match msg {
                InternMessage::SystemMessage(message) => &message == "/connected",
                _ => false
            }
        },
        Err(_) => false
    };

    if !con_success {
        ui::write_err_message(&term, "chat partner was unable to connect successfully");
        return ();
    } else {
        ui::write_sys_message(&term, &format!("{} connected successfully!", chat_partner));
    }

    print_messages_to_ui(receiver, term);
}

fn print_messages_to_ui(receiver: Receiver<InternMessage>, term: Arc<Mutex<ui::UI>>) {
    while match receiver.recv() {
        Ok(message) => {
            let mut continue_loop = false;

            match message {
                InternMessage::ChatMessage(info) => {
                    if &info.message != "/exit" {
                        ui::write_info_message(&term, &info.message);
                        println!("{}: {}", info.message_writer, info.message);
                        continue_loop = true
                    }
                },
                InternMessage::SystemMessage(text) => {
                    if &text == "/terminated" {
                        ui::write_sys_message(&term, "connection with your chat partner was terminated");
                    } else {
                        ui::write_sys_message(&term, &text);
                        continue_loop = true;
                    }
                }
            }
            
            continue_loop
        },
        Err(e) => {
            ui::write_err_message(&term, &format!("ERROR: {}", e));
            false
        }
    } {}
}

// TODO: switch to numbers + 'exit' to go back
fn select_chat_partner(term: &Arc<Mutex<ui::UI>>) -> String {
    let input = ui::read_line(term, "Select chat partner: ");
    return input;
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
    //send_string(room_name, stream);
}

fn send_string(name: String, stream: &mut TcpStream, term: &Arc<Mutex<ui::UI>>) {
    let serialized_name = bincode::serialize(&name).unwrap();
    match stream.write(&serialized_name) {
        Ok(size) => ui::write_sys_message(term, &format!("string transmitted, wrote {} bytes", size)),
        Err(e) => ui::write_err_message(term, &format!("error transmitting the string: {}", e))
    }
}

// TODO: Better print it enumerated and pick with numbers
fn print_string_vec(names: &Vec<String>) {
    for name in names {
        println!("{}", name);
    }
}

fn select_or_create_room() -> String {
    // let input_vec = read_line("");
    // annoying - i still dont really get the difference between / neccessity for str + String
    //let fake_string = from_utf8(&input_vec).unwrap();
    String::from("")
}

fn get_and_send_chat_mode(stream: &mut TcpStream, term: &Arc<Mutex<ui::UI>>) -> ChatMode {
    let mode = get_chat_mode(term);
    let serialized = bincode::serialize(&mode).unwrap();
    match stream.write(&serialized) {
        Ok(size) => ui::write_sys_message(term, &format!("chat mode transmitted, wrote {} bytes", size)),
        Err(e) => ui::write_err_message(term, &format!("error transmitting the chat mode: {}", e))
    }
    mode
}

fn get_chat_mode(term: &Arc<Mutex<ui::UI>>) -> ChatMode {
    let mut mode: ChatMode = ChatMode::DIRECT;
    while match ui::read_line(term, "(1) direct chat, (2) chat rooms, (3) wait: ").as_str() {
        "1" => {
            mode = ChatMode::DIRECT;
            false
        },
        "2" => {
            mode = ChatMode::ROOM;
            false
        },
        "3" => {
            mode = ChatMode::WAIT;
            false
        },
        x => {
            ui::write_err_message(term, x);
            true
        }
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

fn get_user(term: &Arc<Mutex<ui::UI>>) -> LoginRequest {
    // let input = get_user_input(String::from("user name:"));
    let input = ui::read_line(term, "user name:");
    LoginRequest{name: input}
}

fn send_request(user: LoginRequest, stream: &mut TcpStream, term: &Arc<Mutex<ui::UI>>) {
    let serialized = user.to_bytes();
    match stream.write(&serialized){
        Ok(size) => ui::write_sys_message(term, &format!("wrote {} bytes", size)),
        Err(e) => ui::write_err_message(term, &format!("failed to transmit user name: {}", e))
    };
}

fn close_connection(connection: &mut TcpStream, term: &Arc<Mutex<ui::UI>>) {
    match connection.shutdown(Shutdown::Both) {
        Ok(_) => {
            ui::write_sys_message(term, &format!("connection with {} terminated", connection.peer_addr().unwrap()));
        },
        Err(e) => {
            ui::write_err_message(term, &format!("failed to properly close connection to server: {}", e));
        }
    }
}
