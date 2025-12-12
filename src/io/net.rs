use crate::eio::{Read, Write};

/// Underlying transport of MQTT. Must provide an ordered, lossless, stream of bytes from Client to Server and Server to Client.
pub trait Transport: Read + Write {}

impl<T> Transport for T where T: Read + Write {}
