use core::{
    fmt,
    str::{Utf8Error, from_utf8, from_utf8_unchecked},
};

use const_fn::const_fn;

use crate::types::{MqttBinary, TooLargeToEncode};

/// Error returned when creating `MqttString` failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MqttStringError {
    /// The passed data is not valid UTF-8.
    Utf8Error(Utf8Error),

    /// The passed data contains at least one null character.
    NullCharacter,

    /// The passed data exceeds the max length of `MqttString::MAX_LENGTH`.
    TooLargeToEncode,
}

impl From<Utf8Error> for MqttStringError {
    fn from(e: Utf8Error) -> Self {
        Self::Utf8Error(e)
    }
}
impl From<TooLargeToEncode> for MqttStringError {
    fn from(_: TooLargeToEncode) -> Self {
        Self::TooLargeToEncode
    }
}

/// Arbitrary UTF-8 encoded string with a length in bytes less than or equal to `MqttString::MAX_LENGTH`
/// (`u16::MAX`) and no null characters.
/// Exceeding this size ultimately leads to malformed packets.
///
/// # Examples
///
/// ```rust
/// use rust_mqtt::types::{MqttBinary, MqttString, MqttStringError};
///
/// let bytes = [b'a'; MqttString::MAX_LENGTH];
/// let too_long = [b'a'; MqttString::MAX_LENGTH + 1];
/// let null_character = "hi\0there";
///
/// let slice = core::str::from_utf8(&bytes)?;
/// let too_long = core::str::from_utf8(&too_long)?;
///
/// let b = MqttBinary::from_slice(&bytes)?;
/// let s = MqttString::from_utf8_binary(b)?;
/// assert_eq!(s.as_str(), slice);
/// let b = MqttBinary::from_slice(null_character.as_bytes())?;
/// assert_eq!(MqttString::from_utf8_binary(b).unwrap_err(), MqttStringError::NullCharacter);
///
/// let s = MqttString::from_str(slice)?;
/// assert_eq!(s.as_str(), slice);
/// assert_eq!(MqttString::from_str(too_long).unwrap_err(), MqttStringError::TooLargeToEncode);
/// assert_eq!(MqttString::from_str(&null_character).unwrap_err(), MqttStringError::NullCharacter);
///
/// let s = MqttString::from_str_unchecked(slice);
/// assert_eq!(s.as_str(), slice);
///
/// let b = MqttBinary::from_slice_unchecked(slice.as_bytes());
/// let s = unsafe { MqttString::from_utf8_binary_unchecked(b) };
/// assert_eq!(s.as_str(), slice);
/// 
/// # Ok::<(), MqttStringError>(())
/// ```
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
        defmt::write!(fmt, "MqttString({:?})", self.as_ref());
    }
}

impl<'s> TryFrom<MqttBinary<'s>> for MqttString<'s> {
    type Error = MqttStringError;

    fn try_from(value: MqttBinary<'s>) -> Result<Self, Self::Error> {
        Self::from_utf8_binary(value)
    }
}
impl<'s> TryFrom<&'s str> for MqttString<'s> {
    type Error = MqttStringError;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        Self::from_str(value)
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

    /// Converts `MqttBinary` into `MqttString` by checking for null characters and valid UTF-8.
    /// Valid length is guaranteed by `MqttBinary`'s invariant.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn from_utf8_binary(b: MqttBinary<'s>) -> Result<Self, MqttStringError> {
        for b in b.as_bytes() {
            if *b == 0 {
                return Err(MqttStringError::NullCharacter);
            }
        }

        match from_utf8(b.as_bytes()) {
            Ok(_) => Ok(Self(b)),
            Err(e) => Err(MqttStringError::Utf8Error(e)),
        }
    }

    /// Converts `MqttBinary` into `MqttString` without checking for null characters or valid UTF-8.
    /// Valid length is guaranteed by `MqttBinary`'s invariant.
    ///
    /// # Safety
    ///
    /// The binary passed in must be valid UTF-8.
    ///
    /// # Invariants
    ///
    /// The binary data does not contain any null characters.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if the binary contains a null character or is not
    /// valid UTF-8.
    pub const unsafe fn from_utf8_binary_unchecked(b: MqttBinary<'s>) -> Self {
        if cfg!(debug_assertions) {
            let mut i = 0;
            while i < b.as_bytes().len() {
                debug_assert!(b.as_bytes()[i] != 0);
                i += 1;
            }
        }
        debug_assert!(from_utf8(b.as_bytes()).is_ok());

        Self(b)
    }

    /// Converts a string slice into `MqttString` by checking for null characters and the max length
    /// of `MqttString::MAX_LENGTH`.
    pub const fn from_str(s: &'s str) -> Result<Self, MqttStringError> {
        let mut i = 0;
        while i < s.len() {
            if s.as_bytes()[i] == 0 {
                return Err(MqttStringError::NullCharacter);
            }
            i += 1;
        }

        match s.len() {
            ..=Self::MAX_LENGTH => Ok(Self(MqttBinary::from_slice_unchecked(s.as_bytes()))),
            _ => Err(MqttStringError::TooLargeToEncode),
        }
    }

    /// Converts a string slice into `MqttString` without checking for null characters or the max
    /// length of `MqttString::MAX_LENGTH`.
    ///
    /// # Invariants
    ///
    /// The length of the string slice must be less than or equal to `MqttString::MAX_LENGTH`. The
    /// string must not contain any null characters.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if the slice contains a null character or its length is greater
    /// than `MqttString::MAX_LENGTH`.
    pub const fn from_str_unchecked(s: &'s str) -> Self {
        if cfg!(debug_assertions) {
            let mut i = 0;
            while i < s.len() {
                debug_assert!(s.as_bytes()[i] != 0);
                i += 1;
            }
        }

        Self(MqttBinary::from_slice_unchecked(s.as_bytes()))
    }

    /// Returns the length of the underlying data in bytes.
    #[inline]
    pub const fn len(&self) -> u16 {
        self.0.len()
    }

    /// Returns whether the underlying data is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the underlying string as `&str`
    #[inline]
    pub const fn as_str(&self) -> &str {
        // Safety: MqttString contains valid UTF-8
        unsafe { from_utf8_unchecked(self.0.as_bytes()) }
    }

    /// Delegates to `Bytes::as_borrowed()`.
    #[inline]
    pub const fn as_borrowed(&'s self) -> Self {
        Self(self.0.as_borrowed())
    }
}
