pub struct Packet<'a> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    header_control: u8,
    // 1 - 4 B
    remain_len: u32,

    // variable header
    //optional
    packet_identifier: u16,
    // property len
    property_len: u32,
    // properties
    properties: &'a mut [u8],
    // Payload of message
    payload: &'a mut [u8]
}

impl<'a> Packet {
    pub fn new(header_control: u8, remain_len: u32, packet_identifier: u16, property_len: u32, properties: &'a mut [u8], payload: &'a mut [u8]) -> Self {
        Self { header_control, remain_len, packet_identifier, property_len , properties, payload}
    }

    pub fn encode(&self) {
        log::info!("Encoding!");
    }

    pub fn get_reason_code(&self) {
        log::info!("Getting reason code!");
    }
}