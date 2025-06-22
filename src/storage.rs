//! Storage interface.

use crate::prelude::*;

/// Storage of the [Application] to store the persistent data of the [Peer]s.
///
/// Storage implementation has the flexibility to use any representation it wants.
///
/// For a basic file based storage, it can be defined as:
/// ```
/// # use std::fs::File;
/// # use rafty::prelude::{RaftApplication, PeerId, Term, Log, Snapshot};
/// pub struct FileStorage<A: RaftApplication> {
///     /// A file on disk which contains a JSON object
///     /// with `current_term` and `voted_for` properties.
///     state_file: File,
///     /// Current term stored in the `state_file`.
///     current_term: Term,
///     /// Voted for stored in the `state_file`.
///     log_file: File,
///
///     /// A file in which each line corresponds to a JSON object of a [LogEntry].
///     voted_for: PeerId,
///     /// Log stored in `log_file`.
///     log: Log<A>,
///
///     /// A file on disk which contains a JSON object of a [Snapshot].
///     snapshot_file: PeerId,
///     /// Snapshot stored in `snapshot_file`.
///     snapshot: Snapshot<A>,
/// }
/// ```
pub trait Storage<A: Application>: Send + Sync + 'static {
    /// Errors that can happen during persistent updates.
    type Error: Error
        + Clone
        + Eq
        + PartialEq
        + Serialize
        + DeserializeOwned
        + Send
        + Sync
        + 'static;

    /// Gets the persistent current term.
    fn current_term(&self) -> Term;
    /// Sets the current term persistently.
    fn set_current_term(&mut self, term: Term) -> Result<(), A::StorageError>;

    /// Gets the persistent voted for.
    fn voted_for(&self) -> Option<PeerId>;
    /// Sets the voted for persistently.
    fn set_voted_for(&mut self, voted_for: Option<PeerId>) -> Result<(), A::StorageError>;

    /// Sets the current term and voted for persistently.
    fn set_current_term_and_voted_for(
        &mut self,
        current_term: Term,
        voted_for: Option<PeerId>,
    ) -> Result<(), A::StorageError>;

    /// Gets the persistent log.
    fn log(&self) -> &Log<A>;
    /// Append an entry to the log persistently.
    fn append_log_entry(&mut self, entry: LogEntry<A>) -> Result<(), A::StorageError>;
    /// Truncate the log down to a certain log index persistently.
    fn truncate_log(&mut self, down_to: LogIndex) -> Result<(), A::StorageError>;

    /// Gets the current persistent snapshot.
    fn snapshot(&self) -> &Snapshot<A>;
    /// Installs a new snapshot persistently.
    fn install_snapshot(&mut self, snapshot: Snapshot<A>) -> Result<(), A::StorageError>;
}
