use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use heapless::Vec;

use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::ParseError;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

pub struct ConnectPacket<'a, const MAX_PROPERTIES: usize, const MAX_WILL_PROPERTIES: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,
    pub protocol_name_len: u16,
    pub protocol_name: u32,
    pub protocol_version: u8,
    pub connect_flags: u8,
    pub keep_alive: u16,
    // property len
    pub property_len: u32,

    // properties
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,

    //payload
    pub client_id: EncodedString<'a>,
    // property len
    pub will_property_len: u32,
    pub will_properties: Vec<Property<'a>, MAX_WILL_PROPERTIES>,
    pub will_topic: EncodedString<'a>,
    pub will_payload: BinaryData<'a>,
    pub username: EncodedString<'a>,
    pub password: BinaryData<'a>,
}

impl<'a, const MAX_PROPERTIES: usize, const MAX_WILL_PROPERTIES: usize>
    ConnectPacket<'a, MAX_PROPERTIES, MAX_WILL_PROPERTIES>
{
    pub fn clean() -> Self {
        let mut x = Self {
            fixed_header: PacketType::Connect.into(),
            remain_len: 0,
            protocol_name_len: 4,
            protocol_name: 0x4d515454,
            protocol_version: 5,
            connect_flags: 0x02,
            keep_alive: 60,
            property_len: 3,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            client_id: EncodedString::new(),
            will_property_len: 0,
            will_properties: Vec::<Property<'a>, MAX_WILL_PROPERTIES>::new(),
            will_topic: EncodedString::new(),
            will_payload: BinaryData::new(),
            username: EncodedString::new(),
            password: BinaryData::new(),
        };

        let y = Property::ReceiveMaximum(20);
        x.properties.push(y);
        x.client_id.len = 0;
        return x;
    }

    pub fn get_reason_code(&self) {
        log::info!("Getting reason code!");
    }

    pub fn add_packet_type(&mut self, new_packet_type: PacketType) {
        self.fixed_header = self.fixed_header & 0x0F;
        self.fixed_header = self.fixed_header | <PacketType as Into<u8>>::into(new_packet_type);
    }

    pub fn add_flags(&mut self, dup: bool, qos: u8, retain: bool) {
        let cur_type: u8 = self.fixed_header & 0xF0;
        if cur_type != 0x30 {
            log::error!("Cannot add flags into packet with other than PUBLISH type");
            return;
        }
        let mut flags: u8 = 0x00;
        if dup {
            flags = flags | 0x08;
        }
        if qos == 1 {
            flags = flags | 0x02;
        }
        if qos == 2 {
            flags = flags | 0x04;
        }
        if retain {
            flags = flags | 0x01;
        }
        self.fixed_header = cur_type | flags;
    }
}
impl<'a, const MAX_PROPERTIES: usize, const MAX_WILL_PROPERTIES: usize> Packet<'a>
    for ConnectPacket<'a, MAX_PROPERTIES, MAX_WILL_PROPERTIES>
{
    fn new() -> Self {
        todo!()
    }

    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] =
            VariableByteIntegerEncoder::encode(self.property_len).unwrap();
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        // 12 = protocol_name_len + protocol_name + protocol_version + connect_flags + keep_alive + client_id_len
        rm_ln = rm_ln + property_len_len as u32 + 12;

        if self.connect_flags & 0x04 == 1 {
            let wil_prop_len_enc =
                VariableByteIntegerEncoder::encode(self.will_property_len).unwrap();
            let wil_prop_len_len = VariableByteIntegerEncoder::len(wil_prop_len_enc);
            rm_ln = rm_ln
                + wil_prop_len_len as u32
                + self.will_property_len as u32
                + self.will_topic.len as u32
                + self.will_payload.len as u32;
        }

        if self.connect_flags & 0x80 == 1 {
            rm_ln = rm_ln + self.username.len as u32;
        }

        if self.connect_flags & 0x40 == 1 {
            rm_ln = rm_ln + self.password.len as u32;
        }

        buff_writer.write_u8(self.fixed_header);
        buff_writer.write_variable_byte_int(rm_ln);

        buff_writer.write_u16(self.protocol_name_len);
        buff_writer.write_u32(self.protocol_name);
        buff_writer.write_u8(self.protocol_version);
        buff_writer.write_u8(self.connect_flags);
        buff_writer.write_u16(self.keep_alive);
        buff_writer.write_variable_byte_int(self.property_len);
        buff_writer.encode_properties::<MAX_PROPERTIES>(&self.properties);
        buff_writer.write_string_ref(&self.client_id);

        if self.connect_flags & 0x04 == 1 {
            buff_writer.write_variable_byte_int(self.will_property_len);
            buff_writer.encode_properties(&self.will_properties);
            buff_writer.write_string_ref(&self.will_topic);
            buff_writer.write_binary_ref(&self.will_payload);
        }

        if self.connect_flags & 0x80 == 1 {
            buff_writer.write_string_ref(&self.username);
        }

        if self.connect_flags & 0x40 == 1 {
            buff_writer.write_binary_ref(&self.password);
        }

        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        log::error!("Decode function is not available for control packet!")
    }

    fn set_property_len(&mut self, value: u32) {
        self.property_len = value;
    }

    fn get_property_len(&mut self) -> u32 {
        return self.property_len;
    }

    fn push_to_properties(&mut self, property: Property<'a>) {
        self.properties.push(property);
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }

    fn decode_properties(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.property_len = buff_reader.read_variable_byte_int().unwrap();
        let mut x: u32 = 0;
        let mut prop: Result<Property, ParseError>;
        loop {
            let mut res: Property;
            prop = Property::decode(buff_reader);
            if let Ok(res) = prop {
                log::info!("Parsed property {:?}", res);
                x = x + res.len() as u32 + 1;
                self.properties.push(res);
            } else {
                // error handlo
                log::error!("Problem during property decoding");
            }

            if x == self.property_len {
                break;
            }
        }
    }
}
