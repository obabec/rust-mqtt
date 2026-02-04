use crate::{
    config::{KeepAlive, MaximumPacketSize, ReceiveMaximum, SessionExpiryInterval},
    eio::{Read, Write},
    io::{
        err::{ReadError, WriteError},
        read::{Readable, Store},
        write::{Writable, wlen},
    },
    types::{MqttBinary, MqttString, QoS, TopicName, VarByteInt},
    v5::property::{Property, PropertyType},
};

/// Implements a newtype with the given identifier and wrapped type.
///
/// * Implements `Writable`: Identifier and content are written
/// * Implements `Readable`: Only content is read. In the case of the newtype having a lifetime `'a`, the `Readable` implementation is trait bounded by `Store<'a>`
macro_rules! property {
    ($name:ident, $ty:ty) => {
        #[derive(Debug, PartialEq, Clone, Copy)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct $name(pub(crate) $ty);

        impl Property for $name {
            const TYPE: PropertyType = PropertyType::$name;
            type Inner = $ty;

            fn into_inner(self) -> Self::Inner {
                self.0
            }
        }

        impl<R: Read> Readable<R> for $name {
            async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
                let content = <$ty as Readable<R>>::read(read).await?;
                Ok(Self(content))
            }
        }

        impl Writable for $name {
            fn written_len(&self) -> usize {
                Self::TYPE.written_len() + self.0.written_len()
            }
            async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
                Self::TYPE.write(write).await?;
                self.0.write(write).await?;

                Ok(())
            }
        }

        impl From<$ty> for $name {
            fn from(value: $ty) -> Self {
                Self(value)
            }
        }
    };
    ($name:ident < $lt:lifetime >, $ty:ty) => {
        #[derive(Debug, PartialEq, Clone)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct $name<$lt>(pub(crate) $ty);

        impl<$lt> Property for $name<$lt> {
            const TYPE: PropertyType = PropertyType::$name;
            type Inner = $ty;

            fn into_inner(self) -> Self::Inner {
                self.0
            }
        }

        impl<$lt, R: Read + Store<$lt>> Readable<R> for $name<$lt> {
            async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
                let content = <$ty as Readable<R>>::read(read).await?;
                Ok(Self(content))
            }
        }

        impl<$lt> Writable for $name<$lt> {
            fn written_len(&self) -> usize {
                Self::TYPE.written_len() + self.0.written_len()
            }
            async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
                Self::TYPE.write(write).await?;
                self.0.write(write).await?;

                Ok(())
            }
        }

        impl<$lt> From<$ty> for $name<$lt> {
            fn from(value: $ty) -> Self {
                Self(value)
            }
        }
    };
}

property!(PayloadFormatIndicator, bool);
property!(MessageExpiryInterval, u32);
property!(ContentType<'c>, MqttString<'c>);
property!(ResponseTopic<'c>, TopicName<'c>);
property!(CorrelationData<'c>, MqttBinary<'c>);
property!(SubscriptionIdentifier, VarByteInt);
property!(AssignedClientIdentifier<'c>, MqttString<'c>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ServerKeepAlive(pub(crate) KeepAlive);
property!(AuthenticationMethod<'c>, MqttString<'c>);
property!(AuthenticationData<'c>, MqttBinary<'c>);
property!(RequestProblemInformation, bool);
property!(WillDelayInterval, u32);
property!(RequestResponseInformation, bool);
property!(ResponseInformation<'c>, MqttString<'c>);
property!(ServerReference<'c>, MqttString<'c>);
property!(ReasonString<'c>, MqttString<'c>);
property!(TopicAliasMaximum, u16);
property!(TopicAlias, u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MaximumQoS(pub(crate) QoS);
property!(RetainAvailable, bool);
// Insert UserProperty here
property!(WildcardSubscriptionAvailable, bool);
property!(SubscriptionIdentifierAvailable, bool);
property!(SharedSubscriptionAvailable, bool);

impl Property for ServerKeepAlive {
    const TYPE: PropertyType = PropertyType::ServerKeepAlive;
    type Inner = KeepAlive;

    fn into_inner(self) -> Self::Inner {
        self.0
    }
}

impl<R: Read> Readable<R> for ServerKeepAlive {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        let value = u16::read(read).await?;

        Ok(Self(match value {
            0 => KeepAlive::Infinite,
            s => KeepAlive::Seconds(s),
        }))
    }
}

impl Writable for ServerKeepAlive {
    fn written_len(&self) -> usize {
        if matches!(self.0, KeepAlive::Infinite) {
            0
        } else {
            Self::TYPE.written_len() + wlen!(u16)
        }
    }
    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let value = match self.0 {
            KeepAlive::Infinite => 0,
            KeepAlive::Seconds(s) => s,
        };

        if value != 0 {
            Self::TYPE.write(write).await?;
            value.write(write).await?;
        }
        Ok(())
    }
}

