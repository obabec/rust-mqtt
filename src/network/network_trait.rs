
pub trait Network {
    fn send(buffer: & mut [u8]);
    fn receive(buffer: & mut [u8]);
}