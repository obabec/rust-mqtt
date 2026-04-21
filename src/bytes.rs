#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use core::{borrow::Borrow, ops::Deref};

/// Contiguous bytes in memory. Is either a [`u8`] slice or (with crate feature "alloc") an owned
/// [`Box`]<[u8]>.
///
/// It is recommended to almost always use owned [`Bytes`] instead of a reference to [`Bytes`],
/// as it makes this type compatible for code designed for both owned and borrowed variants.
///
/// Important: Cloning this will clone the underlying Box if it is an owned variant.
/// You can however borrow another owned [`Bytes`] by calling [`Bytes::as_borrowed`].
/// The [`Bytes::as_borrowed`] method is passed on through wrapper types, for example
/// [`MqttString`].
///
/// [`MqttString`]: crate::types::MqttString
pub enum Bytes<'a> {
    /// Owned variant, only available with the `alloc` feature enabled.
    #[cfg(feature = "alloc")]
    Owned(Box<[u8]>),

    /// Borrowed variant.
    Borrowed(&'a [u8]),
}

impl<'a> Bytes<'a> {
    /// Returns the underlying data as `&[u8]`.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => b,
            Self::Borrowed(s) => s,
        }
    }

    /// Borrows `self` with its full lifetime to create another owned [`Self`] instance.
    #[inline]
    #[must_use]
    pub const fn as_borrowed(&'a self) -> Self {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => Self::Borrowed(b),
            Self::Borrowed(s) => Self::Borrowed(s),
        }
    }

    /// Returns the number of bytes.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => b.len(),
            Self::Borrowed(s) => s.len(),
        }
    }

    /// Returns whether the underlying data has a length of 0.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Clone for Bytes<'_> {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => Self::Owned(b.clone()),
            Self::Borrowed(s) => Self::Borrowed(s),
        }
    }
}

#[cfg(feature = "alloc")]
impl From<Bytes<'_>> for Box<[u8]> {
    fn from(bytes: Bytes<'_>) -> Self {
        match bytes {
            Bytes::Owned(b) => b,
            Bytes::Borrowed(s) => s.into(),
        }
    }
}

impl Default for Bytes<'_> {
    fn default() -> Self {
        Self::Borrowed(&[])
    }
}

impl core::fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Borrowed(x) => f.debug_tuple("Borrowed").field(x).finish(),
            #[cfg(feature = "alloc")]
            Self::Owned(x) => f.debug_tuple("Owned").field(x).finish(),
        }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Bytes<'_> {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Self::Borrowed(x) => defmt::write!(fmt, "Borrowed({:?})", *x),
            #[cfg(feature = "alloc")]
            Self::Owned(x) => defmt::write!(fmt, "Owned({:?})", x.as_ref()),
        }
    }
}

impl PartialEq for Bytes<'_> {
    fn eq(&self, other: &Self) -> bool {
        let one: &[u8] = self;
        let other: &[u8] = other;

        one == other
    }
}
impl Eq for Bytes<'_> {}

impl<'a> From<&'a mut [u8]> for Bytes<'a> {
    fn from(value: &'a mut [u8]) -> Self {
        Self::Borrowed(value)
    }
}
impl<'a> From<&'a [u8]> for Bytes<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::Borrowed(value)
    }
}
impl<'a> From<&'a mut str> for Bytes<'a> {
    fn from(value: &'a mut str) -> Self {
        Self::Borrowed(value.as_bytes())
    }
}
impl<'a> From<&'a str> for Bytes<'a> {
    fn from(value: &'a str) -> Self {
        Self::Borrowed(value.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl From<Box<[u8]>> for Bytes<'_> {
    fn from(value: Box<[u8]>) -> Self {
        Self::Owned(value)
    }
}

impl Deref for Bytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl Borrow<[u8]> for Bytes<'_> {
    fn borrow(&self) -> &[u8] {
        self
    }
}