impl Property for SessionExpiryInterval {
    const TYPE: PropertyType = PropertyType::SessionExpiryInterval;
    type Inner = Self;

    fn into_inner(self) -> Self::Inner {
        self
    }
}

impl<R: Read> Readable<R> for SessionExpiryInterval {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        let value = u32::read(read).await?;

        Ok(match value {
            0 => Self::EndOnDisconnect,
            u32::MAX => Self::NeverEnd,
            s => Self::Seconds(s),
        })
    }
}

impl Writable for SessionExpiryInterval {
    fn written_len(&self) -> usize {
        Self::TYPE.written_len() + wlen!(u32)
    }
    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let value = match self {
            Self::EndOnDisconnect => 0,
            Self::NeverEnd => u32::MAX,
            Self::Seconds(s) => *s,
        };

        Self::TYPE.write(write).await?;
        value.write(write).await?;

        Ok(())
    }
}

impl Property for MaximumQoS {
    const TYPE: PropertyType = PropertyType::MaximumQoS;
    type Inner = QoS;

    fn into_inner(self) -> Self::Inner {
        self.0
    }
}
impl<R: Read> Readable<R> for MaximumQoS {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        let byte = u8::read(read).await?;
        let qos = QoS::try_from_bits(byte).ok_or(ReadError::MalformedPacket)?;
        Ok(Self(qos))
    }
}

impl Property for MaximumPacketSize {
    const TYPE: PropertyType = PropertyType::MaximumPacketSize;
    type Inner = Self;

    fn into_inner(self) -> Self::Inner {
        self
    }
}
impl<R: Read> Readable<R> for MaximumPacketSize {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        let max = u32::read(read).await?;

        if max > 0 {
            Ok(Self::Limit(max))
        } else {
            Err(ReadError::ProtocolError)
        }
    }
}
impl Writable for MaximumPacketSize {
    fn written_len(&self) -> usize {
        match self {
            Self::Unlimited => 0,
            Self::Limit(_) => Self::TYPE.written_len() + wlen!(u32),
        }
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        if let Self::Limit(l) = self {
            Self::TYPE.write(write).await?;
            l.write(write).await?;
        }

        Ok(())
    }
}

impl Property for ReceiveMaximum {
    const TYPE: PropertyType = PropertyType::ReceiveMaximum;
    type Inner = u16;

    fn into_inner(self) -> Self::Inner {
        self.0
    }
}
impl<R: Read> Readable<R> for ReceiveMaximum {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        let max = u16::read(read).await?;

        if max > 0 {
            Ok(Self(max))
        } else {
            Err(ReadError::ProtocolError)
        }
    }
}
impl Writable for ReceiveMaximum {
    fn written_len(&self) -> usize {
        match self.0 {
            u16::MAX => 0,
            _ => Self::TYPE.written_len() + wlen!(u16),
        }
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        if self.0 < u16::MAX {
            Self::TYPE.write(write).await?;
            self.0.write(write).await?;
        }

        Ok(())
    }
}
