//! Contains the trait the client uses to store slices of memory and basic implementations.

use crate::bytes::Bytes;

#[cfg(feature = "alloc")]
pub use alloc::AllocBuffer;
#[cfg(feature = "bump")]
pub use bump::BumpBuffer;

/// A trait to describe anything that can allocate memory.
///
/// Returned memory can be borrowed or owned. Either way, it is bound by the `'a`
/// lifetime - usually just the lifetime of the underlying buffer.
///
/// The client does not store any references to memory returned by this provider.
pub trait BufferProvider<'a> {
    /// The type returned from a successful buffer provision.
    /// Must implement `AsMut<[u8]>` so that it can be borrowed mutably right after allocation for initialization
    /// and `Into<Bytes<'a>>` for storing.
    type Buffer: AsMut<[u8]> + Into<Bytes<'a>>;

    /// The error type returned from a failed buffer provision.
    type ProvisionError: core::fmt::Debug;

    /// If successful, returns contiguous memory with a size in bytes of the `len` argument.
    fn provide_buffer(&mut self, len: usize) -> Result<Self::Buffer, Self::ProvisionError>;
}

#[cfg(feature = "bump")]
mod bump {
    use core::slice;

    use crate::buffer::BufferProvider;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InsufficientSpace;

    /// Allocates memory from an underlying buffer by bumping up a pointer by the requested length.
    ///
    /// Can be resetted when no references to buffer contents exist.
    #[derive(Debug)]
    pub struct BumpBuffer<'a> {
        slice: &'a mut [u8],
        index: usize,
    }

    impl<'a> BufferProvider<'a> for BumpBuffer<'a> {
        type Buffer = &'a mut [u8];
        type ProvisionError = InsufficientSpace;

        /// Return the next `len` bytes from the buffer, advancing the internal
        /// pointer. Returns `InsufficientSpace` if there isn't enough room.
        fn provide_buffer(&mut self, len: usize) -> Result<Self::Buffer, Self::ProvisionError> {
            if self.remaining_len() < len {
                Err(InsufficientSpace)
            } else {
                let start = self.index;
                // Safety: we checked bounds above, and the pointer originates from
                // the backing slice owned by this struct with the same lifetime.
                let ptr = unsafe { self.slice.as_mut_ptr().add(start) };
                // Advance index after computing start so callers don't observe a
                // partially-advanced state if we ever change ordering.
                self.index += len;

                // Safety: the slice starts at the self.index which is not part of any other reservations.
                // self.index has been skipped ahead the slice's full length
                let slice = unsafe { slice::from_raw_parts_mut(ptr, len) };

                Ok(slice)
            }
        }
    }

    impl<'a> BumpBuffer<'a> {
        /// Creates a new `BumpBuffer` with the provided slice as underlying buffer.
        pub fn new(slice: &'a mut [u8]) -> Self {
            Self { slice, index: 0 }
        }

        /// Returns the remaining amount of unallocated bytes in the underlying buffer.
        #[inline]
        pub fn remaining_len(&self) -> usize {
            self.slice.len() - self.index
        }

        /// Invalidates all previous allocations by resetting the `BumpBuffer`'s index,
        /// allowing the underlying buffer to be reallocated.
        ///
        /// # Safety
        /// No more references exist to previously allocated slices / underlying buffer content.
        /// In the context of the client, this is the case when no server publication content
        /// (topic & message) and no reason strings still held.
        #[inline]
        pub unsafe fn reset(&mut self) {
            self.index = 0;
        }
    }

    #[cfg(test)]
    mod unit {
        use tokio_test::{assert_err, assert_ok};

        use super::*;

        #[test]
        fn provide_buffer_and_remaining_len() {
            let mut backing = [0; 10];

            {
                let mut buf = BumpBuffer::new(&mut backing);

                assert_eq!(buf.remaining_len(), 10);

                let s1 = assert_ok!(buf.provide_buffer(4));
                assert_eq!(s1.len(), 4);

                s1.copy_from_slice(&[1, 2, 3, 4]);
                assert_eq!(buf.remaining_len(), 6);

                // take remaining 6 bytes
                let s2 = assert_ok!(buf.provide_buffer(6));
                assert_eq!(s2.len(), 6);

                s2.copy_from_slice(&[5, 6, 7, 8, 9, 10]);
                assert_eq!(buf.remaining_len(), 0);

                assert_eq!(s1, [1, 2, 3, 4]);
                assert_eq!(s2, [5, 6, 7, 8, 9, 10]);

                let err = assert_err!(buf.provide_buffer(1));
                assert_eq!(err, InsufficientSpace);
            }

            assert_eq!(backing, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        }

        #[test]
        fn reset_allows_reuse() {
            let mut backing = [0; 6];

            {
                let mut buf = BumpBuffer::new(&mut backing);

                let s1 = assert_ok!(buf.provide_buffer(3));
                s1.copy_from_slice(&[11, 12, 13]);

                // reset and take again from start
                unsafe { buf.reset() }
                let s2 = assert_ok!(buf.provide_buffer(3));
                assert_eq!(s1, s2);
            }

            assert_eq!(backing, [11, 12, 13, 0, 0, 0]);
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc {
    use core::convert::Infallible;

    use alloc::boxed::Box;
    use alloc::vec;

    use crate::buffer::BufferProvider;

    /// Allocates memory using the global allocator.
    #[derive(Debug)]
    pub struct AllocBuffer;

    impl<'a> BufferProvider<'a> for AllocBuffer {
        type Buffer = Box<[u8]>;
        type ProvisionError = Infallible;

        /// Allocates `len` bytes on the heap
        fn provide_buffer(&mut self, len: usize) -> Result<Self::Buffer, Self::ProvisionError> {
            let buffer = vec![0; len].into_boxed_slice();

            Ok(buffer)
        }
    }

    #[cfg(test)]
    mod unit {
        use crate::buffer::{BufferProvider, alloc::AllocBuffer};
        use tokio_test::assert_ok;

        #[test]
        fn provide_buffer() {
            let mut alloc = AllocBuffer;

            let buffer = alloc.provide_buffer(10);
            let buffer = assert_ok!(buffer);
            assert_eq!(10, buffer.len());
        }
    }
}
