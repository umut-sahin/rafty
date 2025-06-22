use crate::*;

/// A sequence of update instructions for a [Peer].
///
/// Update sequence is built with chained method calls that take and return `self` by value:
/// ```
/// # use rafty::prelude::{PeerId, Term, RaftApplication};
/// # use rafty_simulator::Update;
/// # fn x<A: RaftApplication>() -> Update<A> {
/// Update::peer(PeerId(1)).set_term(Term(1)).set_voted_for(Some(PeerId(1)))
/// # }
/// ```
pub struct Update<A: RaftApplication> {
    peer_id: PeerId,
    changes: Vec<Change<A>>,
}

impl<A: RaftApplication> Update<A> {
    /// Creates a new update for a peer.
    pub fn peer(peer_id: impl Into<PeerId>) -> Self {
        Self { peer_id: peer_id.into(), changes: Vec::new() }
    }
}

impl<A: RaftApplication> Update<A> {
    /// Sets the current term of the peer.
    pub fn set_term(mut self, new_term: impl Into<Term>) -> Self {
        self.changes.push(Change::SetTerm { new_term: new_term.into() });
        self
    }

    /// Sets the voted for of the peer.
    pub fn set_voted_for(mut self, new_voted_for: Option<PeerId>) -> Self {
        self.changes.push(Change::SetVotedFor { new_voted_for });
        self
    }

    /// Sets the log of the peer.
    pub fn set_log(mut self, new_log: Vec<LogEntry<A>>) -> Self {
        self.changes.push(Change::SetLog { new_log });
        self
    }

    /// Sets the snapshot of the peer.
    pub fn set_snapshot(mut self, new_snapshot: Snapshot<A>) -> Self {
        self.changes.push(Change::SetSnapshot { new_snapshot });
        self
    }

    /// Sets the commit index of the peer.
    pub fn set_commit_index(mut self, new_commit_index: impl Into<LogIndex>) -> Self {
        self.changes.push(Change::SetCommitIndex { new_commit_index: new_commit_index.into() });
        self
    }

    /// Sets the last applied of the peer.
    pub fn set_last_applied(mut self, new_last_applied: impl Into<LogIndex>) -> Self {
        self.changes.push(Change::SetLastApplied { new_last_applied: new_last_applied.into() });
        self
    }

    /// Sets the role of the peer.
    pub fn set_role(mut self, new_role: Role<A>) -> Self {
        self.changes.push(Change::SetRole { new_role });
        self
    }

    /// Sets the machine of the peer.
    pub fn set_machine(mut self, new_machine: A::Machine) -> Self {
        self.changes.push(Change::SetMachine { new_machine });
        self
    }

    /// Sets the buffered peer transmits of the peer.
    pub fn set_buffered_peer_transmits(
        mut self,
        new_transmits: impl Iterator<Item = PeerTransmit<A>>,
    ) -> Self {
        self.changes
            .push(Change::SetBufferedPeerTransmits { new_transmits: new_transmits.collect() });
        self
    }

    /// Clears the buffered peer transmits of the peer.
    pub fn clear_buffered_peer_transmits(mut self) -> Self {
        self.changes.push(Change::SetBufferedPeerTransmits { new_transmits: Default::default() });
        self
    }

    /// Sets the buffered client transmits of the peer.
    pub fn set_buffered_client_transmits(
        mut self,
        new_transmits: impl Iterator<Item = ClientTransmit<A>>,
    ) -> Self {
        self.changes
            .push(Change::SetBufferedClientTransmits { new_transmits: new_transmits.collect() });
        self
    }

    /// Clears the buffered client transmits of the peer.
    pub fn clear_buffered_client_transmits(mut self) -> Self {
        self.changes.push(Change::SetBufferedClientTransmits { new_transmits: Default::default() });
        self
    }
}

impl<A: RaftApplication> Update<A> {
    /// Applies the update on a cluster of peers.
    pub fn apply_to(self, peers: &mut [Peer<A>]) -> anyhow::Result<()> {
        let peer = &mut peers[self.peer_id.0 - 1];
        for change in self.changes {
            match change {
                Change::SetTerm { new_term } => {
                    peer.set_current_term(new_term).with_context(|| {
                        format!(
                            "\nUnable to set the expected Term of {} in its storage",
                            self.peer_id,
                        )
                    })?;
                },
                Change::SetVotedFor { new_voted_for } => {
                    peer.set_voted_for(new_voted_for).with_context(|| {
                        format!(
                            "\nUnable to set the expected Voted For of {} in its storage",
                            self.peer_id,
                        )
                    })?;
                },
                Change::SetLog { new_log } => {
                    peer.set_log(new_log).with_context(|| {
                        format!(
                            "\nUnable to set the expected Log of {} in its storage",
                            self.peer_id,
                        )
                    })?;
                },
                Change::SetSnapshot { new_snapshot } => {
                    peer.set_snapshot(new_snapshot).with_context(|| {
                        format!(
                            "\nUnable to set the expected Snapshot of {} in its storage",
                            self.peer_id,
                        )
                    })?;
                },
                Change::SetCommitIndex { new_commit_index } => {
                    peer.set_commit_index(new_commit_index);
                },
                Change::SetLastApplied { new_last_applied } => {
                    peer.set_last_applied(new_last_applied);
                },
                Change::SetRole { new_role } => {
                    peer.set_role(new_role);
                },
                Change::SetMachine { new_machine } => {
                    peer.set_machine(new_machine);
                },
                Change::SetBufferedPeerTransmits { new_transmits } => {
                    peer.set_buffered_peer_transmits(new_transmits);
                },
                Change::SetBufferedClientTransmits { new_transmits } => {
                    peer.set_buffered_client_transmits(new_transmits);
                },
            }
        }
        Ok(())
    }
}

#[allow(clippy::enum_variant_names)]
enum Change<A: RaftApplication> {
    SetTerm { new_term: Term },
    SetVotedFor { new_voted_for: Option<PeerId> },
    SetLog { new_log: Vec<LogEntry<A>> },
    SetSnapshot { new_snapshot: Snapshot<A> },
    SetCommitIndex { new_commit_index: LogIndex },
    SetLastApplied { new_last_applied: LogIndex },
    SetRole { new_role: Role<A> },
    SetMachine { new_machine: A::Machine },
    SetBufferedPeerTransmits { new_transmits: VecDeque<PeerTransmit<A>> },
    SetBufferedClientTransmits { new_transmits: VecDeque<ClientTransmit<A>> },
}
