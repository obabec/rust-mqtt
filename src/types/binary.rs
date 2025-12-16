use core::fmt;

use crate::{
    bytes::Bytes,
    types::{MqttString, TooLargeToEncode},
};

/// Arbitrary binary data with a length less than or equal to `Self::MAX_LENGTH`
#[derive(Default, Clone, PartialEq, Eq)]
pub struct MqttBinary<'b>(pub(crate) Bytes<'b>);

impl<'b> fmt::Debug for MqttBinary<'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MqttBinary").field(&self.as_ref()).finish()
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for MqttBinary<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "MqttBinary({:?}", self.as_ref());
    }
}

impl<'b> TryFrom<&'b [u8]> for MqttBinary<'b> {
    type Error = TooLargeToEncode;

    fn try_from(value: &'b [u8]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}
impl<'b> TryFrom<&'b str> for MqttBinary<'b> {
    type Error = TooLargeToEncode;

    fn try_from(value: &'b str) -> Result<Self, Self::Error> {
        Self::from_slice(value.as_bytes())
    }
}
impl<'b> From<MqttString<'b>> for MqttBinary<'b> {
    fn from(value: MqttString<'b>) -> Self {
        Self(value.0.0)
    }
}

impl<'b> AsRef<[u8]> for MqttBinary<'b> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<'b> MqttBinary<'b> {
    /// The maximum length of binary data so that it can be encoded. This value is limited by the 2-byte length field.
    pub const MAX_LENGTH: usize = u16::MAX as usize;

    /// Creates MQTT binary data and checks for the max length in bytes of `Self::MAX_LENGTH`.
    pub fn new(bytes: Bytes<'b>) -> Result<Self, TooLargeToEncode> {
        match bytes.len() {
            ..=Self::MAX_LENGTH => Ok(Self(bytes)),
            _ => Err(TooLargeToEncode),
        }
    }

    /// Creates MQTT binary data and checks for the max length in bytes of `Self::MAX_LENGTH`.
    pub const fn from_slice(slice: &'b [u8]) -> Result<Self, TooLargeToEncode> {
        match slice.len() {
            ..=Self::MAX_LENGTH => Ok(Self(Bytes::Borrowed(slice))),
            _ => Err(TooLargeToEncode),
        }
    }

    /// Creates MQTT binary data without checking for the max length in bytes of `Self::MAX_LENGTH`.
    ///
    /// # Safety
    /// The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
    pub const unsafe fn new_unchecked(bytes: Bytes<'b>) -> Self {
        Self(bytes)
    }

    /// Creates MQTT binary data without checking for the max length in bytes of `Self::MAX_LENGTH`.
    ///
    /// # Safety
    /// The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
    pub const unsafe fn from_slice_unchecked(slice: &'b [u8]) -> Self {
        Self(Bytes::Borrowed(slice))
    }

    /// Returns the length of the underlying data.
    #[inline]
    pub const fn len(&self) -> u16 {
        self.0.len() as u16
    }

    /// Returns whether the underlying data is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub fn as_borrowed(&'b self) -> Self {
        Self(self.0.as_borrowed())
    }
}
