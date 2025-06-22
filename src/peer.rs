//! Peer definitions.

use crate::prelude::*;

/// Peers to control replication and manage requests from [Client]s.
pub struct Peer<A: Application> {
    pub(crate) id: PeerId,
    pub(crate) cluster: Cluster,
    pub(crate) consistency: Consistency,

    pub(crate) role: Role<A>,
    pub(crate) machine: A::Machine,
    pub(crate) storage: A::Storage,

    pub(crate) commit_index: LogIndex,
    pub(crate) last_applied: LogIndex,

    pub(crate) request_counter: RequestCounter,

    pub(crate) buffered_peer_transmits: VecDeque<PeerTransmit<A>>,
    pub(crate) buffered_client_transmits: VecDeque<ClientTransmit<A>>,
}

impl<A: Application> Peer<A> {
    /// Creates a new peer.
    pub fn new(
        id: PeerId,
        cluster: Cluster,
        consistency: Consistency,
        storage: A::Storage,
    ) -> Self {
        let role = Role::default();

        let snapshot = storage.snapshot();
        let machine = snapshot.machine().clone();

        let commit_index = snapshot.last_included_index();
        let last_applied = snapshot.last_included_index();

        let request_counter = RequestCounter::default();

        let buffered_peer_transmits = VecDeque::default();
        let buffered_client_transmits = VecDeque::default();

        Self {
            id,
            cluster,
            consistency,
            role,
            machine,
            storage,
            commit_index,
            last_applied,
            request_counter,
            buffered_peer_transmits,
            buffered_client_transmits,
        }
    }
}

impl<A: Application> Peer<A> {
    /// Gets the identifier of the peer.
    pub fn id(&self) -> PeerId {
        self.id
    }

    /// Gets the cluster of the peer.
    pub fn cluster(&self) -> &Cluster {
        &self.cluster
    }

    /// Gets how many peers are required to achieve majority within the [Cluster] the peer is in.
    pub fn majority(&self) -> usize {
        (self.cluster.len() / 2) + 1
    }

    /// Gets the role of the peer.
    pub fn role(&self) -> &Role<A> {
        &self.role
    }

    /// Gets the machine of the peer.
    pub fn machine(&self) -> &A::Machine {
        &self.machine
    }

    /// Gets the storage of the peer.
    pub fn storage(&self) -> &A::Storage {
        &self.storage
    }

    /// Gets the current term of the peer.
    pub fn current_term(&self) -> Term {
        self.storage.current_term()
    }

    /// Gets the voted for of the peer.
    pub fn voted_for(&self) -> Option<PeerId> {
        self.storage.voted_for()
    }

    /// Gets the log of the peer.
    pub fn log(&self) -> &Log<A> {
        self.storage.log()
    }

    /// Gets the latest snapshot of the peer.
    pub fn snapshot(&self) -> &Snapshot<A> {
        self.storage.snapshot()
    }

    /// Gets the commit index of the peer.
    pub fn commit_index(&self) -> LogIndex {
        self.commit_index
    }

    /// Gets the last applied of the peer.
    pub fn last_applied(&self) -> LogIndex {
        self.last_applied
    }

    /// Gets the buffered peer transmits of the peer.
    pub fn buffered_peer_transmits(&self) -> &VecDeque<PeerTransmit<A>> {
        &self.buffered_peer_transmits
    }

    /// Gets the buffered client transmits of the peer.
    pub fn buffered_client_transmits(&self) -> &VecDeque<ClientTransmit<A>> {
        &self.buffered_client_transmits
    }
}

impl<A: Application> Peer<A> {
    /// Triggers an election timout on the peer.
    pub fn trigger_election_timeout(&mut self) {
        log::info!("({}) Election timed out.", self.id);

        let current_term = self.current_term();
        let new_term = current_term.next();

        log::info!(
            "({}) Stepping up to become a candidate for term {} and voting for self.",
            self.id,
            new_term,
        );
        if let Err(error) = self.storage.set_current_term_and_voted_for(new_term, Some(self.id)) {
            log::error!(
                "({}) Failed to persistently update current term to {} and voted for to self ({}).",
                self.id,
                new_term,
                error,
            );
            log::info!("({}) Going back to being a follower.", self.id);
            return;
        };

        if self.cluster.len() == 1 {
            self.become_leader();
            return;
        }

        let request = RequestVoteRequest::builder()
            .term(self.current_term())
            .candidate_id(self.id)
            .last_log_index(
                self.log()
                    .last()
                    .map(|entry| entry.index())
                    .unwrap_or(self.snapshot().last_included_index()),
            )
            .last_log_term(
                self.log()
                    .last()
                    .map(|entry| entry.term())
                    .unwrap_or(self.snapshot().last_included_term()),
            )
            .build();

        let mut request_ids = BTreeSet::new();
        for peer_id in self.cluster.iter().copied() {
            if peer_id == self.id {
                continue;
            }

            let request_id = self.request_counter.next();
            let transmit = PeerTransmit::builder()
                .peer_id(peer_id)
                .request_id(request_id)
                .message(request.clone())
                .build();
            request_ids.insert(transmit.request_id());
            self.buffered_peer_transmits.push_back(transmit);
        }

        self.role = Role::Candidate(
            CandidateState::builder().votes_granted(1).vote_request_ids(request_ids).build(),
        );
    }

