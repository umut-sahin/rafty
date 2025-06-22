//! Preludes of the crate.

#[doc(inline)]
pub use crate::{
    application::Application as RaftApplication,
    client::Client,
    command::{
        Command as RaftCommand,
        CommandResult as RaftCommandResult,
    },
    errors::ClientError,
    log::{
        Log,
        LogEntry,
    },
    machine::Machine as RaftMachine,
    message::{
        AppendEntriesReply,
        AppendEntriesRequest,
        ClientMessage,
        CommandReply,
        CommandRequest,
        PeerMessage,
        QueryReply,
        QueryRequest,
        RequestVoteReply,
        RequestVoteRequest,
        Vote,
    },
    peer::Peer,
    primitives::{
        ClientId,
        Cluster,
        Consistency,
        LogIndex,
        PeerId,
        RequestCounter,
        RequestId,
        Term,
    },
    query::{
        Query as RaftQuery,
        QueryResult as RaftQueryResult,
    },
    role::{
        CandidateState,
        FollowerState,
        LeaderState,
        Role,
    },
    snapshot::Snapshot,
    storage::Storage as RaftStorage,
    transmit::{
        ClientTransmit,
        PeerTransmit,
    },
};

pub(crate) use {
    crate::{
        application::Application,
        command::{
            Command,
            CommandResult,
        },
        machine::Machine,
        query::{
            Query,
            QueryResult,
        },
        storage::Storage,
    },
    rand::prelude::*,
    serde::{
        de::DeserializeOwned,
        Deserialize,
        Serialize,
    },
    std::{
        collections::{
            BTreeMap,
            BTreeSet,
            VecDeque,
        },
        error::Error,
        fmt::Debug,
        ops::{
            Deref,
            DerefMut,
        },
        sync::atomic::{
            AtomicUsize,
            Ordering as AtomicOrdering,
        },
    },
};
