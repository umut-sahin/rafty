use crate::*;

/// A simulation of a [RaftApplication].
pub struct Simulation<A: RaftApplication> {
    clients: Vec<Client<A>>,
    consistency: Consistency,
    peers: Vec<Peer<A>>,
    replay_peers: Vec<Peer<A>>,
}

impl<A: RaftApplication> Simulation<A> {
    /// Creates a new simulation.
    pub fn new(
        consistency: Consistency,
        initial_peer_storages: Vec<A::Storage>,
        number_of_clients: usize,
    ) -> anyhow::Result<Self> {
        assert_ne!(number_of_clients, 0);
        assert_ne!(initial_peer_storages.len(), 0);

        let cluster =
            Cluster::from((1..=initial_peer_storages.len()).map(PeerId).collect::<BTreeSet<_>>());

        let clients = (1..=number_of_clients)
            .map(ClientId)
            .map(|client_id| Client::new(client_id, cluster.clone()))
            .collect();

        let mut peers = Vec::with_capacity(cluster.len());
        for (peer_index, initial_storage) in initial_peer_storages.into_iter().enumerate() {
            let peer_id = PeerId(peer_index + 1);
            peers.push(Peer::<A>::new(peer_id, cluster.clone(), consistency, initial_storage));
        }

        Ok(Self { clients, consistency, peers, replay_peers: vec![] })
    }

    /// Enables support for [Action::Check] using replay storages.
    pub fn enable_checks(mut self, replay_storages: Vec<A::Storage>) -> anyhow::Result<Self> {
        assert_eq!(replay_storages.len(), self.number_of_peers());

        let cluster =
            Cluster::from((1..=replay_storages.len()).map(PeerId).collect::<BTreeSet<_>>());

        let mut replay_peers = Vec::with_capacity(self.peers.len());
        for (peer_index, replay_storage) in replay_storages.into_iter().enumerate() {
            let peer_id = PeerId(peer_index + 1);
            replay_peers.push(Peer::<A>::new(
                peer_id,
                cluster.clone(),
                self.consistency,
                replay_storage,
            ));
        }
        self.replay_peers = replay_peers;

        Ok(self)
    }
}

impl<A: RaftApplication> Simulation<A> {
    /// Gets the number of clients in the simulation.
    pub fn number_of_clients(&self) -> usize {
        self.clients.len()
    }

    /// Gets the number of peers in the simulation.
    pub fn number_of_peers(&self) -> usize {
        self.peers.len()
    }

    /// Gets the peer with the given identifier within the simulation.
    pub fn peer(&self, peer_id: PeerId) -> &Peer<A> {
        &self.peers[peer_id.0 - 1]
    }

    /// Gets the peer with the given identifier within the simulation mutably.
    pub fn peer_mut(&mut self, peer_id: PeerId) -> &mut Peer<A> {
        &mut self.peers[peer_id.0 - 1]
    }

    /// Gets the client with the given identifier within the simulation.
    pub fn client(&self, client_id: ClientId) -> &Client<A> {
        &self.clients[client_id.0 - 1]
    }

    /// Gets the client with the given identifier within the simulation mutably.
    pub fn client_mut(&mut self, client_id: ClientId) -> &mut Client<A> {
        &mut self.clients[client_id.0 - 1]
    }
}

