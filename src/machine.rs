//! Machine interface.

use crate::prelude::*;

/// Machine of an [Application] to replicate.
///
/// For a basic key-value database application, it could be defined as:
/// ```
/// # use std::collections::BTreeMap;
/// # use serde::{Deserialize, Serialize};
/// #[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
/// struct Machine(BTreeMap<String, String>);
/// ```
pub trait Machine<A: Application<Machine = Self>>:
    Clone + Debug + Default + Eq + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static
{
    /// Applies a [Command] to the machine.
    fn apply(&mut self, command: &A::Command) -> A::CommandResult;

    /// Runs a [Query] in the machine.
    fn query(&self, query: &A::Query) -> A::QueryResult;
}
