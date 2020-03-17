use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use std::thread;
use std::time;
use std::error::Error;
use common::{LoginRequest, ChatRoom, ChatMode, User, MasterSelectionResult, Message};
use crossbeam_channel::{Sender, Receiver};

extern crate bincode;
extern crate crossbeam_channel;
extern crate console;

mod ui;

enum InternMessage {
    SystemMessage(String),
    ErrorMessage(String),
    ChatMessage(MessageInfo)
}

struct MessageInfo {
    message_writer: String,
    message: String
}

/// Green color
macro_rules! sys_message {
    ($msg:expr => $snd:expr) => ($snd.send(InternMessage::SystemMessage(String::from($msg))).unwrap());
}

/// Red color
macro_rules! err_message {
    ($msg:expr => $snd:expr) => ($snd.send(InternMessage::ErrorMessage(String::from($msg))).unwrap());
}

/// Yellow color (to be changed)
macro_rules! chat_message {
    ($writer:expr,$msg:expr => $snd:expr) => {
        {
            let info = MessageInfo{message_writer: $writer, message: $msg};
            $snd.send(InternMessage::ChatMessage(info)).unwrap();
        }
    }
}

fn main() {
    let print_term = ui::create_ui();
    print_term.update_title("Rusty Chat");
    print_term.clear_screen_and_reset_cursor();

    let (snd, rcv): (Sender<InternMessage>, Receiver<InternMessage>) = crossbeam_channel::unbounded();
    thread::spawn(move || {
        print_messages_to_ui(rcv, print_term);
    });

    let term = ui::create_ui();
    term.move_to_input_pos();
    let user = get_user(&term, &snd);
    match TcpStream::connect("localhost:3333") {
        Ok(mut stream) => {
            sys_message!("connected to port 3333" => snd);

            send_request(user, &mut stream, &snd);

            let mode = get_and_send_chat_mode(&mut stream, &term, &snd);

            if mode == ChatMode::DIRECT {
                direct_mode(&mut stream, term, snd);
            } else if mode == ChatMode::WAIT {
                wait_mode(&mut stream, term, snd);
            } else {

            }
        },
        Err(e) => {
            err_message!(&format!("failed to connect: {}", e) => snd)
        }
    }
}

fn wait_mode(stream: &mut TcpStream, term: ui::UI, snd: Sender<InternMessage>) {
    let master_selection = receive_master_selection(stream, &snd).expect("no selection submitted");
    close_connection(stream, &snd);
    drop(stream);

    if master_selection.is_own_ip {
        start_master_server_direct(snd, master_selection.chat_partner_name, term);
    } else {
        sys_message!("waiting 5 seconds before connecting to elected master server" => snd);
        thread::sleep(time::Duration::from_millis(5000));
        connect_to_master(master_selection.target_ip, master_selection.chat_partner_name, term, snd);
    }
}

fn direct_mode(stream: &mut TcpStream, term: ui::UI, snd: Sender<InternMessage>) {
    let mut buffer = vec!(0; 20 * User::NAME_SIZE);
    // TODO: whats up with that empty name?
    let chat_partner = match receive_string_vec(stream, &mut buffer, &snd){
        Some(names) => {
            print_string_vec(&names, &snd);
            select_chat_partner(&term, &snd)
        },
        None => {
            String::from("")
        }
    };
    // TODO: better to return Result? saves me handing in that sender
    send_string(chat_partner.clone(), stream, &snd);

    // TODO: handle bad case
    let master_selection = receive_master_selection(stream, &snd).expect("no selection submitted");
    close_connection(stream, &snd);
    drop(stream);

    if master_selection.is_own_ip {
        start_master_server_direct(snd, chat_partner, term);
    } else {
        sys_message!("waiting 5 seconds before connecting to elected master server" => snd);
        thread::sleep(time::Duration::from_millis(5000));
        connect_to_master(master_selection.target_ip, chat_partner, term, snd);
    }
}
fn connect_to_master(master_ip: String, chat_partner: String, term: ui::UI, snd: Sender<InternMessage>) {
    let server_address = master_ip + ":3334";
    match TcpStream::connect(server_address) {
        Ok(mut stream) => {
            term.update_title(&chat_partner);

            create_network_listener(&snd, &chat_partner, &stream);

            user_input_loop(snd, &mut stream, &term);
        },
        Err(e) => {
           err_message!(&format!("failed to connect: {}", e) => snd)
        }
    }
}