impl<A: RaftApplication> Simulation<A> {
    /// Runs a sequence of actions in the simulation.
    pub fn run(&mut self, actions: impl Iterator<Item = Action<A>>) -> anyhow::Result<()> {
        for (index, action) in actions.enumerate() {
            let action_name = match action {
                Action::TimeoutElection { .. } => "TimeoutElection",
                Action::TimeoutElections { .. } => "TimeoutElections",

                Action::TransmitPeerRequest { .. } => "TransmitRequest",
                Action::TransmitPeerRequests { .. } => "TransmitRequests",

                Action::DropPeerRequest { .. } => "DropPeerRequest",
                Action::DropPeerRequests { .. } => "DropPeerRequests",

                Action::TransmitPeerReply { .. } => "TransmitReply",
                Action::TransmitPeerReplies { .. } => "TransmitReplies",
                Action::DropPeerReply { .. } => "DropPeerReply",
                Action::DropPeerReplies { .. } => "DropPeerReplies",

                Action::TimeoutHeartbeat { .. } => "TimeoutHeartbeat",
                Action::ApplyCommitted { .. } => "ApplyCommitted",

                Action::SendCommand { .. } => "SendCommand",
                Action::SendQuery { .. } => "SendQuery",

                Action::TransmitClientRequest { .. } => "TransmitClientRequest",
                Action::TransmitClientReply { .. } => "TransmitClientReply",
                Action::DropClientReply { .. } => "DropClientReply",

                Action::Check { .. } => "Check",
            };
            self.perform(action)
                .with_context(|| format!("Failed to run Action #{index} ({action_name})"))?;
        }
        Ok(())
    }

