use crate::prelude::*;

/// Errors that can be returned to [Client]s.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    derive_more::Display,
    derive_more::Error
)]
pub enum ClientError<A: Application> {
    #[display("Cluster is empty")]
    EmptyCluster,
    #[display("Leader is not known by the peer")]
    LeaderUnknown,
    #[display("Leader changed to peer {new_leader_id}")]
    LeaderChanged { new_leader_id: PeerId },
    #[display("Storage error: {underlying_error}")]
    StorageError { underlying_error: A::StorageError },
}
