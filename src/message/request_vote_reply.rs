use crate::prelude::*;

/// Vote outcome of a [RequestVoteReply].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Vote {
    /// Vote is not granted due to requested peer being in a higher [Term].
    NotGrantedDueToBeingInHigherTerm,
    /// Vote is not granted due to requested peers [Log] being more up to date.
    NotGrantedDueToBeingLessUpToDate,
    /// Vote is not granted due to being granted to another [Peer] within the [Term].
    NotGrantedDueToBeingGrantedToAnotherPeer,
    /// Vote is not granted due to a storage error.
    NotGrantedDueToStorageError,
    /// Vote is granted.
    Granted,
}

/// Reply to a [RequestVoteRequest].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct RequestVoteReply {
    #[builder(into)]
    term: Term,

    #[builder(into)]
    vote: Vote,
}

impl RequestVoteReply {
    pub(crate) fn set_term(&mut self, term: Term) {
        self.term = term;
    }

    pub(crate) fn set_vote(&mut self, vote: Vote) {
        self.vote = vote;
    }
}

impl RequestVoteReply {
    pub(crate) fn receive<A: Application>(
        self,
        sending_peer_id: PeerId,
        request_id: RequestId,
        receiving_peer: &mut Peer<A>,
    ) {
        let receiving_peer_id = receiving_peer.id;
        if self.term > receiving_peer.current_term() {
            log::info!(
                "({}) Peer {} didn't grant vote as it's in a higher term.",
                receiving_peer_id,
                sending_peer_id,
            );

            log::info!(
                "({}) Updating current term to peers term {} and clearing voted for.",
                receiving_peer_id,
                self.term,
            );
            if let Err(error) =
                receiving_peer.storage.set_current_term_and_voted_for(self.term, None)
            {
                log::error!(
                    "({}) Failed to persistently update \
                            current term to {} and clear voted for ({}).",
                    receiving_peer_id,
                    self.term,
                    error,
                );
            };

            log::info!("({}) Stepping down to become a follower.", receiving_peer_id);
            receiving_peer.role = Role::Follower(FollowerState::default());

            receiving_peer.buffered_peer_transmits.retain(|transmit| {
                !matches!(transmit.message(), PeerMessage::RequestVoteRequest(..))
            });
            return;
        }

        let current_term = receiving_peer.current_term();
        let candidate_state = match &mut receiving_peer.role {
            Role::Follower(_) => {
                log::info!(
                    "({}) Peer {} replied to an old vote request but it doesn't matter \
                    as election is over.",
                    receiving_peer.id,
                    sending_peer_id,
                );
                return;
            },
            Role::Candidate(candidate_state) => candidate_state,
            Role::Leader(_) => {
                log::info!(
                    "({}) Peer {} replied to the vote request but it doesn't matter \
                    as majority of the cluster already granted vote.",
                    receiving_peer.id,
                    sending_peer_id,
                );
                return;
            },
        };

        if !candidate_state.vote_request_ids().contains(&request_id) {
            log::info!(
                "({}) Peer {} replied to an old vote request, which will be ignored.",
                receiving_peer.id,
                sending_peer_id,
            );
            return;
        }

        match self.vote {
            Vote::Granted => {
                assert_eq!(self.term, current_term);

                log::info!("({}) Peer {} granted vote.", receiving_peer_id, sending_peer_id);
                candidate_state.grant_vote(request_id);
                log::info!(
                    "({}) {} votes are granted.",
                    receiving_peer_id,
                    candidate_state.votes_granted(),
                );

                if candidate_state.votes_granted() >= receiving_peer.majority() {
                    receiving_peer.become_leader();
                }
            },
            Vote::NotGrantedDueToStorageError => {
                log::warn!(
                    "({}) Peer {} didn't grant vote due to a persistence failure.",
                    receiving_peer_id,
                    sending_peer_id,
                );
                log::info!(
                    "({}) Requesting vote from peer {} again.",
                    receiving_peer_id,
                    sending_peer_id,
                );

                let request = RequestVoteRequest::builder()
                    .term(current_term)
                    .candidate_id(receiving_peer_id)
                    .last_log_index(
                        receiving_peer
                            .log()
                            .last()
                            .map(|entry| entry.index())
                            .unwrap_or(receiving_peer.snapshot().last_included_index()),
                    )
                    .last_log_term(
                        receiving_peer
                            .log()
                            .last()
                            .map(|entry| entry.term())
                            .unwrap_or(receiving_peer.snapshot().last_included_term()),
                    )
                    .build();
                let transmit = PeerTransmit::builder()
                    .peer_id(sending_peer_id)
                    .request_id(request_id)
                    .message(request)
                    .build();
                receiving_peer.buffered_peer_transmits.push_back(transmit);
            },
            Vote::NotGrantedDueToBeingInHigherTerm => {},
            Vote::NotGrantedDueToBeingLessUpToDate => {
                log::info!(
                    "({}) Peer {} didn't grant vote as its log is more up to date.",
                    receiving_peer_id,
                    sending_peer_id,
                );
            },
            Vote::NotGrantedDueToBeingGrantedToAnotherPeer => {
                log::info!(
                    "({}) Peer {} didn't grant vote as it voted for another peer already.",
                    receiving_peer_id,
                    sending_peer_id,
                );
            },
        }
    }
}
