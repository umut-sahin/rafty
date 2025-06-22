use crate::prelude::*;

/// Reply to a [AppendEntriesRequest].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct AppendEntriesReply {
    #[builder(into)]
    pub(crate) term: Term,

    #[builder(into)]
    pub(crate) success: bool,
}

impl AppendEntriesReply {
    pub(crate) fn receive<A: Application>(
        self,
        sending_peer_id: PeerId,
        request_id: RequestId,
        receiving_peer: &mut Peer<A>,
    ) {
        let majority = receiving_peer.majority();

        let current_term = receiving_peer.storage.current_term();
        if self.term > current_term {
            log::info!(
                "({}) Peer {} is in term {} which means the current term is over.",
                receiving_peer.id,
                sending_peer_id,
                self.term,
            );

            log::info!(
                "({}) Updating current term to peers term {} and clearing voted for.",
                receiving_peer.id,
                self.term,
            );
            if let Err(error) =
                receiving_peer.storage.set_current_term_and_voted_for(self.term, None)
            {
                log::error!(
                    "({}) Failed to persistently update current term to {} and clear voted for ({}).",
                    receiving_peer.id,
                    self.term,
                    error,
                );
            }

            if let Role::Leader(_leader_state) = &mut receiving_peer.role {
                log::info!("({}) Redirecting awaiting client requests.", receiving_peer.id);
                // TODO: remember client requests, redirect them to the new leader
                log::info!("({}) Stepping down to become a follower.", receiving_peer.id);
                receiving_peer.role =
                    Role::Follower(FollowerState::builder().leader_id(None).build());
            }

            return;
        }

        if self.term < current_term {
            log::info!(
                "({}) Peer {} replied to an old append entries request from term {}, ignoring.",
                receiving_peer.id,
                sending_peer_id,
                self.term,
            );
            return;
        }

        if let Role::Leader(leader_state) = &mut receiving_peer.role {
            let Some(request) = leader_state.append_entries_requests.remove(&request_id) else {
                return;
            };

            if self.success {
                if let Some(next_log_index) = leader_state.next_index.get_mut(&sending_peer_id) {
                    let new_log_index = request
                        .entries
                        .last()
                        .map(|entry| entry.index())
                        .unwrap_or(request.prev_log_index)
                        .next();
                    if *next_log_index != new_log_index {
                        log::info!(
                            "({}) Peer {} appended entries up to log index {}.",
                            receiving_peer.id,
                            sending_peer_id,
                            next_log_index,
                        );
                        *next_log_index = new_log_index;
                    }
                }
                if let Some(match_index) = leader_state.match_index.get_mut(&sending_peer_id) {
                    let replicated_log_index = request
                        .entries
                        .last()
                        .map(|entry| entry.index())
                        .unwrap_or(request.prev_log_index);
                    if *match_index != replicated_log_index {
                        log::info!(
                            "({}) Peer {} replicated entries up to log index {}.",
                            receiving_peer.id,
                            sending_peer_id,
                            match_index,
                        );
                        *match_index = replicated_log_index;
                    }
                }

                let mut replication_counts: BTreeMap<LogIndex, usize> = BTreeMap::new();
                for replicated_log_index in leader_state.match_index.values() {
                    *replication_counts.entry(*replicated_log_index).or_insert(0) += 1;
                }
                let mut accumulated_replication_count = 0;

                let mut new_commit_index = receiving_peer.commit_index;
                'search: for (log_index, replication_count) in replication_counts.into_iter().rev()
                {
                    if log_index <= new_commit_index {
                        break 'search;
                    }

                    accumulated_replication_count += replication_count;
                    if accumulated_replication_count >= majority {
                        log::info!(
                            "({}) Majority of the peers appended up to log index {} \
                                so committing log entries from index {} to index {}.",
                            receiving_peer.id,
                            log_index,
                            new_commit_index,
                            log_index,
                        );
                        new_commit_index = log_index;
                        break 'search;
                    }
                }
                receiving_peer.commit_index = new_commit_index;

                return;
            }

            if let Some(next_index) = leader_state.next_index.get_mut(&sending_peer_id) {
                if *next_index > receiving_peer.storage.snapshot().last_included_index().next() {
                    *next_index = next_index.previous();
                } else {
                    // TODO: snapshots
                    unimplemented!()
                }

                let log = receiving_peer.storage.log();
                let next_index_position =
                    match log.binary_search_by(|entry| entry.index().cmp(next_index)) {
                        Ok(position) => position,
                        Err(_) => {
                            // TODO: needs snapshot
                            unimplemented!()
                        },
                    };

                let next_term = if *next_index == LogIndex(0) {
                    Term(0)
                } else {
                    log[next_index_position].term()
                };

                let request = AppendEntriesRequest::builder()
                    .term(receiving_peer.storage.current_term())
                    .leader_id(receiving_peer.id)
                    .prev_log_index(*next_index)
                    .prev_log_term(next_term)
                    .entries(log[next_index_position..].to_vec())
                    .leader_commit(receiving_peer.commit_index)
                    .build();

                let request_id = receiving_peer.request_counter.next();
                let transmit = PeerTransmit::builder()
                    .peer_id(sending_peer_id)
                    .request_id(request_id)
                    .message(request.clone())
                    .build();

                leader_state.append_entries_requests.insert(transmit.request_id(), request);
                receiving_peer.buffered_peer_transmits.push_back(transmit);
            }
        }
    }
}
