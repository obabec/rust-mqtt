use core::fmt;

use const_fn::const_fn;

use crate::{
    bytes::Bytes,
    types::{MqttString, TooLargeToEncode},
};

/// Arbitrary binary data with a length less than or equal to `MqttBinary::MAX_LENGTH`.
///
/// # Examples
///
/// ```rust
/// use rust_mqtt::Bytes;
/// use rust_mqtt::types::{MqttBinary, MqttString, TooLargeToEncode};
///
/// let slice = [0x00; MqttBinary::MAX_LENGTH];
/// let too_long = [0x00; MqttBinary::MAX_LENGTH + 1];
///
/// let b = MqttBinary::from_slice(&slice)?;
/// assert_eq!(b.as_bytes(), &slice);
/// assert!(MqttBinary::from_slice(&too_long).is_err());
///
/// let b = MqttBinary::from_bytes(Bytes::Borrowed(&slice))?;
/// assert_eq!(b.as_bytes(), &slice);
/// assert!(MqttBinary::from_bytes(Bytes::Borrowed(&too_long)).is_err());
///
/// let from_slice_unchecked = MqttBinary::from_slice_unchecked(&slice);
/// assert_eq!(from_slice_unchecked.as_bytes(), &slice);
/// 
/// let from_bytes_unchecked = MqttBinary::from_bytes_unchecked(Bytes::Borrowed(&slice));
/// assert_eq!(from_bytes_unchecked.as_bytes(), &slice);
/// 
/// # Ok::<(), TooLargeToEncode>(())
/// ```
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
        defmt::write!(fmt, "MqttBinary({:?})", self.as_ref());
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
        self.as_bytes()
    }
}

impl<'b> MqttBinary<'b> {
    /// The maximum length of binary data so that it can be encoded. This value is limited by the 2-byte length field.
    pub const MAX_LENGTH: usize = u16::MAX as usize;

    /// Converts `Bytes` into `MqttBinary` by checking for the max length of `MqttBinary::MAX_LENGTH`.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn from_bytes(bytes: Bytes<'b>) -> Result<Self, TooLargeToEncode> {
        match bytes.len() {
            ..=Self::MAX_LENGTH => Ok(Self(bytes)),
            _ => Err(TooLargeToEncode),
        }
    }

    /// Converts a slice into `MqttBinary` by cloning the reference and checking for the max length of `MqttBinary::MAX_LENGTH`.
    pub const fn from_slice(slice: &'b [u8]) -> Result<Self, TooLargeToEncode> {
        match slice.len() {
            ..=Self::MAX_LENGTH => Ok(Self(Bytes::Borrowed(slice))),
            _ => Err(TooLargeToEncode),
        }
    }

    /// Converts `Bytes` into `MqttBinary` without checking for the max length of `MqttBinary::MAX_LENGTH`.
    ///
    /// # Invariants
    ///
    /// The length of the slice parameter in bytes is less than or equal to `MqttBinary::MAX_LENGTH`.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if the bytes' length is greater than `MqttBinary::MAX_LENGTH`.
    pub const fn from_bytes_unchecked(bytes: Bytes<'b>) -> Self {
        debug_assert!(
            bytes.len() <= Self::MAX_LENGTH,
            "the slice's length exceeds MAX_LENGTH"
        );

        Self(bytes)
    }

    /// Converts a slice into `MqttBinary` without checking for the max length of `MqttBinary::MAX_LENGTH`.
    ///
    /// # Invariants
    ///
    /// The length of the slice parameter in bytes is less than or equal to `MqttBinary::MAX_LENGTH`.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if the slice's length is greater than `MqttBinary::MAX_LENGTH`.
    pub const fn from_slice_unchecked(slice: &'b [u8]) -> Self {
        debug_assert!(
            slice.len() <= Self::MAX_LENGTH,
            "the slice's length exceeds MAX_LENGTH"
        );

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

    /// Returns the underlying bytes as `&[u8]`
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub const fn as_borrowed(&'b self) -> Self {
        Self(self.0.as_borrowed())
    }
}
