//! Contains traits and types for MQTTv5's enhanced authentication.

use heapless::Vec;

use crate::{
    client::event::Auth,
    types::{MqttBinary, MqttString, MqttStringPair, ReasonCode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum ReAuthState {
    Inactive,
    AwaitAuth,
    DueAuth,
}

/// Options for enhanced authentication for the AUTH packet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AuthOptions<'a, const MAX_USER_PROPERTIES: usize> {
    /// The authentication data property of the AUTH packet.
    pub authentication_data: Option<MqttBinary<'a>>,

    /// The reason string property of the AUTH packet.
    pub reason_string: Option<MqttString<'a>>,

    /// Arbitrary key-value pairs of strings sent as the user property entries
    /// of the AUTH packet.
    pub user_properties: Vec<MqttStringPair<'a>, MAX_USER_PROPERTIES>,
}

/// An enhanced authentication mechanism for use in MQTTv5's extended/enhanced
/// authentication. [`Client`] only requires this for enhanced authentication
/// used in [`Client::connect_enhanced`], but typically this can be reused for
/// enhanced re-authentication. Note that this trait only provides methods for
/// handling a received authentication exchange (received AUTH or CONNACK
/// packet). If authentication data is required for the first step of the
/// authentication exchange, which is always sent by the client (in the CONNECT
/// packet, in case of re-authentication this is a AUTH packet sent with
/// [`Client::reauthenticate`]), this data must be created by the user
/// beforehand.
///
/// [`Client`]: crate::client::Client
/// [`Client::connect_enhanced`]: crate::client::Client::connect_enhanced
/// [`Client::reauthenticate`]: crate::client::Client::reauthenticate
pub trait AuthMechanism<const MAX_USER_PROPERTIES: usize> {
    /// The authentication mechanism may detect formatting errors, hit errors
    /// within cryptographic implementations or fail to authenticate the server
    /// in case of mutual authentication (such as the SCRAM-SHA family). After
    /// handling the failed authentication by disconnecting, this error is
    /// wrapped in [`MqttError::EnhancedAuthFailed`] and returned.
    ///
    /// [`MqttError::EnhancedAuthFailed`]: crate::client::MqttError::EnhancedAuthFailed
    type Error;

    /// An AUTH packet with [`ReasonCode::ContinueAuthentication`] was received.
    /// The [`AuthMechanism`] executes its checks and produces the next step
    /// (which in turn is an AUTH packet) of the authentication exchange.
    ///
    /// # Returns
    ///
    /// - [`Err((Self::Error, None))`] if an error occured and the client should
    ///   disconnect without a DISCONNECT packet, i.e. close the network
    ///   connection. The contained [`AuthMechanism::Error`] is wrapped in
    ///   [`MqttError::EnhancedAuthFailed`] and returned.
    /// - [`Err((Self::Error, Some(ReasonCode)))`] if an error occured and the
    ///   client should disconnect with a DISCONNECT packet. This DISCONNECT
    ///   packet is scheduled to be sent with [`Client::abort`]. The
    ///   [`ReasonCode`] must be a valid Disconnect Reason Code (compare
    ///   [`Client::disconnect`]). The contained [`AuthMechanism::Error`] is
    ///   wrapped in [`MqttError::EnhancedAuthFailed`] and returned.
    /// - [`Ok(Auth)`] if the authentication continues regularly. The contained
    ///   [`Auth`] is converted into an AUTH packet with
    ///   [`ReasonCode::ContinueAuthentication`] and sent to the server.
    ///
    /// [`ReasonCode::ContinueAuthentication`]: crate::types::ReasonCode::ContinueAuthentication
    /// [`Err((Self::Error, None))`]: core::result::Result::Err
    /// [`MqttError::EnhancedAuthFailed`]: crate::client::MqttError::EnhancedAuthFailed
    /// [`Err((Self::Error, Some(ReasonCode)))`]: core::result::Result::Err
    /// [`Client::abort`]: crate::client::Client::abort
    /// [`Client::disconnect`]: crate::client::Client::disconnect
    /// [`Ok(Auth)`]: core::result::Result::Ok
    fn kontinue(
        &mut self,
        auth: &Auth<'_, MAX_USER_PROPERTIES>,
    ) -> Result<AuthOptions<'_, MAX_USER_PROPERTIES>, (Self::Error, Option<ReasonCode>)>;

    /// A CONNACK packet with [`ReasonCode::Success`] was received. The
    /// properties of this CONNACK packet have been mapped to the fields of the
    /// [`Auth`] parameter.
    ///
    /// In case this trait is used for enhanced re-authentication, this method
    /// typically corresponds to the receival of an AUTH packet with
    /// [`ReasonCode::Success`].
    ///
    /// There are no more steps within this authentication exchange. This method
    /// validates the data in the CONNACK (or AUTH) packet if required. It may
    /// be a no-op otherwise if no such validation is part of the authentication
    /// mechanism or if no such data is present.
    ///
    /// # Returns
    ///
    /// - [`Err((Self::Error, None))`] if an error occured and the client should
    ///   disconnect without a DISCONNECT packet, i.e. close the network
    ///   connection. The contained [`AuthMechanism::Error`] is wrapped in
    ///   [`MqttError::EnhancedAuthFailed`] and returned.
    /// - [`Err((Self::Error, Some(ReasonCode)))`] if an error occured and
    ///   the client should disconnect with a DISCONNECT packet. This DISCONNECT
    ///   packet is scheduled to be sent with [`Client::abort`]. The
    ///   [`ReasonCode`] must be a valid Disconnect Reason Code (compare
    ///   [`Client::disconnect`]). The contained [`AuthMechanism::Error`] is
    ///   wrapped in [`MqttError::EnhancedAuthFailed`] and returned.
    /// - [`Ok`] if the authentication was successful.
    ///
    /// [`ReasonCode::Success`]: crate::types::ReasonCode::Success
    /// [`Err((Self::Error, None))`]: core::result::Result::Err
    /// [`Err((Self::Error, Some(ReasonCode)))`]: core::result::Result::Err
    /// [`MqttError::EnhancedAuthFailed`]: crate::client::MqttError::EnhancedAuthFailed
    /// [`Client::abort`]: crate::client::Client::abort
    /// [`Client::disconnect`]: crate::client::Client::disconnect
    fn success(
        &mut self,
        auth: &Auth<'_, MAX_USER_PROPERTIES>,
    ) -> Result<(), (Self::Error, Option<ReasonCode>)>;
}
