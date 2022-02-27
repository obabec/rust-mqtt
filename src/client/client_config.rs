use crate::packet::publish_packet::QualityOfService;
use crate::utils::buffer_reader::{BinaryData, EncodedString};

pub struct ClientConfig<'a> {
    pub qos: QualityOfService,
    pub username_flag: bool,
    pub username: EncodedString<'a>,
    pub password_flag: bool,
    pub password: BinaryData<'a>
}

impl ClientConfig<'a> {
    pub fn new() -> Self {
        Self {
            qos: QualityOfService::QoS0,
            username_flag: false,
            username: EncodedString::new(),
            password_flag: false,
            password: BinaryData::new(),
        }
    }

    pub fn add_qos(& mut self, qos: QualityOfService) {
        self.qos = qos;
    }

    pub fn add_username(& mut self, username: &'a str) {
        let mut username_s: EncodedString = EncodedString::new();
        username_s.string = username;
        username_s.len = username.len() as u16;
        self.username_flag = true;
        self.username = username_s;
    }

    pub fn add_password(& mut self, password: &'a str) {
        let mut password_s: BinaryData = BinaryData::new();
        password_s.bin = password.as_bytes();
        password_s.len = password_s.bin.len() as u16;
        self.password = password_s;
        self.password_flag = true;
    }
}