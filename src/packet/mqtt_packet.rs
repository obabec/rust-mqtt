pub struct Packet<'a> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub header_control: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    // variable header
    //optional  prida se pouze u packetu ve kterych ma co delat 
    pub packet_identifier: u16,
    // property len
    pub property_len: u32,
    // properties
    pub properties: &'a mut [u8],
    // Payload of message
    pub payload: &'a mut [u8]
}

impl<'a> Packet<'a> {
    pub fn new(header_control: u8, remain_len: u32, packet_identifier: u16, property_len: u32, 
                properties: &'a mut [u8], payload: &'a mut [u8]) -> Self {
        Self { header_control, remain_len, packet_identifier, property_len, properties, payload }
    }

    pub fn clean(properties: &'a mut [u8], payload: &'a mut [u8]) -> Self {
        Self{ header_control: 0, remain_len: 0, packet_identifier: 0, property_len: 0, properties, payload }
    }

    pub fn encode(&self) {
        log::info!("Encoding!");
    }

    pub fn get_reason_code(&self) {
        log::info!("Getting reason code!");
    }
}