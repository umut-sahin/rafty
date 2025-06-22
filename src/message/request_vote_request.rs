use crate::prelude::*;

/// Request from the candidates to other [Peer]s to request their vote for a [Term].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct RequestVoteRequest {
    #[builder(into)]
    term: Term,

    #[builder(into)]
    candidate_id: PeerId,

    #[builder(into)]
    last_log_index: LogIndex,

    #[builder(into)]
    last_log_term: Term,
}

impl RequestVoteRequest {
    pub(crate) fn receive<A: Application>(
        self,
        sending_peer_id: PeerId,
        receiving_peer: &mut Peer<A>,
    ) -> RequestVoteReply {
        log::info!(
            "({}) Peer {} requests vote for term {}.",
            receiving_peer.id,
            sending_peer_id,
            self.term,
        );

        let current_term = receiving_peer.current_term();
        let mut reply = RequestVoteReply::builder().term(current_term).vote(Vote::Granted).build();

        if self.term < current_term {
            log::info!(
                "({}) Peer {} denied vote as current term {} is higher than the voted term.",
                receiving_peer.id,
                sending_peer_id,
                current_term,
            );
            reply.set_vote(Vote::NotGrantedDueToBeingInHigherTerm);
            return reply;
        }

        #[allow(clippy::collapsible_if)]
        if self.term == current_term {
            if let Some(voted_peer_id) = receiving_peer.voted_for() {
                if voted_peer_id != sending_peer_id {
                    log::info!(
                        "({}) Not granting vote to peer {} because \
                            the vote for this term has already been granted to peer {}.",
                        receiving_peer.id,
                        sending_peer_id,
                        voted_peer_id,
                    );
                    reply.set_vote(Vote::NotGrantedDueToBeingGrantedToAnotherPeer);
                }
                return reply;
            }
        }

        if self.term > current_term {
            log::info!(
                "({}) Voted term {} is higher than the current term {}, \
                    trying to grant vote to peer {}.",
                receiving_peer.id,
                self.term,
                current_term,
                sending_peer_id,
            );

            if let Err(error) =
                receiving_peer.storage.set_current_term_and_voted_for(self.term, None)
            {
                log::error!(
                    "({}) Failed to persistently set current term and clear voted for {} ({}).",
                    receiving_peer.id,
                    sending_peer_id,
                    error
                );
                log::info!(
                    "({}) Not granting vote to peer {} due to a persistence failure.",
                    receiving_peer.id,
                    sending_peer_id,
                );
                reply.set_vote(Vote::NotGrantedDueToStorageError);
                return reply;
            }

            reply.set_term(self.term);
        }

        if let Some(voted_peer_id) = receiving_peer.voted_for() {
            if voted_peer_id != sending_peer_id {
                log::info!(
                    "({}) Not granting vote to peer {} because \
                            the vote for this term has already been granted to peer {}.",
                    receiving_peer.id,
                    sending_peer_id,
                    voted_peer_id,
                );
                reply.set_vote(Vote::NotGrantedDueToBeingGrantedToAnotherPeer);
            }
            return reply;
        }

        let (last_log_index, last_log_term) =
            receiving_peer.log().last().map(|entry| (entry.index(), entry.term())).unwrap_or((
                receiving_peer.snapshot().last_included_index(),
                receiving_peer.snapshot().last_included_term(),
            ));

        let is_at_least_as_up_to_date = self.last_log_term > last_log_term
            || (self.last_log_term == last_log_term && self.last_log_index >= last_log_index);

        if !is_at_least_as_up_to_date {
            log::info!(
                "({}) Not granting vote to peer {} because it's log is not as up to date.",
                receiving_peer.id,
                sending_peer_id,
            );
            reply.set_vote(Vote::NotGrantedDueToBeingLessUpToDate);
            return reply;
        }

        log::info!("({}) Updating voted for to peer {}.", receiving_peer.id, sending_peer_id);
        if let Err(error) = receiving_peer.storage.set_voted_for(Some(sending_peer_id)) {
            log::error!(
                "({}) Failed to persistently set voted for to peer {} ({}).",
                receiving_peer.id,
                sending_peer_id,
                error
            );
            log::info!(
                "({}) Not granting vote to peer {} due to a persistence failure.",
                receiving_peer.id,
                sending_peer_id,
            );
            reply.set_vote(Vote::NotGrantedDueToStorageError);
            return reply;
        }

        reply
    }
}
