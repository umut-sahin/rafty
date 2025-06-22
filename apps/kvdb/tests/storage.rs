//! Test storage definition.

use {
    rafty::prelude::*,
    rafty_kvdb::*,
    serde::{
        Deserialize,
        Serialize,
    },
};

/// An in-memory [RaftStorage] for testing
#[derive(Clone)]
pub struct Storage {
    pub(crate) current_term: Term,
    pub(crate) voted_for: Option<PeerId>,
    pub(crate) log: Log<KeyValueDatabase<Storage>>,
    pub(crate) snapshot: Snapshot<KeyValueDatabase<Storage>>,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            current_term: Term(0),
            voted_for: None,
            log: Log::default(),
            snapshot: Snapshot::default(),
        }
    }
}

impl RaftStorage<KeyValueDatabase<Self>> for Storage {
    type Error = StorageError;

    fn current_term(&self) -> Term {
        self.current_term
    }

    fn set_current_term(&mut self, term: Term) -> Result<(), Self::Error> {
        self.current_term = term;
        Ok(())
    }

    fn voted_for(&self) -> Option<PeerId> {
        self.voted_for
    }

    fn set_voted_for(&mut self, voted_for: Option<PeerId>) -> Result<(), Self::Error> {
        self.voted_for = voted_for;
        Ok(())
    }

    fn set_current_term_and_voted_for(
        &mut self,
        current_term: Term,
        voted_for: Option<PeerId>,
    ) -> Result<(), Self::Error> {
        self.current_term = current_term;
        self.voted_for = voted_for;
        Ok(())
    }

    fn log(&self) -> &Log<KeyValueDatabase<Self>> {
        &self.log
    }

    fn append_log_entry(
        &mut self,
        entry: LogEntry<KeyValueDatabase<Self>>,
    ) -> Result<(), Self::Error> {
        self.log.push(entry);
        Ok(())
    }

    fn truncate_log(&mut self, down_to: LogIndex) -> Result<(), Self::Error> {
        self.log.truncate(down_to.0);
        Ok(())
    }

    fn snapshot(&self) -> &Snapshot<KeyValueDatabase<Self>> {
        &self.snapshot
    }

    fn install_snapshot(
        &mut self,
        snapshot: Snapshot<KeyValueDatabase<Self>>,
    ) -> Result<(), Self::Error> {
        self.snapshot = snapshot;
        Ok(())
    }
}

/// Errors that can happen during [Storage] operations.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    derive_more::Error,
    derive_more::Display
)]
pub enum StorageError {}
