use crate::{
    eio::Read,
    io::{err::ReadError, read::Readable},
};

mod types;
mod values;

pub use types::PropertyType;
pub use values::*;

/// This implementation serves as both an enabler trait for a generic Readable and Writable implementation for the Property as well as a marker trait for the following qualities of the Readable and Writable impls:
///
/// * Writable writes both its property type's identifier and its content
/// * Readable reads only the property's content
pub trait Property {
    const TYPE: PropertyType;
    type Inner;

    fn into_inner(self) -> Self::Inner;
}

/// Helper trait to read optional, but at max once properties into a packet
pub trait AtMostOnceProperty<R: Read, T: Property> {
    async fn try_set(
        &mut self,
        read: &mut R,
    ) -> Result<(), AtMostOncePropertyError<ReadError<R::Error>>>;
}
pub enum AtMostOncePropertyError<E> {
    Read(E),
    AlreadySet,
}
impl<E> From<ReadError<E>> for AtMostOncePropertyError<ReadError<E>> {
    fn from(e: ReadError<E>) -> Self {
        Self::Read(e)
    }
}
impl<R: Read, T: Property + Readable<R>> AtMostOnceProperty<R, T> for Option<T> {
    async fn try_set(
        &mut self,
        read: &mut R,
    ) -> Result<(), AtMostOncePropertyError<ReadError<R::Error>>> {
        if self.is_some() {
            Err(AtMostOncePropertyError::AlreadySet)
        } else {
            let value = T::read(read).await.map_err(AtMostOncePropertyError::Read)?;

            self.replace(value);
            Ok(())
        }
    }
}
