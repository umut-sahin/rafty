use crate::*;

/// An action to perform in a [Simulation].
pub enum Action<A: RaftApplication> {
    /// Triggers election timeout of a [Peer].
    TimeoutElection { peer_id: PeerId },
    /// Triggers election timeout of multiple [Peer]s.
    TimeoutElections { peer_ids: Vec<PeerId> },

    /// Transmits a peer request of a [Peer].
    TransmitPeerRequest { peer_id: PeerId, request_id: RequestId },
    /// Transmits multiple peer requests of a [Peer].
    TransmitPeerRequests { peer_id: PeerId, request_ids: Vec<RequestId> },

    /// Drops a peer request of a [Peer].
    DropPeerRequest { peer_id: PeerId, request_id: RequestId },
    /// Drops multiple peer requests of a [Peer].
    DropPeerRequests { peer_id: PeerId, request_ids: Vec<RequestId> },

    /// Transmits a peer reply from a [Peer].
    TransmitPeerReply { peer_id: PeerId, replied_peer_id_and_request_id: (PeerId, RequestId) },
    /// Transmits multiple peer replies from a [Peer].
    TransmitPeerReplies {
        peer_id: PeerId,
        replied_peer_ids_and_request_ids: Vec<(PeerId, RequestId)>,
    },

    /// Drops a peer reply of a [Peer].
    DropPeerReply { peer_id: PeerId, replied_peer_id_and_request_id: (PeerId, RequestId) },
    /// Drops multiple peer replies of a [Peer].
    DropPeerReplies { peer_id: PeerId, replied_peer_ids_and_request_ids: Vec<(PeerId, RequestId)> },

    /// Triggers heartbeat timeout of a [Peer].
    TimeoutHeartbeat { peer_id: PeerId },

    /// Applies committed [LogEntry]s of a [Peer] to its [Machine].
    ///
    /// If `peer_id` is `None`, applies committed entries of all peers.
    ApplyCommitted { peer_id: Option<PeerId> },

    /// Sends a [Command](RaftCommand) from a [Client].
    SendCommand { client_id: ClientId, peer_id: Option<PeerId>, command: A::Command },

    /// Sends a [Query](RaftQuery) from a [Client].
    SendQuery { client_id: ClientId, peer_id: Option<PeerId>, query: A::Query },

    /// Transmits a client request to a [Peer].
    TransmitClientRequest { client_id: ClientId, request_id: RequestId },

    /// Transmits a client reply from a [Peer].
    TransmitClientReply { peer_id: PeerId, replied_client_id_and_request_id: (ClientId, RequestId) },

    /// Drops a client reply from a [Peer].
    DropClientReply { peer_id: PeerId, replied_client_id_and_request_id: (ClientId, RequestId) },

    /// Applies [Update]s to the replay peers and checks them against actual peers.
    ///
    /// During [Simulation], [Action]s other than [Action::Check] are executed
    /// on `simulation.peers`. When an [Action::Check] is encountered, its updates
    /// are applied to `simulation.replay_peers` then `simulation.peers` are checked against
    /// `simulation.replay_peers`. Every single [Peer] property is checked, so [Action::Check]
    /// also verifies that non-updated fields are left untouched.
    ///
    /// [Action::Check] needs `simulation.enable_check(replay_peer_storages)` to work.
    Check { updates: Vec<Update<A>> },
}
