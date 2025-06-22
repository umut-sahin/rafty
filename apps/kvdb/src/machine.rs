use crate::*;

/// [RaftMachine] of a [KeyValueDatabase].
#[derive(Clone, Default, Eq, PartialEq, Serialize, Deserialize, derive_more::Debug)]
#[debug("{_0:#?}")]
pub struct Machine(pub BTreeMap<String, String>);

impl<S: RaftStorage<KeyValueDatabase<S>>> RaftMachine<KeyValueDatabase<S>> for Machine {
    fn apply(&mut self, command: &Command) -> CommandResult {
        match command {
            Command::NoOp => CommandResult::Done,
            Command::Insert { key, value } => {
                match self.0.entry(key.clone()) {
                    BTreeMapEntry::Vacant(slot) => {
                        slot.insert(value.clone());
                        CommandResult::Done
                    },
                    BTreeMapEntry::Occupied(_) => CommandResult::AlreadyExists,
                }
            },
            Command::Upsert { key, value } => {
                self.0.insert(key.clone(), value.clone());
                CommandResult::Done
            },
            Command::Clear { key } => {
                self.0.remove(key);
                CommandResult::Done
            },
        }
    }

    fn query(&self, query: &Query) -> QueryResult {
        match query {
            Query::Length => QueryResult::Length { length: self.0.len() },
            Query::Entry { key } => QueryResult::Entry { value: self.0.get(key).cloned() },
        }
    }
}
