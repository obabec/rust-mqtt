//! Contains types used throughout the MQTT specification.

mod binary;
mod int;
mod qos;
mod reason_code;
mod string;
mod topic;
mod will;

pub(crate) use will::Will;

pub use binary::MqttBinary;
pub use int::VarByteInt;
pub use qos::QoS;
pub use reason_code::ReasonCode;
pub use string::MqttString;
pub use topic::{Topic, TopicFilter};

/// Variable byte integer: If `VarByteInt::MAX_ENCODABLE` is exceeded, an error of this type is returned.
/// MQTT string: If `MqttString::MAX_LENGTH` is exceeded, an error of this type is returned.
/// MQTT binary: If `MqttBinary::MAX_LENGTH` is exceeded, an error of this type is returned.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TooLargeToEncode;
