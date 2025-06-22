//! Application interface.

use crate::prelude::*;

/// Application to make distributed.
pub trait Application: Clone + Debug + Eq + PartialEq + Send + Sync + 'static {
    /// Machine to replicate across [Peer]s.
    type Machine: Machine<Self>;

    /// Commands to be issued by [Client]s.
    type Command: Command;
    /// Results of the commands issued by [Client]s.
    type CommandResult: CommandResult;

    /// Queries to be issued by [Client]s.
    type Query: Query;
    /// Results of the queries issued by [Client]s.
    type QueryResult: QueryResult;

    /// Storage to save and load the information that needs to persist in [Peer]s.
    type Storage: Storage<Self>;
    /// Errors that can happen during storage operations.
    type StorageError: Error
        + Clone
        + Eq
        + PartialEq
        + Serialize
        + DeserializeOwned
        + Send
        + Sync
        + 'static;
}