    /// Triggers a heartbeat timout on the peer.
    pub fn trigger_heartbeat_timeout(&mut self) {
        let request = AppendEntriesRequest::builder()
            .term(self.current_term())
            .leader_id(self.id)
            .prev_log_index(
                self.log()
                    .last()
                    .map(|entry| entry.index())
                    .unwrap_or(self.snapshot().last_included_index()),
            )
            .prev_log_term(
                self.log()
                    .last()
                    .map(|entry| entry.term())
                    .unwrap_or(self.snapshot().last_included_term()),
            )
            .entries([])
            .leader_commit(self.commit_index())
            .build();
        if let Role::Leader(leader_state) = &mut self.role {
            for peer_id in self.cluster.iter().copied() {
                if peer_id == self.id {
                    continue;
                }

                let request_id = self.request_counter.next();
                let transmit = PeerTransmit::builder()
                    .peer_id(peer_id)
                    .request_id(request_id)
                    .message(request.clone())
                    .build();
                leader_state.append_entries_requests.insert(transmit.request_id(), request.clone());
                self.buffered_peer_transmits.push_back(transmit);
            }
        } else {
            log::warn!(
                "({}) Heartbeat timed out but is ignored as {}.",
                self.id,
                match self.role {
                    Role::Follower(_) => "a follower",
                    Role::Candidate(_) => "a candidate",
                    Role::Leader(_) => unreachable!(),
                }
            );
        }
    }

    /// Receives a message from another peer and updates internal state accordingly.
    pub fn receive_peer_message(
        &mut self,
        peer_id: PeerId,
        request_id: RequestId,
        message: PeerMessage<A>,
    ) {
        match message {
            PeerMessage::RequestVoteRequest(request) => {
                let reply = request.receive(peer_id, self);
                let transmit = PeerTransmit::builder()
                    .peer_id(peer_id)
                    .request_id(request_id)
                    .message(reply)
                    .build();
                self.buffered_peer_transmits.push_back(transmit);
            },
            PeerMessage::RequestVoteReply(reply) => {
                reply.receive(peer_id, request_id, self);
            },

            PeerMessage::AppendEntriesRequest(request) => {
                let reply = request.receive(peer_id, self);
                let transmit = PeerTransmit::builder()
                    .peer_id(peer_id)
                    .request_id(request_id)
                    .message(reply)
                    .build();
                self.buffered_peer_transmits.push_back(transmit);
            },
            PeerMessage::AppendEntriesReply(reply) => {
                reply.receive(peer_id, request_id, self);
            },
        }
    }

    /// Receives a client message and updates internal state accordingly.
    pub fn receive_client_message(
        &mut self,
        client_id: ClientId,
        request_id: RequestId,
        message: ClientMessage<A>,
    ) {
        match message {
            ClientMessage::QueryReply(_) | ClientMessage::CommandReply(_) => {
                log::warn!(
                    "({}) Client {} sent a reply which shouldn't have happened.",
                    self.id,
                    client_id,
                );
            },
            ClientMessage::QueryRequest(request) => {
                let reply = request.receive(client_id, request_id, self);
                if let Some(reply) = reply {
                    let transmit = ClientTransmit::builder()
                        .peer_id(self.id)
                        .client_id(client_id)
                        .request_id(request_id)
                        .message(reply)
                        .build();
                    self.buffered_client_transmits.push_back(transmit);
                }
            },
            ClientMessage::CommandRequest(request) => {
                let reply = request.receive(client_id, request_id, self);
                if let Some(reply) = reply {
                    let transmit = ClientTransmit::builder()
                        .peer_id(self.id)
                        .client_id(client_id)
                        .request_id(request_id)
                        .message(reply)
                        .build();
                    self.buffered_client_transmits.push_back(transmit);
                }
            },
        }
    }

    /// Applies commands of log entries that are replicated by majority to the machine of the peer.
    pub fn apply_committed(&mut self) {
        let mut last_applied = self.last_applied;
        while last_applied < self.commit_index {
            last_applied = last_applied.next();
            match self.storage.log().entry(last_applied) {
                Some(entry) => {
                    log::info!("({}) Applying `{:?}`.", self.id, entry,);

                    let command = entry.command();
                    self.machine.apply(command);
                },
                None => {
                    unreachable!()
                },
            }
        }
        self.last_applied = last_applied;
    }
}

