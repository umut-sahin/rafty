//! Role definitions.

use crate::prelude::*;

/// Role of a [Peer].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role<A: Application> {
    Follower(FollowerState),
    Candidate(CandidateState),
    Leader(LeaderState<A>),
}

impl<A: Application> Role<A> {
    /// Gets whether the role is a follower.
    pub fn is_follower(&self) -> bool {
        matches!(self, Role::Follower(_))
    }

    /// Gets whether the role is a candidate.
    pub fn is_candidate(&self) -> bool {
        matches!(self, Role::Candidate(_))
    }

    /// Gets whether the role is a leader.
    pub fn is_leader(&self) -> bool {
        matches!(self, Role::Leader(_))
    }
}

impl<A: Application> Default for Role<A> {
    fn default() -> Self {
        Role::Follower(FollowerState::default())
    }
}

/// State of a follower.
#[derive(Clone, Debug, Default, Eq, PartialEq, bon::Builder)]
pub struct FollowerState {
    #[builder(required, into)]
    pub(crate) leader_id: Option<PeerId>,
}

impl FollowerState {
    /// Gets the leader id of the follower.
    pub fn leader_id(&self) -> Option<PeerId> {
        self.leader_id
    }
}

/// State of a candidate.
#[derive(Clone, Debug, Eq, PartialEq, bon::Builder)]
pub struct CandidateState {
    #[builder(with = FromIterator::from_iter)]
    pub(crate) vote_request_ids: BTreeSet<RequestId>,

    pub(crate) votes_granted: usize,
}

impl CandidateState {
    /// Gets vote request ids for this term.
    pub fn vote_request_ids(&self) -> &BTreeSet<RequestId> {
        &self.vote_request_ids
    }

    /// Gets the number of votes granted for this term.
    pub fn votes_granted(&self) -> usize {
        self.votes_granted
    }
}

impl CandidateState {
    pub(crate) fn grant_vote(&mut self, request_id: RequestId) {
        if self.vote_request_ids.remove(&request_id) {
            self.votes_granted += 1;
        }
    }
}

/// State of a leader.
#[derive(Clone, Debug, Eq, PartialEq, bon::Builder)]
pub struct LeaderState<A: Application> {
    #[builder(with = FromIterator::from_iter)]
    pub(crate) next_index: BTreeMap<PeerId, LogIndex>,

    #[builder(with = FromIterator::from_iter)]
    pub(crate) match_index: BTreeMap<PeerId, LogIndex>,

    #[builder(with = FromIterator::from_iter, default)]
    pub(crate) append_entries_requests: BTreeMap<RequestId, AppendEntriesRequest<A>>,
}

impl<A: Application> LeaderState<A> {
    /// Gets the next index of peers.
    pub fn next_index(&self) -> &BTreeMap<PeerId, LogIndex> {
        &self.next_index
    }

    /// Gets the match index of peers.
    pub fn match_index(&self) -> &BTreeMap<PeerId, LogIndex> {
        &self.match_index
    }
}
