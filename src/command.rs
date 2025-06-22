//! Command interface.

use crate::prelude::*;

/// Command of an [Application].
///
/// Commands are for [Client]s to modify the replicated [Machine]s via [Peer]s.
///
/// For a basic key-value database application, it could be defined as:
/// ```
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// enum Command {
///     /// Sets a key to a value in the database.
///     Set { key: String, value: String },
///     /// Removes a key from the database.
///     Clear { key: String },
///     /// Used internally for replication.
///     NoOp,
/// }
/// # impl rafty::prelude::RaftCommand for Command { fn no_op() -> Command { Command::NoOp } }
/// ```
pub trait Command:
    Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static
{
    /// Gets the no-op command.
    ///
    /// Leaders send [AppendEntriesRequest]s with a no-op entry to other [Peer]s upon being elected.
    /// This ensured the log replication process starts as soon as the leader is elected.
    fn no_op() -> Self;
}

/// Result of a [Command].
///
/// Command results are for [Peer]s to return the results of [Command]s of [Client]s.
/// They need to be able to represent the error cases.
///
/// For a basic key-value database application, it could be defined as:
/// ```
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// enum CommandResult {
///     /// Operation is applied to the database.
///     Done,
///     /// Modified key is not found in the database.
///     NotFound,
/// }
/// # impl rafty::prelude::RaftCommandResult for CommandResult {}
/// ```
pub trait CommandResult:
    Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static
{
}