    /// Performs a single action in the simulation.
    pub fn perform(&mut self, action: Action<A>) -> anyhow::Result<()> {
        match action {
            Action::TimeoutElection { peer_id } => {
                let peer = self.peer_mut(peer_id);
                peer.trigger_election_timeout();
            },
            Action::TimeoutElections { peer_ids } => {
                for peer_id in peer_ids {
                    let raft = self.peer_mut(peer_id);
                    raft.trigger_election_timeout();
                }
            },

            Action::TransmitPeerRequest { peer_id, request_id } => {
                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_request() && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(transmit) => {
                                let target_peer = self.peer_mut(transmit.peer_id());
                                target_peer.receive_peer_message(
                                    peer_id,
                                    transmit.request_id(),
                                    transmit.into_message(),
                                );
                            },
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot transmit {} of {} as it doesn't exist",
                            request_id,
                            peer_id,
                        ));
                    },
                }
            },
            Action::TransmitPeerRequests { peer_id, request_ids } => {
                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                let mut ordered_transmits = BTreeMap::new();

                let mut new_buffered_transmits = VecDeque::with_capacity(buffered_transmits.len());
                for transmit in std::mem::take(buffered_transmits) {
                    if !transmit.message().is_request() {
                        new_buffered_transmits.push_back(transmit);
                        continue;
                    }

                    let request_id = transmit.request_id();
                    match request_ids.iter().position(|candidate| *candidate == request_id) {
                        Some(position) => {
                            ordered_transmits.insert(position, transmit);
                        },
                        None => {
                            new_buffered_transmits.push_back(transmit);
                        },
                    }
                }

                let mut non_existing_request_ids =
                    request_ids.iter().copied().collect::<BTreeSet<_>>();

                for transmit in ordered_transmits.into_values() {
                    non_existing_request_ids.remove(&transmit.request_id());

                    let target_peer = self.peer_mut(transmit.peer_id());
                    target_peer.receive_peer_message(
                        peer_id,
                        transmit.request_id(),
                        transmit.into_message(),
                    );
                }

                if !non_existing_request_ids.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Cannot transmit requests {:?} of {} as they don't exist",
                        non_existing_request_ids,
                        peer_id,
                    ));
                }

                let peer = self.peer_mut(peer_id);
                *peer.buffered_peer_transmits_mut() = new_buffered_transmits;
            },

            Action::DropPeerRequest { peer_id, request_id } => {
                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_request() && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(_) => {},
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot drop request {} of {} as it doesn't exist",
                            request_id,
                            peer_id,
                        ));
                    },
                }
            },
            Action::DropPeerRequests { peer_id, request_ids } => {
                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                let mut ordered_transmits = BTreeMap::new();

                let mut new_buffered_transmits = VecDeque::with_capacity(buffered_transmits.len());
                for transmit in std::mem::take(buffered_transmits) {
                    if !transmit.message().is_request() {
                        new_buffered_transmits.push_back(transmit);
                        continue;
                    }

                    let request_id = transmit.request_id();
                    match request_ids.iter().position(|candidate| *candidate == request_id) {
                        Some(position) => {
                            ordered_transmits.insert(position, transmit);
                        },
                        None => {
                            new_buffered_transmits.push_back(transmit);
                        },
                    }
                }

                let mut non_existing_request_ids =
                    request_ids.iter().copied().collect::<BTreeSet<_>>();

                for transmit in ordered_transmits.into_values() {
                    non_existing_request_ids.remove(&transmit.request_id());
                }

                if !non_existing_request_ids.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Cannot drop requests {:?} of {} as they don't exist",
                        non_existing_request_ids,
                        peer_id,
                    ));
                }

                let peer = self.peer_mut(peer_id);
                *peer.buffered_peer_transmits_mut() = new_buffered_transmits;
            },

            Action::TransmitPeerReply {
                peer_id,
                replied_peer_id_and_request_id: replied_peer_and_request_id,
            } => {
                let (replied_peer_id, request_id) = replied_peer_and_request_id;

                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_reply()
                        && transmit.peer_id() == replied_peer_id
                        && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(transmit) => {
                                let target_peer = self.peer_mut(transmit.peer_id());
                                target_peer.receive_peer_message(
                                    peer_id,
                                    transmit.request_id(),
                                    transmit.into_message(),
                                );
                            },
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot transmit the reply of {} of {} from {} as it doesn't exist",
                            request_id,
                            replied_peer_id,
                            peer_id,
                        ));
                    },
                }
            },
            Action::TransmitPeerReplies {
                peer_id,
                replied_peer_ids_and_request_ids: replied_peer_and_request_ids,
            } => {
                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                let mut ordered_transmits = BTreeMap::new();

                let mut new_buffered_transmits = VecDeque::with_capacity(buffered_transmits.len());
                for transmit in std::mem::take(buffered_transmits) {
                    if !transmit.message().is_reply() {
                        new_buffered_transmits.push_back(transmit);
                        continue;
                    }

                    let peer_id = transmit.peer_id();
                    let request_id = transmit.request_id();
                    match replied_peer_and_request_ids
                        .iter()
                        .position(|candidate| *candidate == (peer_id, request_id))
                    {
                        Some(position) => {
                            ordered_transmits.insert(position, transmit);
                        },
                        None => {
                            new_buffered_transmits.push_back(transmit);
                        },
                    }
                }

                let mut non_existing_replied_peer_and_request_ids =
                    replied_peer_and_request_ids.iter().copied().collect::<BTreeSet<_>>();

                for transmit in ordered_transmits.into_values() {
                    non_existing_replied_peer_and_request_ids
                        .remove(&(transmit.peer_id(), transmit.request_id()));

                    let target_peer = self.peer_mut(transmit.peer_id());
                    target_peer.receive_peer_message(
                        peer_id,
                        transmit.request_id(),
                        transmit.into_message(),
                    );
                }

                if !non_existing_replied_peer_and_request_ids.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Cannot transmit replies {:?} of {} as they don't exist",
                        non_existing_replied_peer_and_request_ids,
                        peer_id,
                    ));
                }

                let peer = self.peer_mut(peer_id);
                *peer.buffered_peer_transmits_mut() = new_buffered_transmits;
            },

            Action::DropPeerReply { peer_id, replied_peer_id_and_request_id } => {
                let (replied_peer_id, request_id) = replied_peer_id_and_request_id;

                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_reply()
                        && transmit.peer_id() == replied_peer_id
                        && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(_) => {},
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot drop the reply of {} of {} from {} as it doesn't exist",
                            request_id,
                            replied_peer_id,
                            peer_id,
                        ));
                    },
                }
            },
            Action::DropPeerReplies { peer_id, replied_peer_ids_and_request_ids } => {
                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_peer_transmits_mut();

                let mut ordered_transmits = BTreeMap::new();

                let mut new_buffered_transmits = VecDeque::with_capacity(buffered_transmits.len());
                for transmit in std::mem::take(buffered_transmits) {
                    if !transmit.message().is_reply() {
                        new_buffered_transmits.push_back(transmit);
                        continue;
                    }

                    let peer_id = transmit.peer_id();
                    let request_id = transmit.request_id();
                    match replied_peer_ids_and_request_ids
                        .iter()
                        .position(|candidate| *candidate == (peer_id, request_id))
                    {
                        Some(position) => {
                            ordered_transmits.insert(position, transmit);
                        },
                        None => {
                            new_buffered_transmits.push_back(transmit);
                        },
                    }
                }

                let mut non_existing_replied_peer_and_request_ids =
                    replied_peer_ids_and_request_ids.iter().copied().collect::<BTreeSet<_>>();

                for transmit in ordered_transmits.into_values() {
                    non_existing_replied_peer_and_request_ids
                        .remove(&(transmit.peer_id(), transmit.request_id()));
                }

                if !non_existing_replied_peer_and_request_ids.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Cannot drop replies {:?} of {} as they don't exist",
                        non_existing_replied_peer_and_request_ids,
                        peer_id,
                    ));
                }

                let peer = self.peer_mut(peer_id);
                *peer.buffered_peer_transmits_mut() = new_buffered_transmits;
            },

            Action::TimeoutHeartbeat { peer_id } => {
                let peer = self.peer_mut(peer_id);
                peer.trigger_heartbeat_timeout();
            },
            Action::ApplyCommitted { peer_id } => {
                if let Some(peer_id) = peer_id {
                    let peer = self.peer_mut(peer_id);
                    peer.apply_committed();
                } else {
                    for peer in self.peers.iter_mut() {
                        peer.apply_committed();
                    }
                }
            },

            Action::SendCommand { client_id, peer_id, command } => {
                let client = &mut self.clients[client_id.0 - 1];
                if let Err(error) = client.command(command.clone(), peer_id) {
                    return Err(anyhow::anyhow!(
                        "Cannot send `{:?}` command from client {}{}: {}",
                        command,
                        client_id,
                        if let Some(peer_id) = peer_id {
                            format!(" to peer {peer_id}")
                        } else {
                            String::new()
                        },
                        error,
                    ));
                }
            },
            Action::SendQuery { client_id, peer_id, query } => {
                let client = &mut self.clients[client_id.0 - 1];
                if let Err(error) = client.query(query.clone(), peer_id) {
                    return Err(anyhow::anyhow!(
                        "Cannot send `{:?}` query from client {}{}: {}",
                        query,
                        client_id,
                        if let Some(peer_id) = peer_id {
                            format!(" to peer {peer_id}")
                        } else {
                            String::new()
                        },
                        error,
                    ));
                }
            },

            Action::TransmitClientRequest { client_id, request_id } => {
                let client = self.client_mut(client_id);
                let buffered_transmits = client.buffered_client_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_request() && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(transmit) => {
                                let target_peer = self.peer_mut(transmit.peer_id());
                                target_peer.receive_client_message(
                                    client_id,
                                    transmit.request_id(),
                                    transmit.into_message(),
                                );
                            },
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot transmit {} of client {} as it doesn't exist",
                            request_id,
                            client_id,
                        ));
                    },
                }
            },
            Action::TransmitClientReply { peer_id, replied_client_id_and_request_id } => {
                let (replied_client_id, request_id) = replied_client_id_and_request_id;

                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_client_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_reply()
                        && transmit.client_id() == replied_client_id
                        && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(transmit) => {
                                let target_client = &mut self.clients[replied_client_id.0 - 1];
                                target_client.receive_reply(
                                    peer_id,
                                    transmit.request_id(),
                                    transmit.into_message(),
                                );
                            },
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot transmit the reply of #{} of client {} \
                                from peer {} as it doesn't exist",
                            request_id,
                            replied_client_id,
                            peer_id,
                        ));
                    },
                }
            },
            Action::DropClientReply { peer_id, replied_client_id_and_request_id } => {
                let (replied_client_id, request_id) = replied_client_id_and_request_id;

                let peer = self.peer_mut(peer_id);
                let buffered_transmits = peer.buffered_client_transmits_mut();

                match buffered_transmits.iter().position(|transmit| {
                    transmit.message().is_reply()
                        && transmit.client_id() == replied_client_id
                        && transmit.request_id() == request_id
                }) {
                    Some(position) => {
                        match buffered_transmits.remove(position) {
                            Some(_) => {},
                            None => unreachable!(),
                        }
                    },
                    None => {
                        return Err(anyhow::anyhow!(
                            "Cannot drop the reply of #{} of client {} \
                                from peer {} as it doesn't exist",
                            request_id,
                            replied_client_id,
                            peer_id,
                        ));
                    },
                }
            },

            Action::Check { updates } => {
                if self.replay_peers.is_empty() {
                    return Err(anyhow::anyhow!("Checks are not enabled"));
                }
                for update in updates {
                    update.apply_to(&mut self.replay_peers)?;
                }
                for peer_id in 1..=self.peers.len() {
                    self.check(PeerId(peer_id))?;
                }
            },
        }
        Ok(())
    }
}

