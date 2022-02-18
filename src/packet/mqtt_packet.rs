use crate::utils::buffer_reader::BuffReader;

pub trait Packet<'a> {
    fn encode(& mut self, buffer: & mut [u8]);
    fn decode(& mut self, buff_reader: & mut BuffReader<'a>);
}