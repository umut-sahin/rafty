use crate::prelude::*;

/// Request from a [Client] to a [Peer] to make a [Query] on the replicated [Machine].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
pub struct QueryRequest<A: Application> {
    query: A::Query,
}

impl<A: Application> QueryRequest<A> {
    pub(crate) fn receive(
        self,
        sending_client_id: ClientId,
        request_id: RequestId,
        receiving_peer: &mut Peer<A>,
    ) -> Option<QueryReply<A>> {
        log::info!(
            "({}) Client {} queries `{:?}` in its request {}.",
            receiving_peer.id,
            sending_client_id,
            self.query,
            request_id,
        );

        if let Consistency::Eventual = receiving_peer.consistency {
            if receiving_peer.last_applied < receiving_peer.commit_index {
                log::info!(
                    "({}) Applying committed entries before running the query.",
                    receiving_peer.id,
                );
                receiving_peer.apply_committed();
            }

            log::info!(
                "({}) Running the query as an eventually consistent peer \
                    and returning the result to the client.",
                receiving_peer.id,
            );

            let query_result = receiving_peer.machine.query(&self.query);
            return Some(QueryReply::builder().result(Ok(query_result)).build());
        }

        match &receiving_peer.role {
            Role::Leader(_) => {
                log::info!("({}) Processing the query as the leader.", receiving_peer.id);
            },
            Role::Candidate(_) => {
                log::info!(
                    "({}) Not running the query as a candidate during the election for term {} \
                    and letting the client know.",
                    receiving_peer.id,
                    receiving_peer.current_term(),
                );
                return Some(QueryReply::builder().result(Err(ClientError::LeaderUnknown)).build());
            },
            Role::Follower(follower_state) => {
                return Some(match follower_state.leader_id {
                    Some(leader_id) => {
                        log::info!(
                            "({}) Not running the query as a follower of peer {} \
                            and letting the user know.",
                            receiving_peer.id,
                            leader_id,
                        );
                        QueryReply::builder()
                            .result(Err(ClientError::LeaderChanged { new_leader_id: leader_id }))
                            .build()
                    },
                    None => {
                        log::info!(
                            "({}) Not running the query as a follower without a leader \
                            and letting the user know.",
                            receiving_peer.id,
                        );
                        QueryReply::builder().result(Err(ClientError::LeaderUnknown)).build()
                    },
                });
            },
        }

        // TODO

        None
    }
}
