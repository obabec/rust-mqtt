use core::{borrow::Borrow, fmt, ops::Deref};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// Contiguous bytes in memory. Is either a `u8` slice or (with crate feature "alloc") an owned `Box<[u8]>`.
///
/// It is recommended to almost always use owned `Bytes<'a>` instead of a reference `&Bytes<'a>`,
/// as it makes this type compatible for code designed for both owned and borrowed variants.
///
/// Important: Cloning this will clone the underlying Box if it is an owned variant.
/// You can however borrow another owned `Bytes<'a>` by calling `as_borrowed()`.
/// The `as_borrowed()` method is passed on through wrapper types, for example `MqttString`.
pub enum Bytes<'a> {
    /// Owned variant, only available with the `alloc` feature enabled.
    #[cfg(feature = "alloc")]
    Owned(Box<[u8]>),

    /// Borrowed variant.
    Borrowed(&'a [u8]),
}

impl<'a> Bytes<'a> {
    /// Returns the underlying data as `&[u8]`
    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => b,
            Self::Borrowed(s) => s,
        }
    }

    /// Borrows `self` with its full lifetime to create another owned `Self` instance.
    #[inline]
    pub const fn as_borrowed(&'a self) -> Self {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => Self::Borrowed(b),
            Self::Borrowed(s) => Self::Borrowed(s),
        }
    }

    /// Returns the number of bytes.
    #[inline]
    pub const fn len(&self) -> usize {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => b.len(),
            Self::Borrowed(s) => s.len(),
        }
    }

    /// Returns whether the underlying data has a length of 0.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> Clone for Bytes<'a> {
    fn clone(&self) -> Self {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(b) => Self::Owned(b.clone()),
            Self::Borrowed(s) => Self::Borrowed(s),
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> From<Bytes<'a>> for Box<[u8]> {
    fn from(bytes: Bytes<'a>) -> Self {
        match bytes {
            Bytes::Owned(b) => b,
            Bytes::Borrowed(s) => s.into(),
        }
    }
}

impl<'a> Default for Bytes<'a> {
    fn default() -> Self {
        Self::Borrowed(&[])
    }
}

impl<'a> fmt::Debug for Bytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Borrowed(x) => f.debug_tuple("Borrowed").field(x).finish(),
            #[cfg(feature = "alloc")]
            Self::Owned(x) => f.debug_tuple("Owned").field(x).finish(),
        }
    }
}

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for Bytes<'a> {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Self::Borrowed(x) => defmt::write!(fmt, "Borrowed({:?})", *x),
            #[cfg(feature = "alloc")]
            Self::Owned(x) => defmt::write!(fmt, "Owned({:?})", x.as_ref()),
        }
    }
}

impl<'a> PartialEq for Bytes<'a> {
    fn eq(&self, other: &Self) -> bool {
        let one: &[u8] = self;
        let other: &[u8] = other;

        one == other
    }
}
impl<'a> Eq for Bytes<'a> {}

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
impl<'a> From<Box<[u8]>> for Bytes<'a> {
    fn from(value: Box<[u8]>) -> Self {
        Self::Owned(value)
    }
}

impl<'a> Deref for Bytes<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl<'a> Borrow<[u8]> for Bytes<'a> {
    fn borrow(&self) -> &[u8] {
        self
    }
}
