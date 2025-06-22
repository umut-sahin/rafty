#![cfg_attr(doctest, doc = "````no_test")]
#![doc = include_str!("../README.md")]

mod application;
mod command;
mod machine;
mod query;

#[doc(inline)]
pub use crate::{
    application::KeyValueDatabase,
    command::{
        Command,
        CommandResult,
    },
    machine::Machine,
    query::{
        Query,
        QueryResult,
    },
};

pub(crate) use {
    rafty::prelude::*,
    serde::{
        Deserialize,
        Serialize,
    },
    std::{
        collections::{
            btree_map::Entry as BTreeMapEntry,
            BTreeMap,
        },
        fmt::{
            self,
            Debug,
        },
        marker::PhantomData,
    },
};
