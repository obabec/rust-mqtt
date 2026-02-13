//! Contains types used throughout the MQTT specification.

mod binary;
mod int;
mod qos;
mod reason_code;
mod string;
mod topic;
mod will;

pub(crate) use topic::SubscriptionFilter;
pub(crate) use will::Will;

pub use binary::MqttBinary;
pub use int::VarByteInt;
pub use qos::{IdentifiedQoS, QoS};
pub use reason_code::ReasonCode;
pub use string::{MqttString, MqttStringError};
pub use topic::{TopicFilter, TopicName};

/// The error returned when types are larger than what is representable according to the specification.
///
/// * [`VarByteInt`]: If [`VarByteInt::MAX_ENCODABLE`] is exceeded, an error of this type is returned.
/// * [`MqttBinary`]: If [`MqttBinary::MAX_LENGTH`] is exceeded, an error of this type is returned.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TooLargeToEncode;
