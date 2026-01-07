//! Contains utilities for session management.

use heapless::Vec;

mod flight;

pub use flight::{CPublishFlightState, InFlightPublish, SPublishFlightState};

/// Session-associated information
///
/// Client identifier is not stored here as it would lead to inconsistencies with the underyling allocation system.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Session<const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize> {
    /// The currently in-flight outgoing publications.
    pub pending_client_publishes: Vec<InFlightPublish<CPublishFlightState>, RECEIVE_MAXIMUM>,
    /// The currently in-flight incoming publications.
    pub pending_server_publishes: Vec<InFlightPublish<SPublishFlightState>, SEND_MAXIMUM>,
}

impl<const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize>
    Session<RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    /// Returns whether the packet identifier is currently in-flight in a client->server publication process.
    pub fn is_used_cpublish_packet_identifier(&self, packet_identifier: u16) -> bool {
        self.cpublish_flight_state(packet_identifier).is_some()
    }
    /// Returns whether the packet identifier is currently in-flight in a server->client publication process.
    pub fn is_used_spublish_packet_identifier(&self, packet_identifier: u16) -> bool {
        self.spublish_flight_state(packet_identifier).is_some()
    }

    /// Returns the state of the publication of the packet identifier if the packet identifier is in-flight in an outgoing publication.
    pub fn cpublish_flight_state(&self, packet_identifier: u16) -> Option<CPublishFlightState> {
        self.pending_client_publishes
            .iter()
            .find(|f| f.packet_identifier == packet_identifier)
            .map(|f| f.state)
    }
    /// Returns the state of the publication of the packet identifier if the packet identifier is in-flight in an incoming publication.
    pub fn spublish_flight_state(&self, packet_identifier: u16) -> Option<SPublishFlightState> {
        self.pending_server_publishes
            .iter()
            .find(|f| f.packet_identifier == packet_identifier)
            .map(|f| f.state)
    }

    /// Returns the amount of currently in-flight outgoing publications.
    pub fn in_flight_cpublishes(&self) -> u16 {
        self.pending_client_publishes.len() as u16
    }
    /// Returns the amount of currently in-flight incoming publications.
    pub fn in_flight_spublishes(&self) -> u16 {
        self.pending_server_publishes.len() as u16
    }
    /// Returns the amount of slots for outgoing publications.
    pub fn cpublish_remaining_capacity(&self) -> u16 {
        (self.pending_client_publishes.capacity() - self.pending_client_publishes.len()) as u16
    }
    /// Returns the amount of slots for incoming publications.
    pub fn spublish_remaining_capacity(&self) -> u16 {
        (self.pending_server_publishes.capacity() - self.pending_server_publishes.len()) as u16
    }

    /// Adds an entry to await a PUBACK packet. Assumes the packet identifier has no entry currently.
    ///
    /// # Safety
    /// `self.pending_client_publishes` has free capacity.
    pub(crate) unsafe fn await_puback(&mut self, packet_identifier: u16) {
        // Safety: self.pending_client_publishes has free capacity.
        unsafe {
            self.pending_client_publishes
                .push(InFlightPublish {
                    packet_identifier,
                    state: CPublishFlightState::AwaitingPuback,
                })
                .unwrap_unchecked()
        }
    }
    /// Adds an entry to await a PUBREC packet. Assumes the packet identifier has no entry currently.
    ///
    /// # Safety
    /// `self.pending_client_publishes` has free capacity.
    pub(crate) unsafe fn await_pubrec(&mut self, packet_identifier: u16) {
        // Safety: self.pending_client_publishes has free capacity.
        unsafe {
            self.pending_client_publishes
                .push(InFlightPublish {
                    packet_identifier,
                    state: CPublishFlightState::AwaitingPubrec,
                })
                .unwrap_unchecked()
        }
    }
    /// Adds an entry to await a PUBREL packet. Assumes the packet identifier has no entry currently.
    ///
    /// # Safety
    /// `self.pending_server_publishes` has free capacity.
    pub(crate) unsafe fn await_pubrel(&mut self, packet_identifier: u16) {
        // Safety: self.pending_server_publishes has free capacity.
        unsafe {
            self.pending_server_publishes
                .push(InFlightPublish {
                    packet_identifier,
                    state: SPublishFlightState::AwaitingPubrel,
                })
                .unwrap_unchecked()
        }
    }
    /// Adds an entry to await a PUBCOMP packet. Assumes the packet identifier has no entry currently.
    ///
    /// # Safety
    /// `self.pending_client_publishes` has free capacity.
    pub(crate) unsafe fn await_pubcomp(&mut self, packet_identifier: u16) {
        // Safety: self.pending_client_publishes has free capacity.
        unsafe {
            self.pending_client_publishes
                .push(InFlightPublish {
                    packet_identifier,
                    state: CPublishFlightState::AwaitingPubcomp,
                })
                .unwrap_unchecked()
        }
    }

    pub(crate) fn remove_cpublish(
        &mut self,
        packet_identifier: u16,
    ) -> Option<CPublishFlightState> {
        self.pending_client_publishes
            .iter()
            .position(|s| s.packet_identifier == packet_identifier)
            .map(|i| {
                // Safety: `.iter().position()` confirms the index is within bounds.
                unsafe { self.pending_client_publishes.swap_remove_unchecked(i) }.state
            })
    }
    pub(crate) fn remove_spublish(
        &mut self,
        packet_identifier: u16,
    ) -> Option<SPublishFlightState> {
        self.pending_server_publishes
            .iter()
            .position(|s| s.packet_identifier == packet_identifier)
            .map(|i| {
                // Safety: `.iter().position()` confirms the index is within bounds.
                unsafe { self.pending_server_publishes.swap_remove_unchecked(i) }.state
            })
    }

    pub(crate) fn clear(&mut self) {
        self.pending_client_publishes.clear();
        self.pending_server_publishes.clear();
    }
}
