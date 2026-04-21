use const_fn::const_fn;
use heapless::Vec;

use crate::{
    client::options::{RetainHandling, SubscriptionOptions},
    eio::Write,
    fmt::const_debug_assert,
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
    const fn is_valid(s: &MqttString) -> bool {
        let s = s.as_str().as_bytes();

        // [MQTT-4.7.3-1]
        // Topic names must be at least one character long.
        if s.is_empty() {
            return false;
        }

        let mut i = 0;

        while i < s.len() {
            let b = s[i];

            // [MQTT-4.7.3-2]
            // Topic names must not include the null character.
            // No null characters are an invariant of `MqttString`

            // [MQTT-4.7.0-1]
            // Wildcard characters must not be used within a topic name.
            if b == b'+' || b == b'#' {
                return false;
            }

            i += 1;
        }

        true
    }

    /// Creates a new topic name while checking for correct syntax of the topic name string.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub fn new(string: MqttString<'t>) -> Option<Self> {
        if Self::is_valid(&string) {
            Some(Self(string))
        } else {
            None
        }
    }

    /// Creates a new topic name without checking for correct syntax of the topic name string.
    ///
    /// # Invariants
    /// The syntax of the topic name is valid. For a fallible version, use [`TopicName::new`]
    ///
    /// # Panics
    /// In debug builds, this function will panic if the syntax of `string` is incorrect.
    #[must_use]
    pub const fn new_unchecked(string: MqttString<'t>) -> Self {
        const_debug_assert!(
            Self::is_valid(&string),
            "the provided string is not valid TopicName syntax"
        );

        Self(string)
    }

    /// Delegates to [`Bytes::as_borrowed`].
    ///
    /// [`Bytes::as_borrowed`]: crate::Bytes::as_borrowed
    #[inline]
    #[must_use]
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
    const fn is_valid(s: &MqttString) -> bool {
        let s = s.as_str().as_bytes();

        // [MQTT-4.7.3-1]
        // Topic filters must be at least one character long.
        if s.is_empty() {
            return false;
        }

        let is_shared = s.len() >= 6
            && s[0] == b'$'
            && s[1] == b's'
            && s[2] == b'h'
            && s[3] == b'a'
            && s[4] == b'r'
            && s[5] == b'e';

        if is_shared {
            // Minimal shared filter "$share/a/a" has length 10
            if s.len() < 10 {
                return false;
            }
            // Check first '/' so that checking the sharename is simpler in the loop
            if s[6] != b'/' {
                return false;
            }
        }

        let mut i = match is_shared {
            true => 7,
            false => 0,
        };
        let mut level_len = 0;
        let mut level_contains_wildcard = false;

        let mut checking_share_name = is_shared;

        while i < s.len() {
            let b = s[i];

            // [MQTT-4.7.3-2]
            // Topic filters must not include the null character.
            // No null characters are an invariant of `MqttString`

            if b == b'#' {
                // [MQTT-4.7.1-1]
                // The multi-level wildcard character must be specified on its own.
                // The multi-level wildcard character must be the last character specified in the topic filter.
                if i == s.len() - 1 {
                    level_contains_wildcard = true;
                } else {
                    return false;
                }
            }

            if b == b'+' {
                level_contains_wildcard = true;
            }

            if b == b'/' {
                if checking_share_name {
                    // ... a ShareName that is at least one character long [MQTT-4.8.2-1].
                    if level_len == 0 {
                        return false;
                    }

                    // The ShareName MUST NOT contain the characters "/", "+" or "#", but MUST be
                    // followed by a "/" character. ...
                    // Topic Filter [MQTT-4.8.2-2]
                    if level_contains_wildcard {
                        return false;
                    }

                    // ... but MUST be followed by a "/" character. This "/" character MUST be
                    // followed by a Topic Filter [MQTT-4.8.2-2]
                    if i + 1 == s.len() {
                        return false;
                    }

                    checking_share_name = false;
                }

                level_len = 0;
                level_contains_wildcard = false;
            } else {
                level_len += 1;

                // [MQTT-4.7.1-2]
                // The single-level wildcard must occupy an entire level of the filter.
                // [MQTT-4.7.1-1]
                // The multi-level wildcard character must be specified on its own.
                if level_len > 1 && level_contains_wildcard {
                    return false;
                }
            }

            i += 1;
        }

        true
    }

    /// Returns whether the topic filter is the topic filter of a shared subscription.
    pub const fn is_shared(&self) -> bool {
        let s = self.0.as_str().as_bytes();

        s.len() >= 10
            && s[0] == b'$'
            && s[1] == b's'
            && s[2] == b'h'
            && s[3] == b'a'
            && s[4] == b'r'
            && s[5] == b'e'
    }

    /// Returns whether the topic filter contains one or more of the wildcard characters `#` or `+`.
    pub const fn has_wildcard(&self) -> bool {
        let s = self.0.as_str().as_bytes();

        let mut i = 0;
        while i < s.len() {
            let b = s[i];

            if b == b'#' || b == b'+' {
                return true;
            }

            i += 1;
        }

        false
    }

    /// Creates a new topic filter while checking for correct syntax of the topic filter string.
    /// If the filter starts with "$share", the constraints for a shared subscription's topic
    /// filter are also enforced.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub fn new(string: MqttString<'t>) -> Option<Self> {
        if Self::is_valid(&string) {
            Some(Self(string))
        } else {
            None
        }
    }

    /// Creates a new topic filter without checking for correct syntax of the topic filter string.
    ///
    /// # Invariants
    /// The syntax of the topic filter is valid. For a fallible version, use [`TopicFilter::new`].
    ///
    /// # Panics
    /// In debug builds, this function will panic if the syntax of `string` is incorrect.
    #[must_use]
    pub const fn new_unchecked(string: MqttString<'t>) -> Self {
        const_debug_assert!(
            Self::is_valid(&string),
            "the provided string is not valid TopicFilter syntax"
        );

        Self(string)
    }

    /// Delegates to [`Bytes::as_borrowed`].
    ///
    /// [`Bytes::as_borrowed`]: crate::Bytes::as_borrowed
    #[inline]
    #[must_use]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SubscriptionFilter<'t> {
    topic: TopicFilter<'t>,
    subscription_options: u8,
}

