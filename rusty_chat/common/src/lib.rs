extern crate bincode;
extern crate serde;
#[macro_use] extern crate serde_derive;

// use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LoginRequest {
    pub name_length: u8,
    pub name: String
}

impl LoginRequest {
    // TODO: call some kind of validate() either at creation or transmission time
    pub const SIZE: usize = 256;

    pub fn to_bytes(&self) -> Vec<u8>{
        let result: Vec<u8> = bincode::serialize(self).unwrap();
        result
    }
}

pub struct Message {
    pub message_count: u8,
    pub current_message_id: u8,
    pub message: [u8; 1008],
}

pub struct User {
    pub name_length: u8,
    pub name: [u8; 248],
}

pub struct ChatRoom {
    pub current_user: u8,
    pub name_length: u8,
    pub name: [u8; 248],
}