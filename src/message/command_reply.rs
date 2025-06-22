use crate::prelude::*;

/// Reply to a [CommandRequest].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct CommandReply<A: Application> {
    result: Result<A::CommandResult, ClientError<A>>,
}

impl<A: RaftApplication> CommandReply<A> {
    pub(crate) fn receive(
        self,
        sending_peer_id: PeerId,
        request_id: RequestId,
        receiving_client: &mut Client<A>,
    ) {
        match self.result {
            Ok(result) => {
                log::info!(
                    "|{}| Peer {} returned the result of request {}.",
                    receiving_client.id,
                    sending_peer_id,
                    request_id,
                );
                receiving_client.commands.remove(&request_id);
                receiving_client.command_results.insert(request_id, Ok(result));
            },
            Err(error) => {
                match &error {
                    ClientError::LeaderChanged { new_leader_id } => {
                        let command = match receiving_client.commands.get(&request_id) {
                            Some(command) => command,
                            None => {
                                log::info!(
                                    "|{}| Peer {} replied to request {}, \
                                    which is either unknown or already been replied.",
                                    receiving_client.id,
                                    sending_peer_id,
                                    request_id,
                                );
                                return;
                            },
                        };

                        log::info!(
                            "|{}| Peer {} says it's not the leader and the leader is peer {}.",
                            receiving_client.id,
                            sending_peer_id,
                            new_leader_id,
                        );

                        log::info!(
                            "|{}| Updating the leader to peer {} and \
                            commanding request {} again via the new leader.",
                            receiving_client.id,
                            new_leader_id,
                            request_id,
                        );
                        receiving_client.leader = Some(*new_leader_id);

                        let request = CommandRequest::builder().command(command.clone()).build();
                        let transmit = ClientTransmit::builder()
                            .peer_id(*new_leader_id)
                            .client_id(receiving_client.id)
                            .request_id(request_id)
                            .message(request)
                            .build();

                        receiving_client.buffered_client_transmits.push_back(transmit);
                    },
                    ClientError::LeaderUnknown => {
                        log::info!(
                            "|{}| Peer {} says it's not the leader and it doesn't know the leader.",
                            receiving_client.id,
                            sending_peer_id,
                        );
                        log::info!("|{}| Try commanding via another peer.", receiving_client.id);
                    },
                    ClientError::StorageError { underlying_error } => {
                        log::info!(
                            "|{}| Peer {} says it has encountered a storage error: {}.",
                            receiving_client.id,
                            sending_peer_id,
                            underlying_error,
                        );
                        log::info!("|{}| Please try again.", receiving_client.id);
                    },
                    ClientError::EmptyCluster => unreachable!(),
                }
            },
        }
    }
}
