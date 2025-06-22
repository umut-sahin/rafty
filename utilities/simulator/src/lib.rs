#![cfg_attr(doctest, doc = "````no_test")]
#![doc = include_str!("../README.md")]

mod action;
mod simulation;
mod update;

#[doc(inline)]
pub use {
    action::Action,
    simulation::Simulation,
    update::Update,
};

pub(crate) use {
    anyhow::Context,
    rafty::prelude::*,
    std::{
        collections::{
            BTreeMap,
            BTreeSet,
            VecDeque,
        },
        fmt::Debug,
    },
};
