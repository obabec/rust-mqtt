use const_fn::const_fn;
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
    /// Creates a new topic name while checking for correct syntax of the topic name string.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub fn new(string: MqttString<'t>) -> Option<Self> {
        let s = string.as_str().as_bytes();

        // [MQTT-4.7.3-1]
        // Topic names must be at least one character long.
        if s.is_empty() {
            return None;
        }

        let mut i = 0;

        while i < s.len() {
            let b = s[i];

            // [MQTT-4.7.3-2]
            // Topic names must not include the null character.
            if b == b'\0' {
                return None;
            }

            // [MQTT-4.7.0-1]
            // Wildcard characters must not be used within a topic name.
            if b == b'+' || b == b'#' {
                return None;
            }

            i += 1;
        }

        Some(Self(string))
    }

    /// Creates a new topic name without checking for correct syntax of the topic name string.
    ///
    /// # Safety
    /// The syntax of the topic name is valid.
    pub const unsafe fn new_unchecked(topic: MqttString<'t>) -> Self {
        Self(topic)
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub const fn as_borrowed(&'t self) -> Self {
        Self(self.0.as_borrowed())
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
    /// Creates a new topic filter while checking for correct syntax of the topic filter string
    #[const_fn(cfg(not(feature = "alloc")))]
    pub fn new(string: MqttString<'t>) -> Option<Self> {
        let s = string.as_str().as_bytes();

        // [MQTT-4.7.3-1]
        // Topic filters must be at least one character long.
        if s.is_empty() {
            return None;
        }

        let mut i = 0;
        let mut level_len = 0;
        let mut level_contains_wildcard = false;

        while i < s.len() {
            let b = s[i];

            // [MQTT-4.7.3-2]
            // Topic filters must not include the null character.
            if b == b'\0' {
                return None;
            }

            if b == b'#' {
                // [MQTT-4.7.1-1]
                // The multi-level wildcard character must be specified on its own.
                // The multi-level wildcard character must be the last character specified in the topic filter.
                if i == s.len() - 1 {
                    level_contains_wildcard = true;
                } else {
                    return None;
                }
            }

            if b == b'+' {
                level_contains_wildcard = true;
            }

            if b == b'/' {
                level_len = 0;
                level_contains_wildcard = false;
            } else {
                level_len += 1;

                // [MQTT-4.7.1-2]
                // The single-level wildcard must occupy an entire level of the filter.
                // [MQTT-4.7.1-1]
                // The multi-level wildcard character must be specified on its own.
                if level_len > 1 && level_contains_wildcard {
                    return None;
                }
            }

            i += 1;
        }

        Some(Self(string))
    }

    /// Creates a new topic filter without checking for correct syntax of the topic filter string.
    ///
    /// # Safety
    /// The syntax of the topic filter is valid.
    pub const unsafe fn new_unchecked(topic: MqttString<'t>) -> Self {
        Self(topic)
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub const fn as_borrowed(&'t self) -> Self {
        Self(self.0.as_borrowed())
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
    pub const fn new(topic: TopicFilter<'t>, options: &SubscriptionOptions) -> Self {
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

#[cfg(test)]
mod unit {
    use crate::types::{MqttString, TopicFilter, TopicName};
    use tokio_test::assert_ok;

    macro_rules! assert_valid {
        ($t:ty, $l:literal) => {
            let s = assert_ok!(MqttString::from_slice($l));
            assert!(<$t>::new(s).is_some())
        };
    }
    macro_rules! assert_invalid {
        ($t:ty, $l:literal) => {
            let s = assert_ok!(MqttString::from_slice($l));
            assert!(<$t>::new(s).is_none())
        };
    }

    #[test]
    fn topic_name_zero_characters() {
        assert_invalid!(TopicName, "");
    }

    #[test]
    fn topic_name_null_character() {
        assert_invalid!(TopicName, "he\0/yo");
    }

    #[test]
    fn topic_name_with_wildcard() {
        assert_invalid!(TopicName, "+wrong");
        assert_invalid!(TopicName, "wro#ng");
        assert_invalid!(TopicName, "w/r/o/n/g+");
        assert_invalid!(TopicName, "w/r/o/+/g");
        assert_invalid!(TopicName, "wrong/#/path");
        assert_invalid!(TopicName, "wrong/+/path");
        assert_invalid!(TopicName, "wrong/path/#");
        assert_invalid!(TopicName, "#");
        assert_invalid!(TopicName, "+");
    }

    #[test]
    fn topic_name_valid() {
        assert_valid!(TopicName, "/");
        assert_valid!(TopicName, "r");
        assert_valid!(TopicName, "right");
        assert_valid!(TopicName, "sport/tennis/player1");
        assert_valid!(TopicName, "sport/tennis/player1/ranking");
        assert_valid!(TopicName, "sport/tennis/player1/score/wimbledon");
    }

    #[test]
    fn topic_filter_zero_characters() {
        assert_invalid!(TopicFilter, "");
    }

    #[test]
    fn topic_filter_null_character() {
        assert_invalid!(TopicFilter, "he\0/yo");
    }

    #[test]
    fn topic_filter_with_invalid_wildcard() {
        assert_invalid!(TopicFilter, "++/");
        assert_invalid!(TopicFilter, "/++");

        assert_invalid!(TopicFilter, "a+/");
        assert_invalid!(TopicFilter, "+a/");
        assert_invalid!(TopicFilter, "/a+/");
        assert_invalid!(TopicFilter, "/+a/");
        assert_invalid!(TopicFilter, "/a+");

        assert_invalid!(TopicFilter, "##");
        assert_invalid!(TopicFilter, "a#");
        assert_invalid!(TopicFilter, "#a");
        
        assert_invalid!(TopicFilter, "a#/");
        assert_invalid!(TopicFilter, "#a/");
        assert_invalid!(TopicFilter, "/a#/");
        assert_invalid!(TopicFilter, "/#a/");
        assert_invalid!(TopicFilter, "/a#");
        assert_invalid!(TopicFilter, "/#a");

        assert_invalid!(TopicFilter, "+wrong");
        assert_invalid!(TopicFilter, "wro#ng");
        assert_invalid!(TopicFilter, "w/r/o/n/g+");
        assert_invalid!(TopicFilter, "wrong/#/path");
    }

    #[test]
    fn topic_filter_valid() {
        assert_valid!(TopicFilter, "#");
        assert_valid!(TopicFilter, "/#");
        assert_valid!(TopicFilter, "a/#");

        assert_valid!(TopicFilter, "+");
        assert_valid!(TopicFilter, "/+");
        assert_valid!(TopicFilter, "+/");
        assert_valid!(TopicFilter, "a/+");
        assert_valid!(TopicFilter, "+/a");

        assert_valid!(TopicFilter, "/");
        assert_valid!(TopicFilter, "//");
        assert_valid!(TopicFilter, "r");

        assert_valid!(TopicFilter, "r/i/g/+/t");
        assert_valid!(TopicFilter, "correct/+/path");
        assert_valid!(TopicFilter, "right/path/#");
        assert_valid!(TopicFilter, "right");
        assert_valid!(TopicFilter, "sport/tennis/player1");
        assert_valid!(TopicFilter, "sport/tennis/player1/ranking");
        assert_valid!(TopicFilter, "sport/tennis/player1/score/wimbledon");
    }
}
