//! Transmit definitions.

use crate::prelude::*;

/// Transmit between a [Peer] and another [Peer].
#[derive(Clone, Debug, Eq, PartialEq, bon::Builder)]
pub struct PeerTransmit<A: Application> {
    #[builder(into)]
    peer_id: PeerId,

    #[builder(into)]
    request_id: RequestId,

    #[builder(into)]
    message: PeerMessage<A>,
}

impl<A: Application> PeerTransmit<A> {
    /// Gets the target [PeerId] of the transmit.
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Gets the [RequestId] of the transmit.
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Gets the [PeerMessage] of the transmit.
    pub fn message(&self) -> &PeerMessage<A> {
        &self.message
    }
}

impl<A: Application> PeerTransmit<A> {
    /// Converts the transmit into its [PeerMessage].
    pub fn into_message(self) -> PeerMessage<A> {
        self.message
    }
}

/// Transmit between a [Peer] and a [Client].
#[derive(Debug, bon::Builder)]
pub struct ClientTransmit<A: Application> {
    #[builder(into)]
    client_id: ClientId,

    #[builder(into)]
    peer_id: PeerId,

    #[builder(into)]
    request_id: RequestId,

    #[builder(into)]
    message: ClientMessage<A>,
}

impl<A: Application> ClientTransmit<A> {
    /// Gets the source [ClientId] of the transmit.
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    /// Gets the target [PeerId] of the transmit.
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Gets the [RequestId] of the transmit.
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Gets the [ClientMessage] of the transmit.
    pub fn message(&self) -> &ClientMessage<A> {
        &self.message
    }
}

impl<A: Application> ClientTransmit<A> {
    /// Converts the transmit into its [PeerMessage].
    pub fn into_message(self) -> ClientMessage<A> {
        self.message
    }
}
