//! Client definitions.

use crate::prelude::*;

/// Client to issue [Command]s and [Query]s to [Peer]s.
pub struct Client<A: Application> {
    pub(crate) id: ClientId,
    pub(crate) cluster: Cluster,

    pub(crate) leader: Option<PeerId>,

    pub(crate) rng: StdRng,
    pub(crate) request_counter: RequestCounter,

    pub(crate) commands: BTreeMap<RequestId, A::Command>,
    pub(crate) command_results: BTreeMap<RequestId, Result<A::CommandResult, ClientError<A>>>,

    pub(crate) queries: BTreeMap<RequestId, A::Query>,
    pub(crate) query_results: BTreeMap<RequestId, Result<A::QueryResult, ClientError<A>>>,

    pub(crate) buffered_client_transmits: VecDeque<ClientTransmit<A>>,
}

impl<A: Application> Client<A> {
    /// Creates a new client.
    pub fn new(id: ClientId, cluster: Cluster) -> Self {
        Self {
            id,
            cluster,
            leader: None,
            rng: StdRng::from_os_rng(),
            request_counter: RequestCounter::default(),
            commands: Default::default(),
            command_results: Default::default(),
            queries: Default::default(),
            query_results: Default::default(),
            buffered_client_transmits: Default::default(),
        }
    }
}

impl<A: Application> Client<A> {
    /// Gets the identifier of the client.
    pub fn id(&self) -> ClientId {
        self.id
    }

    /// Gets the buffered transmits of the client.
    pub fn buffered_client_transmits(&self) -> &VecDeque<ClientTransmit<A>> {
        &self.buffered_client_transmits
    }
}

impl<A: Application> Client<A> {
    /// Submits a command to the cluster.
    pub fn command(
        &mut self,
        command: A::Command,
        peer_id: Option<PeerId>,
    ) -> Result<RequestId, ClientError<A>> {
        let request_id = RequestId(self.request_counter.next());
        let peer_id = match peer_id {
            Some(peer_id) => {
                log::info!(
                    "|{}| Commanding `{:?}` in request {} via peer {}.",
                    self.id,
                    command,
                    request_id,
                    peer_id,
                );
                peer_id
            },
            None => {
                match self.leader {
                    Some(leader_id) => {
                        log::info!(
                            "|{}| Commanding `{:?}` in request {} \
                            via peer {} which is the current known leader.",
                            self.id,
                            command,
                            request_id,
                            leader_id,
                        );
                        leader_id
                    },
                    None => {
                        match self.cluster.iter().choose(&mut self.rng).copied() {
                            Some(random_peer_id) => {
                                log::info!(
                                    "|{}| Commanding `{:?}` in request {} \
                                    via the randomly selected peer {} as the leader is not known.",
                                    self.id,
                                    command,
                                    request_id,
                                    random_peer_id,
                                );
                                random_peer_id
                            },
                            None => {
                                return Err(ClientError::EmptyCluster);
                            },
                        }
                    },
                }
            },
        };
        self.commands.insert(request_id, command.clone());

        let request = CommandRequest::builder().command(command).build();
        let transmit = ClientTransmit::builder()
            .peer_id(peer_id)
            .client_id(self.id)
            .request_id(request_id)
            .message(request)
            .build();

        self.buffered_client_transmits.push_back(transmit);
        Ok(request_id)
    }

    /// Submits a query to the cluster.
    pub fn query(
        &mut self,
        query: A::Query,
        peer_id: Option<PeerId>,
    ) -> Result<RequestId, ClientError<A>> {
        let request_id = RequestId(self.request_counter.next());
        let peer_id = match peer_id {
            Some(peer_id) => {
                log::info!(
                    "|{}| Querying `{:?}` in request {} via peer {}.",
                    self.id,
                    query,
                    request_id,
                    peer_id,
                );
                peer_id
            },
            None => {
                match self.leader {
                    Some(leader_id) => {
                        log::info!(
                            "|{}| Querying `{:?}` in request {} \
                            via peer {} which is the current known leader.",
                            self.id,
                            query,
                            request_id,
                            leader_id,
                        );
                        leader_id
                    },
                    None => {
                        match self.cluster.iter().choose(&mut self.rng).copied() {
                            Some(random_peer_id) => {
                                log::info!(
                                    "|{}| Querying `{:?}` in request {} \
                                    via the randomly selected peer {} as the leader is not known.",
                                    self.id,
                                    query,
                                    request_id,
                                    random_peer_id,
                                );
                                random_peer_id
                            },
                            None => {
                                return Err(ClientError::EmptyCluster);
                            },
                        }
                    },
                }
            },
        };
        self.queries.insert(request_id, query.clone());

        let request = QueryRequest::builder().query(query).build();
        let transmit = ClientTransmit::builder()
            .peer_id(peer_id)
            .client_id(self.id)
            .request_id(request_id)
            .message(request)
            .build();

        self.buffered_client_transmits.push_back(transmit);
        Ok(request_id)
    }

    pub fn receive_reply(
        &mut self,
        peer_id: PeerId,
        request_id: RequestId,
        message: ClientMessage<A>,
    ) {
        match message {
            ClientMessage::CommandRequest(_) | ClientMessage::QueryRequest(_) => {
                log::warn!(
                    "|{}| Peer {} sent a request to the client which shouldn't have happened.",
                    self.id,
                    peer_id,
                );
            },

            ClientMessage::CommandReply(reply) => {
                reply.receive(peer_id, request_id, self);
            },
            ClientMessage::QueryReply(reply) => {
                reply.receive(peer_id, request_id, self);
            },
        }
    }
}

#[cfg(feature = "direct-control")]
impl<A: Application> Client<A> {
    /// Gets the buffered transmits of the client mutably.
    pub fn buffered_client_transmits_mut(&mut self) -> &mut VecDeque<ClientTransmit<A>> {
        &mut self.buffered_client_transmits
    }

    /// Overwrites the buffered transmits of the client.
    ///
    /// Should only be used for testing purposes!
    pub fn set_buffered_client_transmits(
        &mut self,
        new_buffered_client_transmits: VecDeque<ClientTransmit<A>>,
    ) {
        self.buffered_client_transmits = new_buffered_client_transmits;
    }
}