fn start_master_server_direct(sender: Sender<InternMessage>, chat_partner: String, term: ui::UI) {
    let listener = TcpListener::bind("0.0.0.0:3334").unwrap();
    // TODO: do i really wait for multiple connections here?
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                term.update_title(&chat_partner);

                sys_message!(&format!("{} connected successfully", &chat_partner) => sender);

                create_network_listener(&sender, &chat_partner, &stream);

                let mut write_stream = stream.try_clone().unwrap();
                let input_clone = sender.clone();
                user_input_loop(input_clone, &mut write_stream, &term);
            },
            Err(e) => err_message!(&format!("Error: {}", e) => sender)
        }
    }
}

/// Spins up a thread which listens on incoming messages.
/// Each chat participant should have his own listener thread at the moment.
fn create_network_listener(sender: &Sender<InternMessage>, chat_partner: &String, stream: &TcpStream) {
    let network_sender = sender.clone();
    let mut read_stream = stream.try_clone().unwrap();
    let partner_clone = chat_partner.clone();
    thread::spawn(move || {
        read_incoming_messages(network_sender, &mut read_stream, partner_clone)
    });
}

fn user_input_loop(sender: Sender<InternMessage>, stream: &mut TcpStream, term: &ui::UI) {
    term.move_to_input_pos();
    while match term.read_line() {
        input => {
            let mut continue_chat = true;
            if &input == "/exit" {
                chat_message!(String::from(""), String::from("/exit") => sender);
                close_connection(stream, &sender);

                continue_chat = false;
            } else {
                let message_to_send = Message{message: input.clone()};
                send_message(message_to_send, stream);

                chat_message!(String::from("me"), input => sender);
            }
            continue_chat
        }
    } {}
}

/// Reads from the given TcpStream and processes the incoming messages.
/// sender: PrintLoop-Sender
/// stream: Stream whose messages we want processed
/// chat_partner: Name of our chat partner
fn read_incoming_messages(sender: Sender<InternMessage>, stream: &mut TcpStream, chat_partner: String) {
    let mut buffer = vec!(0; Message::SIZE);
    while match stream.read(&mut buffer) {
        Ok(size) => {
            let mut continue_reading = true;
            if size == 0 {
                sys_message!("/terminated" => sender);
                continue_reading = false;
            } else {
                let msg = deserialize_message(&buffer);
                chat_message!(chat_partner.clone(), msg.message => sender);
            }
            continue_reading
        },
        Err(e) => {
            err_message!(&format!("Error: {}, Caused By: {}", e.description(), e.source().unwrap()) => sender);
            false
        }
    } {}
}

fn send_message(msg: Message, stream: &mut TcpStream) {
    let serialized = bincode::serialize(&msg).unwrap();
    stream.write(&serialized).expect("catastrophic failure");
}

fn deserialize_message(buffer: &[u8]) -> Message {
    bincode::deserialize(buffer).unwrap()
}

