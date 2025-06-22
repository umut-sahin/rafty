use crate::prelude::*;

/// Request from the leader to other [Peer]s to replicate log entries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct AppendEntriesRequest<A: Application> {
    #[builder(into)]
    pub(crate) term: Term,

    #[builder(into)]
    pub(crate) leader_id: PeerId,

    #[builder(into)]
    pub(crate) prev_log_index: LogIndex,

    #[builder(into)]
    pub(crate) prev_log_term: Term,

    #[builder(into)]
    pub(crate) entries: Vec<LogEntry<A>>,

    #[builder(into)]
    pub(crate) leader_commit: LogIndex,
}

impl<A: Application> AppendEntriesRequest<A> {
    pub(crate) fn receive(
        self,
        sending_peer_id: PeerId,
        receiving_peer: &mut Peer<A>,
    ) -> AppendEntriesReply {
        let current_term = receiving_peer.current_term();
        if self.term < current_term {
            log::info!(
                "({}) Peer {} wanted to append entries in term {} which is finished.",
                receiving_peer.id,
                sending_peer_id,
                self.term,
            );
            return AppendEntriesReply::builder().term(current_term).success(false).build();
        }

        if self.prev_log_index != LogIndex(0) {
            let log = receiving_peer.log();
            let prev_log_position = match log
                .binary_search_by(|entry| entry.index().cmp(&self.prev_log_index))
            {
                Ok(position) => position,
                Err(_) => {
                    return AppendEntriesReply::builder().term(current_term).success(false).build();
                },
            };

            let prev_log = &log[prev_log_position];
            if prev_log.term() != self.prev_log_term {
                receiving_peer.storage.truncate_log(prev_log.index()).expect("TODO");
                return AppendEntriesReply::builder().term(current_term).success(false).build();
            }
        }

        if self.term > current_term {
            log::info!(
                "({}) Entering term {} as a follower of peer {}.",
                receiving_peer.id,
                self.term,
                sending_peer_id,
            );
            receiving_peer.role =
                Role::Follower(FollowerState::builder().leader_id(sending_peer_id).build());

            receiving_peer.storage.set_current_term(self.term).expect("TODO");
        }

        match &mut receiving_peer.role {
            Role::Follower(follower_state) => {
                follower_state.leader_id = Some(sending_peer_id);
            },
            Role::Candidate(_) => {
                log::info!(
                    "({}) Peer {} is elected to be the leader of term {}, \
                        so stepping down from the election to become a follower.",
                    receiving_peer.id,
                    sending_peer_id,
                    current_term,
                );
                receiving_peer.role =
                    Role::Follower(FollowerState::builder().leader_id(sending_peer_id).build());
            },
            Role::Leader(_) => {
                unreachable!();
            },
        }

        for new_entry in self.entries {
            log::info!(
                "({}) Appending `{:?}` as instructed by the leader.",
                receiving_peer.id,
                new_entry
            );
            receiving_peer.storage.append_log_entry(new_entry.clone()).expect("TODO");
        }

        log::info!(
            "({}) Setting commit index from {} to leaders commit index {}",
            receiving_peer.id,
            receiving_peer.commit_index,
            self.leader_commit,
        );
        receiving_peer.commit_index = self.leader_commit;

        AppendEntriesReply::builder().term(current_term).success(true).build()
    }
}
