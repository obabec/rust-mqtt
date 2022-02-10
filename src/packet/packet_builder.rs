use super::mqtt_packet::Packet;
use super::packet_type::PacketType;
use super::property::Property;
use crate::utils::buffer_reader::*;

// Je potreba vytvori

// metody packet buildery budou prijimat jako parametr buff reader, z ktereho bude postupne parsovat 

pub struct PacketBuilder<'a> {
    currentPacket: Packet<'a>,
}

impl<'a> PacketBuilder<'a>  {

    pub fn new(packet: Packet<'a>) -> Self {
        Self{ currentPacket: packet }
    }

    pub fn build(&self) -> &Packet<'a> {
        return &self.currentPacket;
    }

    pub fn decode(&self, buffer: &'a mut [u8]) -> &Packet<'a> {
        return &self.currentPacket;
    }

    pub fn addPacketType(& mut self, new_packet_type: PacketType) {
        self.currentPacket.fixed_header = self.currentPacket.fixed_header & 0x0F;
        self.currentPacket.fixed_header = self.currentPacket.fixed_header | <PacketType as Into<u8>>::into(new_packet_type);
    }

    pub fn addFlags(& mut self, dup: bool, qos: u8, retain: bool) {
        let cur_type: u8 = self.currentPacket.fixed_header & 0xF0;
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
        self.currentPacket.fixed_header = cur_type | flags;
    }

    pub fn completePacket(& mut self) {
        // Tutaj se cely packet dokonci - spocita se remaining len co chybi v hlavicce atd...
    }

    pub fn decode_packet(& mut self, buff_reader: & mut BuffReader<'a>) {
        self.decodeFixedHeader(buff_reader);
        if self.currentPacket.fixed_header & 0xF0 == (PacketType::Connect).into() {
            self.decodeControllPacket(buff_reader);
        }
    }

    pub fn decodeFixedHeader(& mut self, buff_reader: & mut BuffReader) -> PacketType {
        let first_byte: u8 = buff_reader.readU8().unwrap();
        self.currentPacket.fixed_header = first_byte;
        self.currentPacket.remain_len = buff_reader.readVariableByteInt().unwrap();
        return PacketType::from(self.currentPacket.fixed_header);
    }

    pub fn decodeControllPacket(& mut self, buff_reader: & mut BuffReader<'a>) {
        self.currentPacket.packet_identifier = 0;
        self.currentPacket.protocol_name_len = buff_reader.readU16().unwrap();
        self.currentPacket.protocol_name = buff_reader.readU32().unwrap();
        self.currentPacket.protocol_version = buff_reader.readU8().unwrap();
        self.currentPacket.connect_flags = buff_reader.readU8().unwrap();
        self.currentPacket.keep_alive = buff_reader.readU16().unwrap();
        self.currentPacket.property_len = buff_reader.readVariableByteInt().unwrap();
        let mut x: u32 = 0;
        let mut prop: Result<Property, ProperyParseError>;
        let mut res: Property;
        loop {
            prop = Property::decode(buff_reader);
            if let Ok(res) = prop {
                x = x + res.len() as u32 + 1;
                self.currentPacket.properties.push(res);
            }
            
            /*if prop.is_ok() {
            
            } else {
                log::error!("Decoding property did not went well!");
            }*/

            
            if x == self.currentPacket.property_len {
                break;
            }
        }
    }
}