impl<A: Application> Peer<A> {
    pub(crate) fn become_leader(&mut self) {
        log::info!("({}) Received the majority of the votes.", self.id);
        log::info!("({}) Stepping up to become the leader.", self.id);

        let prev_log_index = self
            .log()
            .last()
            .map(|entry| entry.index())
            .unwrap_or(self.snapshot().last_included_index());
        let prev_log_term = self
            .log()
            .last()
            .map(|entry| entry.term())
            .unwrap_or(self.snapshot().last_included_term());

        let no_op = A::Command::no_op();
        let no_op_log_index = prev_log_index.next();

        let no_op_entry = LogEntry::builder()
            .index(no_op_log_index)
            .term(self.current_term())
            .command(no_op)
            .build();
        log::info!(
            "({}) Appending `{:?}` as the leader and instructing the peers to do the same.",
            self.id,
            no_op_entry,
        );
        self.storage.append_log_entry(no_op_entry.clone()).unwrap();

        let request = AppendEntriesRequest::builder()
            .term(self.current_term())
            .leader_id(self.id)
            .prev_log_index(prev_log_index)
            .prev_log_term(prev_log_term)
            .entries([no_op_entry])
            .leader_commit(self.commit_index())
            .build();

        let mut append_entries_requests = BTreeMap::default();
        for peer_id in self.cluster.iter().copied() {
            if peer_id == self.id {
                continue;
            }

            let request_id = self.request_counter.next();
            let transmit = PeerTransmit::builder()
                .peer_id(peer_id)
                .request_id(request_id)
                .message(request.clone())
                .build();
            append_entries_requests.insert(transmit.request_id(), request.clone());
            self.buffered_peer_transmits.push_back(transmit);
        }

        let mut next_index = BTreeMap::new();
        for peer_id in self.cluster.iter().copied() {
            if peer_id == self.id {
                continue;
            }
            next_index.insert(peer_id, no_op_log_index.next());
        }

        let mut match_index = BTreeMap::new();
        for peer_id in self.cluster.iter().copied() {
            if peer_id == self.id {
                match_index.insert(peer_id, no_op_log_index);
            } else {
                match_index.insert(peer_id, self.snapshot().last_included_index());
            }
        }

        self.role = Role::Leader(
            LeaderState::builder()
                .next_index(next_index)
                .match_index(match_index)
                .append_entries_requests(append_entries_requests)
                .build(),
        );
    }
}

#[cfg(feature = "direct-control")]
impl<A: Application> Peer<A> {
    /// Overwrites the current term of the peer persistently.
    ///
    /// Should only be used for testing purposes!
    pub fn set_current_term(&mut self, new_term: Term) -> Result<(), A::StorageError> {
        self.storage.set_current_term(new_term)
    }

    /// Overwrites the voted for of the peer persistently.
    ///
    /// Should only be used for testing purposes!
    pub fn set_voted_for(&mut self, new_voted_for: Option<PeerId>) -> Result<(), A::StorageError> {
        self.storage.set_voted_for(new_voted_for)
    }

    /// Overwrites the log of the peer persistently.
    ///
    /// Should only be used for testing purposes!
    pub fn set_log(&mut self, new_log: Vec<LogEntry<A>>) -> Result<(), A::StorageError> {
        self.storage.truncate_log(LogIndex(0))?;
        for entry in new_log.iter().cloned() {
            self.storage.append_log_entry(entry)?;
        }
        Ok(())
    }

    /// Overwrites the snapshot of the peer persistently.
    ///
    /// Should only be used for testing purposes!
    pub fn set_snapshot(&mut self, new_snapshot: Snapshot<A>) -> Result<(), A::StorageError> {
        self.storage.install_snapshot(new_snapshot)
    }

    /// Overwrites the commit index of the peer.
    ///
    /// Should only be used for testing purposes!
    pub fn set_commit_index(&mut self, new_commit_index: LogIndex) {
        self.commit_index = new_commit_index;
    }

    /// Overwrites the last applied of the peer.
    ///
    /// Should only be used for testing purposes!
    pub fn set_last_applied(&mut self, new_last_applied: LogIndex) {
        self.last_applied = new_last_applied;
    }

    /// Overwrites the role of the peer.
    ///
    /// Should only be used for testing purposes!
    pub fn set_role(&mut self, new_role: Role<A>) {
        self.role = new_role;
    }

    /// Overwrites the machine of the peer.
    ///
    /// Should only be used for testing purposes!
    pub fn set_machine(&mut self, new_machine: A::Machine) {
        self.machine = new_machine;
    }

    /// Gets the buffered peer transmits of the peer mutably.
    pub fn buffered_peer_transmits_mut(&mut self) -> &mut VecDeque<PeerTransmit<A>> {
        &mut self.buffered_peer_transmits
    }

    /// Overwrites the buffered peer transmits of the peer.
    ///
    /// Should only be used for testing purposes!
    pub fn set_buffered_peer_transmits(
        &mut self,
        new_buffered_peer_transmits: VecDeque<PeerTransmit<A>>,
    ) {
        self.buffered_peer_transmits = new_buffered_peer_transmits;
    }

    /// Gets the buffered client transmits of the peer mutably.
    pub fn buffered_client_transmits_mut(&mut self) -> &mut VecDeque<ClientTransmit<A>> {
        &mut self.buffered_client_transmits
    }

    /// Overwrites the buffered client transmits of the peer.
    ///
    /// Should only be used for testing purposes!
    pub fn set_buffered_client_transmits(
        &mut self,
        new_buffered_client_transmits: VecDeque<ClientTransmit<A>>,
    ) {
        self.buffered_client_transmits = new_buffered_client_transmits;
    }
}
