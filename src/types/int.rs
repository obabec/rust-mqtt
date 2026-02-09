use crate::types::TooLargeToEncode;

/// MQTT's variable byte integer encoding. The value has to be less than `VarByteInt::MAX_ENCODABLE`
/// (268_435_455). Exceeding this ultimately leads to panics or malformed packets.
///
/// Used for packet length and some properties.
///
/// Use its `TryFrom<u32>`, `From<u16>` and `From<u8>` implementations to construct a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct VarByteInt(u32);

impl VarByteInt {
    /// The maximum encodable value using the variable byte integer encoding according to
    /// <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>.
    pub const MAX_ENCODABLE: u32 = 268_435_455;

    /// Creates a variable byte integer by checking for the maximum value of [`VarByteInt::MAX_ENCODABLE`].
    /// For a version accepting `u16` and `u8`, use `From::from`.
    pub const fn new(value: u32) -> Option<Self> {
        if value > Self::MAX_ENCODABLE {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Creates a variable byte integer without checking for the maximum value of
    /// `VarByteInt::MAX_ENCODABLE`.
    /// For a fallible version, use `VarByteInt::new`.
    /// For an infallible version accepting `u16` and `u8`, use `From::from`.
    ///
    /// # Invariants
    /// The value parameter must be less than or equal to [`VarByteInt::MAX_ENCODABLE`].
    ///
    /// # Panics
    /// Panics in debug builds if `value` exceeds [`VarByteInt::MAX_ENCODABLE`]
    pub const fn new_unchecked(value: u32) -> Self {
        debug_assert!(
            value <= Self::MAX_ENCODABLE,
            "the value exceeds MAX_ENCODABLE"
        );

        Self(value)
    }

    /// Returns the inner value.
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns `Self::value() as usize`
    pub const fn size(&self) -> usize {
        self.0 as usize
    }

    /// Decodes a variable byte integer from a slice.
    ///
    /// # Invariants
    /// The slice must contain a correctly encoded variable byte integer and
    /// have exactly the length of that encoding.
    pub(crate) fn from_slice_unchecked(slice: &[u8]) -> Self {
        let mut multiplier = 1;
        let mut value = 0;

        debug_assert!(
            !slice.is_empty() && slice.len() <= 4,
            "encodings are always 1..=4 bytes long, {} is invalid",
            slice.len()
        );

        debug_assert_eq!(
            slice.last().unwrap() & 128,
            0,
            "the last byte of the encoding must not have bit 7 set"
        );

        for b in slice {
            value += (b & 0x7F) as u32 * multiplier;
            multiplier *= 128;
            if b & 128 == 0 {
                break;
            }
        }

        Self::new_unchecked(value)
    }
}

impl TryFrom<u32> for VarByteInt {
    type Error = TooLargeToEncode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(TooLargeToEncode)
    }
}
impl From<u16> for VarByteInt {
    fn from(value: u16) -> Self {
        Self(value as u32)
    }
}
impl From<u8> for VarByteInt {
    fn from(value: u8) -> Self {
        Self(value as u32)
    }
}
