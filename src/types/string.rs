use core::{
    fmt,
    str::{Utf8Error, from_utf8, from_utf8_unchecked},
};

use crate::{
    Bytes,
    types::{MqttBinary, TooLargeToEncode},
};

/// Arbitrary UTF-8 encoded string with a length in bytes less than or equal to `Self::MAX_LENGTH`
#[derive(Default, Clone, PartialEq, Eq)]
pub struct MqttString<'s>(pub(crate) MqttBinary<'s>);

impl<'s> fmt::Debug for MqttString<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MqttString").field(&self.as_ref()).finish()
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for MqttString<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "MqttString({:?}", self.as_ref());
    }
}

impl<'s> TryFrom<MqttBinary<'s>> for MqttString<'s> {
    type Error = Utf8Error;

    fn try_from(value: MqttBinary<'s>) -> Result<Self, Self::Error> {
        from_utf8(value.0.as_ref())?;
        Ok(Self(value))
    }
}
impl<'s> TryFrom<&'s str> for MqttString<'s> {
    type Error = TooLargeToEncode;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        let binary = MqttBinary::try_from(value.as_bytes())?;

        Ok(Self(binary))
    }
}

impl<'s> AsRef<str> for MqttString<'s> {
    fn as_ref(&self) -> &str {
        // Safety: MqttString contains valid UTF-8
        unsafe { from_utf8_unchecked(self.0.as_ref()) }
    }
}

impl<'s> MqttString<'s> {
    /// The maximum length of a string in bytes so that it can be encoded. This value is limited by the 2-byte length field.
    pub const MAX_LENGTH: usize = MqttBinary::MAX_LENGTH;

    /// Creates an MQTT string and checks for the max length in bytes of `Self::MAX_LENGTH`.
    ///
    /// # Important
    /// Does not check that the data is valid UTF-8!
    pub fn new(bytes: Bytes<'s>) -> Result<Self, TooLargeToEncode> {
        Ok(Self(MqttBinary::new(bytes)?))
    }

    /// Creates an MQTT string and checks for the max length in bytes of `Self::MAX_LENGTH`.
    pub const fn from_slice(slice: &'s str) -> Result<Self, TooLargeToEncode> {
        match slice.len() {
            // Safety: The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
            ..=Self::MAX_LENGTH => Ok(Self(unsafe {
                MqttBinary::from_slice_unchecked(slice.as_bytes())
            })),
            _ => Err(TooLargeToEncode),
        }
    }

    /// Creates an MQTT string without checking for the max length in bytes of `Self::MAX_LENGTH`.
    ///
    /// # Safety
    /// The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
    pub unsafe fn new_unchecked(bytes: Bytes<'s>) -> Self {
        // Safety: The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
        Self(unsafe { MqttBinary::new_unchecked(bytes) })
    }

    /// Creates an MQTT string without checking for the max length in bytes of `Self::MAX_LENGTH`.
    ///
    /// # Safety
    /// The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
    pub const unsafe fn from_slice_unchecked(slice: &'s str) -> Self {
        // Safety: The length of the slice parameter in bytes is less than or equal to `Self::MAX_LENGTH`.
        Self(unsafe { MqttBinary::from_slice_unchecked(slice.as_bytes()) })
    }

    /// Returns the length of the underlying data in bytes.
    #[inline]
    pub fn len(&self) -> u16 {
        self.0.len()
    }

    /// Returns whether the underlying data is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub fn as_borrowed(&'s self) -> Self {
        Self(self.0.as_borrowed())
    }
}
