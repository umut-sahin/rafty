//! Message definitions.

use crate::prelude::*;

mod append_entries_request;
mod command_request;
mod query_request;
mod request_vote_request;

mod append_entries_reply;
mod command_reply;
mod query_reply;
mod request_vote_reply;

pub use {
    append_entries_reply::AppendEntriesReply,
    append_entries_request::AppendEntriesRequest,
    command_reply::CommandReply,
    command_request::CommandRequest,
    query_reply::QueryReply,
    query_request::QueryRequest,
    request_vote_reply::{
        RequestVoteReply,
        Vote,
    },
    request_vote_request::RequestVoteRequest,
};

/// Message between a [Peer] and another [Peer].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, derive_more::From)]
pub enum PeerMessage<A: Application> {
    RequestVoteRequest(#[from] RequestVoteRequest),
    RequestVoteReply(#[from] RequestVoteReply),

    AppendEntriesRequest(#[from] AppendEntriesRequest<A>),
    AppendEntriesReply(#[from] AppendEntriesReply),
}

impl<A: Application> PeerMessage<A> {
    /// Gets whether the message is a request.
    pub fn is_request(&self) -> bool {
        matches!(self, PeerMessage::RequestVoteRequest(_) | PeerMessage::AppendEntriesRequest(_))
    }

    /// Gets whether the message is a reply.
    pub fn is_reply(&self) -> bool {
        matches!(self, PeerMessage::RequestVoteReply(_) | PeerMessage::AppendEntriesReply(_))
    }
}

/// Message between a [Peer] and a [Client].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, derive_more::From)]
pub enum ClientMessage<A: Application> {
    CommandRequest(#[from] CommandRequest<A>),
    CommandReply(#[from] CommandReply<A>),

    QueryRequest(#[from] QueryRequest<A>),
    QueryReply(#[from] QueryReply<A>),
}

impl<A: Application> ClientMessage<A> {
    /// Gets whether the message is a request.
    pub fn is_request(&self) -> bool {
        matches!(self, ClientMessage::CommandRequest(_) | ClientMessage::QueryRequest(_))
    }

    /// Gets whether the message is a reply.
    pub fn is_reply(&self) -> bool {
        matches!(self, ClientMessage::CommandReply(_) | ClientMessage::QueryReply(_))
    }
}
