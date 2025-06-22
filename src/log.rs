//! Log definitions.

use crate::prelude::*;

/// Log of a [Peer].
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, derive_more::From)]
pub struct Log<A: Application>(#[from] Vec<LogEntry<A>>);

impl<A: Application> Deref for Log<A> {
    type Target = Vec<LogEntry<A>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A: Application> DerefMut for Log<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A: Application> Log<A> {
    /// Gets the log entry with the given index.
    pub fn entry(&self, index: LogIndex) -> Option<&LogEntry<A>> {
        self.binary_search_by_key(&index, |entry| entry.index()).map(|index| &self[index]).ok()
    }
}


/// Entries within a [Log].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, bon::Builder)]
#[serde(bound = "A::Command : Serialize + DeserializeOwned")]
pub struct LogEntry<A: Application> {
    #[builder(into)]
    index: LogIndex,

    #[builder(into)]
    term: Term,

    #[builder(into)]
    command: A::Command,
}

impl<A: Application> LogEntry<A> {
    /// Gets the index of the log entry.
    pub fn index(&self) -> LogIndex {
        self.index
    }

    /// Gets the term of the log entry
    pub fn term(&self) -> Term {
        self.term
    }

    /// Gets the command of the log entry.
    pub fn command(&self) -> &A::Command {
        &self.command
    }
}
