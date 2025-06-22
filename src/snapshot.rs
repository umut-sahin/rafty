//! Snapshot definitions.

use crate::prelude::*;

/// Snapshot of a [Machine] after [LogEntry]s up to a certain [LogIndex] is applied.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Snapshot<A: Application> {
    last_included_index: LogIndex,
    last_included_term: Term,
    machine: A::Machine,
}

impl<A: Application> Snapshot<A> {
    /// Gets the index of the last applied log entry before the snapshot is taken.
    pub fn last_included_index(&self) -> LogIndex {
        self.last_included_index
    }

    /// Gets the term of the last applied log entry before the snapshot is taken.
    pub fn last_included_term(&self) -> Term {
        self.last_included_term
    }

    /// Gets the machine when the snapshot is taken.
    pub fn machine(&self) -> &A::Machine {
        &self.machine
    }
}

impl<A: Application> Default for Snapshot<A> {
    fn default() -> Self {
        Self {
            last_included_index: LogIndex(0),
            last_included_term: Term(0),
            machine: A::Machine::default(),
        }
    }
}
