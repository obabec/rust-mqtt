use crate::packet::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::ParseError;

use super::property::Property;

pub trait Packet<'a> {
    //fn new() -> dyn Packet<'a> where Self: Sized;

    fn encode(&mut self, buffer: &mut [u8]) -> usize;
    fn decode(&mut self, buff_reader: &mut BuffReader<'a>);

    // properties
    fn set_property_len(&mut self, value: u32);
    fn get_property_len(&mut self) -> u32;
    fn push_to_properties(&mut self, property: Property<'a>);

    // header
    fn set_fixed_header(&mut self, header: u8);
    fn set_remaining_len(&mut self, remaining_len: u32);

    fn decode_properties(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.set_property_len(buff_reader.read_variable_byte_int().unwrap());
        let mut x: u32 = 0;
        let mut prop: Result<Property, ParseError>;
        loop {
            let mut res: Property;
            prop = Property::decode(buff_reader);
            if let Ok(res) = prop {
                log::info!("Parsed property {:?}", res);
                x = x + res.len() as u32 + 1;
                self.push_to_properties(res);
            } else {
                // error handler
                log::error!("Problem during property decoding");
            }

            if x == self.get_property_len() {
                break;
            }
        }
    }

    fn decode_fixed_header(&mut self, buff_reader: &mut BuffReader) -> PacketType {
        let first_byte: u8 = buff_reader.read_u8().unwrap();
        self.set_fixed_header(first_byte);
        self.set_remaining_len(buff_reader.read_variable_byte_int().unwrap());
        return PacketType::from(first_byte);
    }
}
