extern crate bincode;
extern crate serde;
extern crate crossbeam_channel;
#[macro_use] extern crate serde_derive;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LoginRequest {
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Message {
    // TODO: Multi-Messages
    pub message: String
}

impl Message {
    pub const SIZE: usize = 1024;
}

pub struct User {
    pub id: u8,
    pub name: String,
    pub ip_address: String,
    pub sender: Option<crossbeam_channel::Sender<MasterSelectionResult>>
}

impl User {
    pub const NAME_SIZE: usize = 256;

    pub fn get_sender(&self) -> Option<crossbeam_channel::Sender<MasterSelectionResult>> {
        match &self.sender {
            Some(snd) => {
                let s = snd.clone();
                Some(s)
            },
            None => None
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum RemoteMessage {
    ChatModeMessage(ChatMode),
    LoginMessage(LoginRequest)
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ChatMode {
    DIRECT,
    ROOM,
    WAIT
}

impl ChatMode {
    // TODO: how are enums sized? addendum: i appear to send 4 bytes
    pub const SIZE: usize = 64;
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ChatRoom {
    pub id: u8,
    pub current_user: u8,
    pub name: String
}

impl ChatRoom {
    // TODO: validate()
    pub const SIZE: usize = 256;
    pub const NAME_SIZE: usize = 240;
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct MasterSelectionResult {
    pub chat_partner_name: String,
    pub target_ip: String,
    pub is_own_ip: bool
}

impl MasterSelectionResult {
    pub const SIZE: usize = 128;
}