impl<const MAX_TOPIC_FILTERS: usize> Writable for Vec<SubscriptionFilter<'_>, MAX_TOPIC_FILTERS> {
    fn written_len(&self) -> usize {
        self.iter()
            .map(|t| &t.topic)
            .map(|t| t.written_len() + wlen!(u8))
            .sum()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        for t in self {
            t.topic.write(write).await?;
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
    use tokio_test::assert_ok;

    use crate::types::{MqttString, TopicFilter, TopicName};

    macro_rules! assert_valid {
        ($t:ty, $l:literal) => {{
            let s = assert_ok!(MqttString::from_str($l), "{}", $l);
            let t = <$t>::new(s);
            assert!(t.is_some(), "{}", $l);
            let t = t.unwrap();

            t
        }};
    }
    macro_rules! assert_valid_shared {
        ($t:ty, $l:literal) => {{
            let s = assert_ok!(MqttString::from_str($l), "{}", $l);
            let t = <$t>::new(s);
            assert!(t.is_some(), "{}", $l);
            let t = t.unwrap();
            assert!(t.is_shared(), "{}", $l);

            t
        }};
    }
    macro_rules! assert_valid_non_shared {
        ($t:ty, $l:literal) => {{
            let s = assert_ok!(MqttString::from_str($l), "{}", $l);
            let t = <$t>::new(s);
            assert!(t.is_some(), "{}", $l);
            let t = t.unwrap();
            assert!(!t.is_shared(), "{}", $l);

            t
        }};
    }
    macro_rules! assert_invalid {
        ($t:ty, $l:literal) => {
            match MqttString::from_str($l) {
                Ok(s) => assert!(<$t>::new(s).is_none(), "{}", $l),
                Err(_) => {}
            }
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
        assert_valid_non_shared!(TopicFilter, "#");
        assert_valid_non_shared!(TopicFilter, "/#");
        assert_valid_non_shared!(TopicFilter, "a/#");

        assert_valid_non_shared!(TopicFilter, "+");
        assert_valid_non_shared!(TopicFilter, "/+");
        assert_valid_non_shared!(TopicFilter, "+/");
        assert_valid_non_shared!(TopicFilter, "a/+");
        assert_valid_non_shared!(TopicFilter, "+/a");

        assert_valid_non_shared!(TopicFilter, "/");
        assert_valid_non_shared!(TopicFilter, "//");
        assert_valid_non_shared!(TopicFilter, "r");

        assert_valid_non_shared!(TopicFilter, "fshare/");
        assert_valid_non_shared!(TopicFilter, "$fhare/");
        assert_valid_non_shared!(TopicFilter, "$sfare/");
        assert_valid_non_shared!(TopicFilter, "$shfre/");
        assert_valid_non_shared!(TopicFilter, "$shafe/");
        assert_valid_non_shared!(TopicFilter, "$sharf/");
        assert_valid_non_shared!(TopicFilter, "share/");
        assert_valid_non_shared!(TopicFilter, "$hare/");
        assert_valid_non_shared!(TopicFilter, "$sare/");
        assert_valid_non_shared!(TopicFilter, "$shre/");
        assert_valid_non_shared!(TopicFilter, "$shae/");

        assert_valid_non_shared!(TopicFilter, "r/i/g/+/t");
        assert_valid_non_shared!(TopicFilter, "correct/+/path");
        assert_valid_non_shared!(TopicFilter, "right/path/#");
        assert_valid_non_shared!(TopicFilter, "right");
        assert_valid_non_shared!(TopicFilter, "sport/tennis/player1");
        assert_valid_non_shared!(TopicFilter, "sport/tennis/player1/ranking");
        assert_valid_non_shared!(TopicFilter, "sport/tennis/player1/score/wimbledon");
    }

    #[test]
    fn topic_filter_shared_no_share_name() {
        assert_invalid!(TopicFilter, "$share");
        assert_invalid!(TopicFilter, "$share/");
        assert_invalid!(TopicFilter, "$share//");
    }

    #[test]
    fn topic_filter_shared_invalid_share_name() {
        assert_invalid!(TopicFilter, "$share/+/");
        assert_invalid!(TopicFilter, "$share/#/a");
        assert_invalid!(TopicFilter, "$share/a+a/a");
        assert_invalid!(TopicFilter, "$share/a+/a");
        assert_invalid!(TopicFilter, "$share/+a/a");
        assert_invalid!(TopicFilter, "$share/a#a/a");
        assert_invalid!(TopicFilter, "$share/a#/a");
        assert_invalid!(TopicFilter, "$share/#a/a");
        assert_invalid!(TopicFilter, "$share///a");
    }

    #[test]
    fn topic_filter_shared_no_topic_filter() {
        assert_invalid!(TopicFilter, "$share/a/");
    }

    #[test]
    fn topic_filter_shared_valid() {
        assert_valid_shared!(TopicFilter, "$share/a/a");
        assert_valid_shared!(TopicFilter, "$share/a/+");
        assert_valid_shared!(TopicFilter, "$share/a/#");
        assert_valid_shared!(TopicFilter, "$share/a//");
        assert_valid_shared!(TopicFilter, "$share/consumer1/sport/tennis/+");
        assert_valid_shared!(TopicFilter, "$share/consumer2/sport/tennis/+");
        assert_valid_shared!(TopicFilter, "$share/consumer1//finance");
        assert_valid_shared!(TopicFilter, "$share/consumer1/+/finance");
    }

    #[test]
    fn topic_filter_has_wildcard() {
        assert!(!assert_valid!(TopicFilter, "a").has_wildcard());
        assert!(!assert_valid!(TopicFilter, "/").has_wildcard());
        assert!(!assert_valid!(TopicFilter, "//").has_wildcard());
        assert!(!assert_valid!(TopicFilter, "/a/").has_wildcard());
        assert!(!assert_valid!(TopicFilter, "$").has_wildcard());
        assert!(!assert_valid!(TopicFilter, "$a/$/").has_wildcard());
        assert!(!assert_valid!(TopicFilter, "$share/consumer1//finance").has_wildcard());

        assert!(assert_valid!(TopicFilter, "#").has_wildcard());
        assert!(assert_valid!(TopicFilter, "+").has_wildcard());
        assert!(assert_valid!(TopicFilter, "/+/").has_wildcard());
        assert!(assert_valid!(TopicFilter, "+/").has_wildcard());
        assert!(assert_valid!(TopicFilter, "/#").has_wildcard());
        assert!(assert_valid!(TopicFilter, "/+").has_wildcard());
        assert!(assert_valid!(TopicFilter, "/+").has_wildcard());
        assert!(assert_valid!(TopicFilter, "$share/consumer1/+/finance").has_wildcard());
        assert!(assert_valid!(TopicFilter, "$share/consumer1/#").has_wildcard());
    }
}
