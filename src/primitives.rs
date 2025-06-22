//! Primitives of the crate.

use crate::prelude::*;

/// Identifier of a [Client].
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Display,
    derive_more::From
)]
#[debug("ClientId({_0})")]
#[display("{_0}")]
pub struct ClientId(#[from] pub usize);

/// Cluster of [PeerId]s.
#[repr(transparent)]
#[derive(
    Clone,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Deref,
    derive_more::From
)]
#[debug("Cluster({_0:?})")]
pub struct Cluster(
    #[from]
    #[deref]
    BTreeSet<PeerId>,
);

/// Consistency requirement of [Peer]s.
#[derive(Clone, Copy)]
pub enum Consistency {
    /// Strong consistency.
    ///
    /// In this mode, followers and candidates can't respond to client queries.
    /// Instead, they redirect the clients to the leader and let the leader reply.
    ///
    /// Leader must ensure it's still the leader before replying to clients
    /// which involves waiting for the majority of replies to upcoming heartbeats.
    ///
    /// It also makes sure all committed entries are applied before responding.
    Strong,

    /// Eventual consistency.
    ///
    /// In this mode, queries can be processed by all peers, which may
    /// lead to stale data being read. As the cluster operates normally,
    /// state machines will get in sync over time and queries will return
    /// more up-to-date data.
    ///
    /// Eventual consistency is enough for many applications, so it's also available.
    Eventual,
}

/// Index of a [LogEntry].
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Display,
    derive_more::From
)]
#[debug("LogIndex({_0})")]
#[display("{_0}")]
pub struct LogIndex(#[from] pub usize);

impl LogIndex {
    /// Gets the previous log index.
    pub fn previous(self) -> Self {
        Self(self.0 - 1)
    }

    /// Gets the next log index.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Identifier of a [Peer].
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Display,
    derive_more::From
)]
#[debug("PeerId({_0})")]
#[display("{_0}")]
pub struct PeerId(#[from] pub usize);

/// Counter for requests of [Peer]s and [Client]s.
#[derive(Debug, Default)]
pub struct RequestCounter {
    next_request_id: AtomicUsize,
}

impl RequestCounter {
    /// Gets the next request id.
    pub fn next(&self) -> usize {
        self.next_request_id.fetch_add(1, AtomicOrdering::Relaxed)
    }
}

/// Identifier of a request.
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Display,
    derive_more::From
)]
#[debug("RequestId({_0})")]
#[display("{_0}")]
pub struct RequestId(#[from] pub usize);

/// Term numbers.
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    derive_more::Debug,
    derive_more::Display,
    derive_more::From
)]
#[debug("Term({_0})")]
#[display("{_0}")]
pub struct Term(#[from] pub usize);

impl Term {
    /// Gets the next term number.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