impl<A: RaftApplication> Simulation<A> {
    fn check(&mut self, peer_id: PeerId) -> anyhow::Result<()> {
        let actual = &mut self.peers[peer_id.0 - 1];
        let expected = &mut self.replay_peers[peer_id.0 - 1];

        fn check_equality<T: Eq + Debug>(
            property: &str,
            peer_id: PeerId,
            expected: T,
            actual: T,
        ) -> anyhow::Result<()> {
            if actual != expected {
                let expected_title = format!("Expected {property} of Peer {peer_id}");
                let actual_title = format!("Actual {property} of Peer {peer_id}");
                return Err(anyhow::anyhow!(
                    "\n{}\n{}\n{:#?}\n\n{}\n{}\n{:#?}\n",
                    expected_title,
                    "-".repeat(expected_title.len()),
                    expected,
                    actual_title,
                    "-".repeat(actual_title.len()),
                    actual,
                ));
            }
            Ok(())
        }

        let expected_current_term = expected.current_term();
        let actual_current_term = actual.current_term();
        check_equality("Current Term", peer_id, expected_current_term, actual_current_term)?;

        let expected_voted_for = expected.voted_for();
        let actual_voted_for = actual.voted_for();
        check_equality("Voted For", peer_id, expected_voted_for, actual_voted_for)?;

        let expected_log = expected.log();
        let actual_log = actual.log();
        check_equality("Log", peer_id, expected_log, actual_log)?;

        let expected_snapshot = expected.snapshot();
        let actual_snapshot = actual.snapshot();
        check_equality("Snapshot", peer_id, expected_snapshot, actual_snapshot)?;

        let expected_commit_index = expected.commit_index();
        let actual_commit_index = actual.commit_index();
        check_equality("Commit Index", peer_id, expected_commit_index, actual_commit_index)?;

        let expected_last_applied = expected.last_applied();
        let actual_last_applied = actual.last_applied();
        check_equality("Last Applied", peer_id, expected_last_applied, actual_last_applied)?;

        let expected_role = expected.role();
        let actual_role = actual.role();
        check_equality("Role", peer_id, expected_role, actual_role)?;

        let expected_machine = expected.machine();
        let actual_machine = actual.machine();
        check_equality("Machine", peer_id, expected_machine, actual_machine)?;

        let expected_buffered_peer_transmits = expected.buffered_peer_transmits();
        let actual_buffered_peer_transmits = actual.buffered_peer_transmits();
        check_equality(
            "Buffered Peer Transmits",
            peer_id,
            expected_buffered_peer_transmits,
            actual_buffered_peer_transmits,
        )?;

        Ok(())
    }
}
