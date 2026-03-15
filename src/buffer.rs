//! Contains the trait the client uses to store slices of memory and basic implementations.

use crate::bytes::Bytes;

#[cfg(feature = "alloc")]
pub use alloc::AllocBuffer;
#[cfg(feature = "bump")]
pub use bump::{BumpBuffer, InsufficientSpace};

/// A trait to describe anything that can allocate memory.
///
/// Returned memory can be borrowed or owned. Either way, it is bound by the `'a`
/// lifetime - usually just the lifetime of the underlying buffer.
///
/// The client does not store any references to memory returned by this provider.
pub trait BufferProvider<'a> {
    /// The type returned from a successful buffer provision.
    /// Must implement [`AsMut`] so that it can be borrowed mutably right after allocation for
    /// initialization and [`Into`] for storing as [`Bytes`].
    type Buffer: AsMut<[u8]> + Into<Bytes<'a>>;

    /// The error type returned from a failed buffer provision.
    type ProvisionError: core::fmt::Debug;

    /// If successful, returns contiguous memory with a size in bytes of the `len` argument.
    fn provide_buffer(&mut self, len: usize) -> Result<Self::Buffer, Self::ProvisionError>;
}

#[cfg(feature = "bump")]
mod bump {
    use core::{marker::PhantomData, slice};

    use crate::buffer::BufferProvider;

    /// Error returned when the [`BumpBuffer`]'s underlying buffer does not have enough unallocated space.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct InsufficientSpace;

    /// Allocates memory from an underlying buffer by bumping up a pointer by the requested length.
    ///
    /// Can be reset when no references to buffer contents exist.
    #[derive(Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct BumpBuffer<'a> {
        ptr: *mut u8,
        len: usize,
        index: usize,
        _phantom_data: PhantomData<&'a mut [u8]>,
    }

    impl<'a> BufferProvider<'a> for BumpBuffer<'a> {
        type Buffer = &'a mut [u8];
        type ProvisionError = InsufficientSpace;

        /// Return the next `len` bytes from the buffer, advancing the internal tracking
        /// index. Returns [`InsufficientSpace`] if there isn't enough room.
        fn provide_buffer(&mut self, len: usize) -> Result<Self::Buffer, Self::ProvisionError> {
            if self.remaining_len() < len {
                Err(InsufficientSpace)
            } else {
                let start = self.index;

                // Safety: we checked the bounds above meaning the resulting pointer
                // is in the backing slice's range. This means the pointer arithmetic
                // does not overflow.
                // The pointer originates from the backing slice owned by this struct with the same lifetime.
                let ptr = unsafe { self.ptr.add(start) };

                self.index += len;

                // Safety: the slice starts at the self.index offset which is not part of any previous reservation.
                // Everything after this offset is not allocated and referenced.
                // The lifetime is correct as the returned slice has the same lifetime as `Self` which is
                // in turn has the lifetime of the backing slice.
                let slice = unsafe { slice::from_raw_parts_mut(ptr, len) };

                Ok(slice)
            }
        }
    }

    impl<'a> BumpBuffer<'a> {
        /// Creates a new [`BumpBuffer`] with the provided slice as underlying buffer.
        pub fn new(slice: &'a mut [u8]) -> Self {
            Self {
                ptr: slice.as_mut_ptr(),
                len: slice.len(),
                index: 0,
                _phantom_data: PhantomData,
            }
        }

        /// Returns the remaining amount of unallocated bytes in the underlying buffer.
        #[inline]
        pub fn remaining_len(&self) -> usize {
            self.len - self.index
        }

        /// Invalidates all previous allocations by resetting the [`BumpBuffer`]'s internal tracking index into the underlying
        /// buffer, allowing the underlying buffer to be reallocated down the line. After this, the bump buffer will allocate
        /// starting with the first byte of the backing buffer again.
        ///
        /// # Safety
        /// 
        /// This method is safe to call when no references to previously allocated slices or underlying buffer content exist.
        /// The caller must ensure no more such references exist. In the context of the client, this is true when no more values
        /// that have a lifetime tied to the used [`BumpBuffer`] instance exist.
        ///
        /// # Example
        ///
        /// ## Sound
        /// 
        /// ```rust,ignore
        /// use rust_mqtt::buffer::BumpBuffer;
        /// use rust_mqtt::client::Client;
        /// use rust_mqtt::client::info::ConnectInfo;
        /// use rust_mqtt::client::options::ConnectOptions;
        /// use tokio::net::TcpStream;
        /// use embedded_io_adapters::tokio_1::FromTokio;
        ///
        /// let mut buffer = [0; 1024];
        /// let mut buffer = BumpBuffer::new(&mut buffer);
        /// let mut client: Client<'_, FromTokio<TcpStream>, _, 0, 1, 0, 0> = Client::new(&mut buffer);
        ///
        /// {
        ///     // client_identifier lives inside buffer's backing buffer, so it prevents a reset call.
        ///     let ConnectInfo { client_identifier, .. } = client.connect(todo!(), &ConnectOptions::new(), None).await.unwrap();
        ///
        /// }   // client_identifier is dropped here, now we can reset the buffer
        ///
        /// // Safety: client_identifier and all other previously returned values living in buffer's backing
        /// // buffer don't exist anymore. No aliasing possible.
        /// unsafe { client.buffer_mut().reset() };
        ///
        /// // The next allocation can happen safely here.
        /// client.poll().await.unwrap();
        /// ```
        ///
        /// ## Unsound
        ///
        /// ```rust,ignore
        /// use rust_mqtt::buffer::BumpBuffer;
        /// use rust_mqtt::client::Client;
        /// use rust_mqtt::client::info::ConnectInfo;
        /// use rust_mqtt::client::options::ConnectOptions;
        /// use tokio::net::TcpStream;
        /// use embedded_io_adapters::tokio_1::FromTokio;
        ///
        /// let mut buffer = [0; 1024];
        /// let mut buffer = BumpBuffer::new(&mut buffer);
        /// let mut client: Client<'_, FromTokio<TcpStream>, _, 0, 1, 0, 0> = Client::new(&mut buffer);
        ///
        /// // client_identifier lives inside buffer's backing buffer, so it prevents a reset call.
        /// let ConnectInfo { client_identifier, .. } = client.connect(todo!(), &ConnectOptions::new(), None).await.unwrap();
        ///
        /// // (No) Safety: client_identifier still lives.
        /// unsafe { client.buffer_mut().reset() };
        ///
        /// // The next allocation can happen here and cause an alias to client_identifier.
        /// client.poll().await.unwrap();
        ///
        /// // client_identifier is still alive. It might have a different or even non-UTF-8 value now, scary...
        /// println!("{:?}", client_identifier);
        /// ```
        #[inline]
        pub unsafe fn reset(&mut self) {
            self.index = 0;
        }
    }

    fn _assert_covariant<'a, 'b: 'a>(x: BumpBuffer<'b>) -> BumpBuffer<'a> {
        x
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

                let s1 = {
                    let s1 = assert_ok!(buf.provide_buffer(3));
                    s1.copy_from_slice(&[11, 12, 13]);

                    s1.as_ptr()
                };

                // reset and take again from start
                unsafe { buf.reset() }
                let s2 = assert_ok!(buf.provide_buffer(3));

                // Checking the slices for equality is UB because we have not upheld the rules of
                // `BumpBuffer::reset` and it subsequently causes aliasing.
                // assert_eq!(s1, s2);

                assert_eq!(s1, s2.as_ptr());
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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
