use crate::*;

/// [RaftCommand] to a [KeyValueDatabase].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Used internally for replication.
    NoOp,
    /// Inserts a key with a value.
    Insert { key: String, value: String },
    /// Upserts a key with a value.
    Upsert { key: String, value: String },
    /// Clears a key.
    Clear { key: String },
}

impl RaftCommand for Command {
    fn no_op() -> Self {
        Command::NoOp
    }
}

/// [RaftCommandResult] of a [Command].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CommandResult {
    /// Command executed successfully.
    Done,
    /// Key to be inserted already exists in the database.
    AlreadyExists,
}

impl RaftCommandResult for CommandResult {}
