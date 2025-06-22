//! Query interface.

use crate::prelude::*;

/// Query of an [Application].
///
/// Queries are for [Client]s to get information about the replicated [Machine]s via [Peer]s.
///
/// For a basic key-value database application, it could be defined as:
/// ```
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// enum Query {
///     /// Gets the length of the database.
///     Len,
///     /// Gets the value of an entry in the database.
///     Get { key: String },
/// }
/// # impl rafty::prelude::RaftQuery for Query {}
/// ```
pub trait Query:
    Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static
{
}

/// Result of a [Query].
///
/// Query results are for [Peer]s to return the results of [Query]s of [Client]s.
/// They need to be able to represent the error cases.
///
/// For a basic key-value database application, it could be defined as:
/// ```
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// enum QueryResult {
///     /// Length of the database.
///     Len { len: usize },
///     /// Value of the queried key in the database.
///     Value { value: String },
///     /// Queried key is not found in the database.
///     NotFound,
/// }
/// # impl rafty::prelude::RaftQueryResult for QueryResult {}
/// ```
pub trait QueryResult:
    Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static
{
}
