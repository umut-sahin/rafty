use crate::prelude::*;

/// Request from a [Client] to a [Peer] to apply a [Command] to the replicated [Machine].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct CommandRequest<A: Application> {
    command: A::Command,
}

impl<A: Application> CommandRequest<A> {
    pub(crate) fn receive(
        self,
        sending_client_id: ClientId,
        request_id: RequestId,
        receiving_peer: &mut Peer<A>,
    ) -> Option<CommandReply<A>> {
        log::info!(
            "({}) Client {} commands `{:?}` in its request {}.",
            receiving_peer.id,
            sending_client_id,
            self.command,
            request_id,
        );

        let leader_state = match &mut receiving_peer.role {
            Role::Leader(leader_state) => {
                log::info!("({}) Processing the command as the leader.", receiving_peer.id);
                leader_state
            },
            Role::Candidate(_) => {
                log::info!(
                    "({}) Not running the command as a candidate during the election for term {} \
                    and letting the client know.",
                    receiving_peer.id,
                    receiving_peer.current_term(),
                );
                return Some(
                    CommandReply::builder().result(Err(ClientError::LeaderUnknown)).build(),
                );
            },
            Role::Follower(follower_state) => {
                return Some(match follower_state.leader_id {
                    Some(leader_id) => {
                        log::info!(
                            "({}) Not running the command as a follower of peer {} \
                            and letting the user know.",
                            receiving_peer.id,
                            leader_id,
                        );
                        CommandReply::builder()
                            .result(Err(ClientError::LeaderChanged { new_leader_id: leader_id }))
                            .build()
                    },
                    None => {
                        log::info!(
                            "({}) Not running the command as a follower without a leader \
                            and letting the user know.",
                            receiving_peer.id,
                        );
                        CommandReply::builder().result(Err(ClientError::LeaderUnknown)).build()
                    },
                });
            },
        };

        let (prev_log_index, prev_log_term) = receiving_peer
            .storage
            .log()
            .last()
            .map(|entry| (entry.index(), entry.term()))
            .unwrap_or((
                receiving_peer.storage.snapshot().last_included_index(),
                receiving_peer.storage.snapshot().last_included_term(),
            ));

        let log_entry = LogEntry::builder()
            .index(prev_log_index.next())
            .term(receiving_peer.storage.current_term())
            .command(self.command)
            .build();

        log::info!(
            "({}) Appending `{:?}` as the leader and instructing the peers to do the same.",
            receiving_peer.id,
            log_entry,
        );
        if let Err(error) = receiving_peer.storage.append_log_entry(log_entry.clone()) {
            log::error!(
                "({}) Failed to persistently append the log entry ({}).",
                receiving_peer.id,
                error,
            );
            log::info!("({}) Letting the user know about the failure.", receiving_peer.id);
            return Some(
                CommandReply::builder()
                    .result(Err(ClientError::StorageError { underlying_error: error }))
                    .build(),
            );
        }

        for peer_id in receiving_peer.cluster.iter() {
            if *peer_id == receiving_peer.id {
                continue;
            }
            let request = AppendEntriesRequest {
                term: receiving_peer.storage.current_term(),
                leader_id: receiving_peer.id,
                prev_log_index,
                prev_log_term,
                entries: vec![log_entry.clone()],
                leader_commit: receiving_peer.commit_index,
            };

            let request_id = receiving_peer.request_counter.next();
            let transmit = PeerTransmit::builder()
                .peer_id(*peer_id)
                .request_id(request_id)
                .message(request.clone())
                .build();

            leader_state.append_entries_requests.insert(transmit.request_id(), request);
            receiving_peer.buffered_peer_transmits.push_back(transmit);
        }

        None
    }
}