/// Prints to the UI. Loops over the given receiver.
/// Every component that wants to write something on the screen needs a sender to this channel.
/// receiver: Consuming end of a multi producer channel
/// term: The UI. This print loop owns it.
fn print_messages_to_ui(receiver: Receiver<InternMessage>, mut term: ui::UI) {
    while match receiver.recv() {
        Ok(message) => {
            let mut continue_loop = false;

            match message {
                InternMessage::ChatMessage(info) => {
                    if &info.message != "/exit" {
                        term.write_info_message(&format!("{}: {}", &info.message_writer, &info.message));
                        continue_loop = true
                    }
                },
                InternMessage::SystemMessage(text) => {
                    if &text == "/terminated" {
                        term.write_sys_message("connection with your chat partner was terminated");
                        continue_loop = false
                    } else {
                        term.write_sys_message(&text);
                        continue_loop = true
                    }
                },
                InternMessage::ErrorMessage(text) => {
                    term.write_err_message(&text);
                    continue_loop = true
                }
            }
            
            continue_loop
        },
        Err(e) => {
            term.write_err_message(&format!("ERROR: {}", e));
            false
        }
    } {}
}

// TODO: switch to numbers + 'exit' to go back
fn select_chat_partner(term: &ui::UI, snd: &Sender<InternMessage>) -> String {
    sys_message!("Select chat partner: " => snd);
    term.read_line()
}

fn send_string(name: String, stream: &mut TcpStream, snd: &Sender<InternMessage>) {
    let serialized_name = bincode::serialize(&name).unwrap();
    match stream.write(&serialized_name) {
        Ok(size) => sys_message!(&format!("string transmitted, wrote {} bytes", size) => snd),
        Err(e) => err_message!(&format!("error transmitting the string: {}", e) => snd)
    }
}

// TODO: Better print it enumerated and pick with numbers
fn print_string_vec(names: &Vec<String>, snd: &Sender<InternMessage>) {
    for name in names {
        sys_message!(&format!("{}", name) => snd);
    }
}

fn get_and_send_chat_mode(stream: &mut TcpStream, term: &ui::UI, snd: &Sender<InternMessage>) -> ChatMode {
    let mode = get_chat_mode(term, snd);
    let serialized = bincode::serialize(&mode).unwrap();
    match stream.write(&serialized) {
        Ok(size) => sys_message!(&format!("chat mode transmitted, wrote {} bytes", size) => snd),
        Err(e) => err_message!(&format!("error transmitting the chat mode: {}", e) => snd)
    }
    mode
}

fn get_chat_mode(term: &ui::UI, snd: &Sender<InternMessage>) -> ChatMode {
    let mut mode: ChatMode = ChatMode::DIRECT;
    // term.move_to_input_pos();
    sys_message!("(1) direct chat, (2) chat rooms, (3) wait: " => snd);
    while match term.read_line().as_str() {
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
            err_message!(x => snd);
            true
        }
    } {}
    mode
}

fn receive_master_selection(stream: &mut TcpStream, snd: &Sender<InternMessage>) -> Option<MasterSelectionResult> {
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
            err_message!(&format!("error receiving master selection result: {}", e) => snd);
            None
        }
    }
}

fn receive_string_vec(stream: &mut TcpStream, buffer: &mut Vec<u8>, snd: &Sender<InternMessage>) -> Option<Vec<String>> {
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
            err_message!(&format!("error receiving room names: {}", e) => snd);
            None
        }
    }
}

fn get_user(term: &ui::UI, snd: &Sender<InternMessage>) -> LoginRequest {
    sys_message!("please enter you name" => snd);
    let input = term.read_line();
    LoginRequest{name: input}
}

fn send_request(user: LoginRequest, stream: &mut TcpStream, snd: &Sender<InternMessage>) {
    let serialized = user.to_bytes();
    match stream.write(&serialized){
        Ok(size) => sys_message!(&format!("wrote {} bytes", size) => snd),
        Err(e) => err_message!(&format!("failed to transmit user name: {}", e) => snd)
    };
}

fn close_connection(connection: &mut TcpStream, snd: &Sender<InternMessage>) {
    match connection.shutdown(Shutdown::Both) {
        Ok(_) => {
            sys_message!(&format!("connection with {} terminated", connection.peer_addr().unwrap()) => snd);
        },
        Err(e) => {
            err_message!(&format!("failed to properly close connection to server: {}", e) => snd);
        }
    }
}
