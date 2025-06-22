use crate::*;

/// A Key-Value database application based on a generic [RaftStorage].
pub struct KeyValueDatabase<S: RaftStorage<Self>>(PhantomData<S>);

impl<S: RaftStorage<Self>> Default for KeyValueDatabase<S> {
    fn default() -> Self {
        KeyValueDatabase(PhantomData)
    }
}

impl<S: RaftStorage<Self>> Eq for KeyValueDatabase<S> {}

impl<S: RaftStorage<Self>> PartialEq for KeyValueDatabase<S> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<S: RaftStorage<Self>> Debug for KeyValueDatabase<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyValueDatabase")
    }
}

impl<S: RaftStorage<Self>> Clone for KeyValueDatabase<S> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<S: RaftStorage<Self>> RaftApplication for KeyValueDatabase<S> {
    type Machine = Machine;

    type Command = Command;
    type CommandResult = CommandResult;

    type Query = Query;
    type QueryResult = QueryResult;

    type Storage = S;
    type StorageError = S::Error;
}
