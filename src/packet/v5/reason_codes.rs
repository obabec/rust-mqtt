/*
 * MIT License
 *
 * Copyright (c) [2022] [Ondrej Babec <ond.babec@gmail.com>]
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use core::fmt::{Display, Formatter};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReasonCode {
    Success,
    GrantedQoS1,
    GrantedQoS2,
    DisconnectWithWillMessage,
    NoMatchingSubscribers,
    NoSubscriptionExisted,
    ContinueAuth,
    ReAuthenticate,
    UnspecifiedError,
    MalformedPacket,
    ProtocolError,
    ImplementationSpecificError,
    UnsupportedProtocolVersion,
    ClientIdNotValid,
    BadUserNameOrPassword,
    NotAuthorized,
    ServerUnavailable,
    ServerBusy,
    Banned,
    ServerShuttingDown,
    BadAuthMethod,
    KeepAliveTimeout,
    SessionTakeOver,
    TopicFilterInvalid,
    TopicNameInvalid,
    PacketIdentifierInUse,
    PacketIdentifierNotFound,
    ReceiveMaximumExceeded,
    TopicAliasInvalid,
    PacketTooLarge,
    MessageRateTooHigh,
    QuotaExceeded,
    AdministrativeAction,
    PayloadFormatInvalid,
    RetainNotSupported,
    QoSNotSupported,
    UseAnotherServer,
    ServerMoved,
    SharedSubscriptionNotSupported,
    ConnectionRateExceeded,
    MaximumConnectTime,
    SubscriptionIdentifiersNotSupported,
    WildcardSubscriptionNotSupported,
    TimerNotSupported,
    BuffError,
    NetworkError,
}

impl From<ReasonCode> for u8 {
    fn from(value: ReasonCode) -> Self {
        match value {
            ReasonCode::Success => 0x00,
            ReasonCode::GrantedQoS1 => 0x01,
            ReasonCode::GrantedQoS2 => 0x02,
            ReasonCode::DisconnectWithWillMessage => 0x04,
            ReasonCode::NoMatchingSubscribers => 0x10,
            ReasonCode::NoSubscriptionExisted => 0x11,
            ReasonCode::ContinueAuth => 0x18,
            ReasonCode::ReAuthenticate => 0x19,
            ReasonCode::UnspecifiedError => 0x80,
            ReasonCode::MalformedPacket => 0x81,
            ReasonCode::ProtocolError => 0x82,
            ReasonCode::ImplementationSpecificError => 0x83,
            ReasonCode::UnsupportedProtocolVersion => 0x84,
            ReasonCode::ClientIdNotValid => 0x85,
            ReasonCode::BadUserNameOrPassword => 0x86,
            ReasonCode::NotAuthorized => 0x87,
            ReasonCode::ServerUnavailable => 0x88,
            ReasonCode::ServerBusy => 0x89,
            ReasonCode::Banned => 0x8A,
            ReasonCode::ServerShuttingDown => 0x8B,
            ReasonCode::BadAuthMethod => 0x8C,
            ReasonCode::KeepAliveTimeout => 0x8D,
            ReasonCode::SessionTakeOver => 0x8E,
            ReasonCode::TopicFilterInvalid => 0x8F,
            ReasonCode::TopicNameInvalid => 0x90,
            ReasonCode::PacketIdentifierInUse => 0x91,
            ReasonCode::PacketIdentifierNotFound => 0x92,
            ReasonCode::ReceiveMaximumExceeded => 0x93,
            ReasonCode::TopicAliasInvalid => 0x94,
            ReasonCode::PacketTooLarge => 0x95,
            ReasonCode::MessageRateTooHigh => 0x96,
            ReasonCode::QuotaExceeded => 0x97,
            ReasonCode::AdministrativeAction => 0x98,
            ReasonCode::PayloadFormatInvalid => 0x99,
            ReasonCode::RetainNotSupported => 0x9A,
            ReasonCode::QoSNotSupported => 0x9B,
            ReasonCode::UseAnotherServer => 0x9C,
            ReasonCode::ServerMoved => 0x9D,
            ReasonCode::SharedSubscriptionNotSupported => 0x9E,
            ReasonCode::ConnectionRateExceeded => 0x9F,
            ReasonCode::MaximumConnectTime => 0xA0,
            ReasonCode::SubscriptionIdentifiersNotSupported => 0xA1,
            ReasonCode::WildcardSubscriptionNotSupported => 0xA2,
            ReasonCode::TimerNotSupported => 0xFD,
            ReasonCode::BuffError => 0xFE,
            ReasonCode::NetworkError => 0xFF,
        }
    }
}

impl From<u8> for ReasonCode {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => ReasonCode::Success,
            0x01 => ReasonCode::GrantedQoS1,
            0x02 => ReasonCode::GrantedQoS2,
            0x04 => ReasonCode::DisconnectWithWillMessage,
            0x10 => ReasonCode::NoMatchingSubscribers,
            0x11 => ReasonCode::NoSubscriptionExisted,
            0x18 => ReasonCode::ContinueAuth,
            0x19 => ReasonCode::ReAuthenticate,
            0x80 => ReasonCode::UnspecifiedError,
            0x81 => ReasonCode::MalformedPacket,
            0x82 => ReasonCode::ProtocolError,
            0x83 => ReasonCode::ImplementationSpecificError,
            0x84 => ReasonCode::UnsupportedProtocolVersion,
            0x85 => ReasonCode::ClientIdNotValid,
            0x86 => ReasonCode::BadUserNameOrPassword,
            0x87 => ReasonCode::NotAuthorized,
            0x88 => ReasonCode::ServerUnavailable,
            0x89 => ReasonCode::ServerBusy,
            0x8A => ReasonCode::Banned,
            0x8B => ReasonCode::ServerShuttingDown,
            0x8C => ReasonCode::BadAuthMethod,
            0x8D => ReasonCode::KeepAliveTimeout,
            0x8E => ReasonCode::SessionTakeOver,
            0x8F => ReasonCode::TopicFilterInvalid,
            0x90 => ReasonCode::TopicNameInvalid,
            0x91 => ReasonCode::PacketIdentifierInUse,
            0x92 => ReasonCode::PacketIdentifierNotFound,
            0x93 => ReasonCode::ReceiveMaximumExceeded,
            0x94 => ReasonCode::TopicAliasInvalid,
            0x95 => ReasonCode::PacketTooLarge,
            0x96 => ReasonCode::MessageRateTooHigh,
            0x97 => ReasonCode::QuotaExceeded,
            0x98 => ReasonCode::AdministrativeAction,
            0x99 => ReasonCode::PayloadFormatInvalid,
            0x9A => ReasonCode::RetainNotSupported,
            0x9B => ReasonCode::QoSNotSupported,
            0x9C => ReasonCode::UseAnotherServer,
            0x9D => ReasonCode::ServerMoved,
            0x9E => ReasonCode::SharedSubscriptionNotSupported,
            0xA0 => ReasonCode::MaximumConnectTime,
            0xA1 => ReasonCode::SubscriptionIdentifiersNotSupported,
            0xA2 => ReasonCode::WildcardSubscriptionNotSupported,
            0xFD => ReasonCode::TimerNotSupported,
            0xFE => ReasonCode::BuffError,
            _ => ReasonCode::NetworkError,
        }
    }
}

impl Display for ReasonCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match *self {
            ReasonCode::Success => write!(f, "Operation was successful!"),
            ReasonCode::GrantedQoS1 => write!(f, "Granted QoS level 1!"),
            ReasonCode::GrantedQoS2 => write!(f, "Granted QoS level 2!"),
            ReasonCode::DisconnectWithWillMessage => write!(f, "Disconnected with Will message!"),
            ReasonCode::NoMatchingSubscribers => write!(f, "No matching subscribers on broker!"),
            ReasonCode::NoSubscriptionExisted => write!(f, "Subscription not exist!"),
            ReasonCode::ContinueAuth => write!(f, "Broker asks for more AUTH packets!"),
            ReasonCode::ReAuthenticate => write!(f, "Broker requires re-authentication!"),
            ReasonCode::UnspecifiedError => write!(f, "Unspecified error!"),
            ReasonCode::MalformedPacket => write!(f, "Malformed packet sent!"),
            ReasonCode::ProtocolError => write!(f, "Protocol specific error!"),
            ReasonCode::ImplementationSpecificError => write!(f, "Implementation specific error!"),
            ReasonCode::UnsupportedProtocolVersion => write!(f, "Unsupported protocol version!"),
            ReasonCode::ClientIdNotValid => write!(f, "Client sent not valid identification"),
            ReasonCode::BadUserNameOrPassword => {
                write!(f, "Authentication error, username of password not valid!")
            }
            ReasonCode::NotAuthorized => write!(f, "Client not authorized!"),
            ReasonCode::ServerUnavailable => write!(f, "Server unavailable!"),
            ReasonCode::ServerBusy => write!(f, "Server is busy!"),
            ReasonCode::Banned => write!(f, "Client is banned on broker!"),
            ReasonCode::ServerShuttingDown => write!(f, "Server is shutting down!"),
            ReasonCode::BadAuthMethod => write!(f, "Provided bad authentication method!"),
            ReasonCode::KeepAliveTimeout => write!(f, "Client reached timeout"),
            ReasonCode::SessionTakeOver => write!(f, "Took over session!"),
            ReasonCode::TopicFilterInvalid => write!(f, "Topic filter is not valid!"),
            ReasonCode::TopicNameInvalid => write!(f, "Topic name is not valid!"),
            ReasonCode::PacketIdentifierInUse => write!(f, "Packet identifier is already in use!"),
            ReasonCode::PacketIdentifierNotFound => write!(f, "Packet identifier not found!"),
            ReasonCode::ReceiveMaximumExceeded => write!(f, "Maximum receive amount exceeded!"),
            ReasonCode::TopicAliasInvalid => write!(f, "Invalid topic alias!"),
            ReasonCode::PacketTooLarge => write!(f, "Sent packet was too large!"),
            ReasonCode::MessageRateTooHigh => write!(f, "Message rate is too high!"),
            ReasonCode::QuotaExceeded => write!(f, "Quota exceeded!"),
            ReasonCode::AdministrativeAction => write!(f, "Administrative action!"),
            ReasonCode::PayloadFormatInvalid => write!(f, "Invalid payload format!"),
            ReasonCode::RetainNotSupported => write!(f, "Message retain not supported!"),
            ReasonCode::QoSNotSupported => write!(f, "Used QoS is not supported!"),
            ReasonCode::UseAnotherServer => write!(f, "Use another server!"),
            ReasonCode::ServerMoved => write!(f, "Server moved!"),
            ReasonCode::SharedSubscriptionNotSupported => {
                write!(f, "Shared subscription is not supported")
            }
            ReasonCode::ConnectionRateExceeded => write!(f, "Connection rate exceeded!"),
            ReasonCode::MaximumConnectTime => write!(f, "Maximum connect time exceeded!"),
            ReasonCode::SubscriptionIdentifiersNotSupported => {
                write!(f, "Subscription identifier not supported!")
            }
            ReasonCode::WildcardSubscriptionNotSupported => {
                write!(f, "Wildcard subscription not supported!")
            }
            ReasonCode::TimerNotSupported => write!(f, "Timer implementation is not provided"),
            ReasonCode::BuffError => write!(f, "Error encountered during write / read from packet"),
            ReasonCode::NetworkError => write!(f, "Unknown error!"),
        }
    }
}
