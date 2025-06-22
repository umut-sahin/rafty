use crate::*;

/// [RaftQuery] on a [KeyValueDatabase].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Query {
    Length,
    Entry { key: String },
}

impl RaftQuery for Query {}

/// [RaftQueryResult] of a [Query].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QueryResult {
    Length { length: usize },
    Entry { value: Option<String> },
}

impl RaftQueryResult for QueryResult {}
