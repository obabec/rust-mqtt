use heapless::Vec;

use crate::{
    client::options::{RetainHandling, SubscriptionOptions},
    eio::Write,
    io::{
        err::WriteError,
        write::{Writable, wlen},
    },
    types::MqttString,
};

/// A topic name string for that messages can be published on according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901241>.
/// Cannot contain wildcard characters.
///
/// Examples:
/// - "sport/tennis/player1"
/// - "sport/tennis/player1/ranking"
/// - "sport/tennis/player1/score/wimbledon"
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TopicName<'t>(MqttString<'t>);

impl<'t> TopicName<'t> {
    /// Creates a new topic name without checking for correct syntax of the topic name string.
    ///
    /// # Safety
    /// The syntax of the topic name is valid.
    pub unsafe fn new_unchecked(topic: MqttString<'t>) -> Self {
        Self(topic)
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub fn as_borrowed(&'t self) -> Self {
        Self(self.as_ref().as_borrowed())
    }
}

impl<'t> AsRef<MqttString<'t>> for TopicName<'t> {
    fn as_ref(&self) -> &MqttString<'t> {
        &self.0
    }
}
impl<'t> From<TopicName<'t>> for MqttString<'t> {
    fn from(value: TopicName<'t>) -> Self {
        value.0
    }
}

/// A topic filter string for subscribing to certain topics according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901241>.
/// Can contain wildcard characters.
///
/// Examples:
/// - "sport/tennis/#"
/// - "sport/+/player1"
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TopicFilter<'t>(MqttString<'t>);

impl<'t> TopicFilter<'t> {
    /// Creates a new topic filter without checking for correct syntax of the topic filter string.
    ///
    /// # Safety
    /// The syntax of the topic filter is valid.
    pub unsafe fn new_unchecked(topic: MqttString<'t>) -> Self {
        Self(topic)
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub fn as_borrowed(&'t self) -> Self {
        Self(self.as_ref().as_borrowed())
    }
}

impl<'t> AsRef<MqttString<'t>> for TopicFilter<'t> {
    fn as_ref(&self) -> &MqttString<'t> {
        &self.0
    }
}
impl<'t> From<TopicFilter<'t>> for MqttString<'t> {
    fn from(value: TopicFilter<'t>) -> Self {
        value.0
    }
}
impl<'t> From<TopicName<'t>> for TopicFilter<'t> {
    fn from(value: TopicName<'t>) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SubscriptionFilter<'t> {
    topic: TopicFilter<'t>,
    subscription_options: u8,
}

impl<'p, const MAX_TOPIC_FILTERS: usize> Writable
    for Vec<SubscriptionFilter<'p>, MAX_TOPIC_FILTERS>
{
    fn written_len(&self) -> usize {
        self.iter()
            .map(|t| &t.topic)
            .map(|t| t.as_ref().written_len() + wlen!(u8))
            .sum()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        for t in self {
            t.topic.as_ref().write(write).await?;
            t.subscription_options.write(write).await?;
        }

        Ok(())
    }
}

impl<'t> SubscriptionFilter<'t> {
    pub fn new(topic: TopicFilter<'t>, options: &SubscriptionOptions) -> Self {
        let retain_handling_bits = match options.retain_handling {
            RetainHandling::AlwaysSend => 0x00,
            RetainHandling::SendIfNotSubscribedBefore => 0x10,
            RetainHandling::NeverSend => 0x20,
        };

        let retain_as_published_bit = match options.retain_as_published {
            true => 0x08,
            false => 0x00,
        };

        let no_local_bit = match options.no_local {
            true => 0x04,
            false => 0x00,
        };

        let qos_bits = options.qos.into_bits(0);

        let subscribe_options_bits =
            retain_handling_bits | retain_as_published_bit | no_local_bit | qos_bits;

        Self {
            topic,
            subscription_options: subscribe_options_bits,
        }
    }
}
