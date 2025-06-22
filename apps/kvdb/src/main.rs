//! Debugger for `rafty-kvdb`.

use {
    anyhow::Context,
    clap::Parser as Clap,
    rafty::prelude::*,
    rafty_debugger::*,
    rafty_kvdb::*,
    rafty_simulator::*,
    std::path::PathBuf,
};

mod storage;
use storage::Storage;

mod widgets;
use widgets::{
    CommandSelectionWidget,
    QuerySelectionWidget,
};

#[derive(Clap)]
struct Args {
    /// Sets the directory to store persistent peer data.
    #[clap(long)]
    data: Option<PathBuf>,

    /// Sets the number of clients.
    #[clap(long)]
    clients: Option<usize>,

    /// Sets the number of peers.
    #[clap(long)]
    peers: Option<usize>,

    /// Enables eventual consistency instead of strong consistency.
    #[clap(long)]
    eventual: bool,

    /// Resets the persistent peer data.
    #[clap(long)]
    reset: bool,

    /// Keeps the persistent peer data read-only.
    #[clap(long)]
    readonly: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let data_directory = args.data.clone().unwrap_or(PathBuf::from(".data"));

    let consistency = if args.eventual { Consistency::Eventual } else { Consistency::Strong };
    let peer_storages = (1..=args.peers.unwrap_or(5))
        .map(|peer_id| {
            Storage::new(data_directory.join(peer_id.to_string()), args.reset)
                .map(|storage| storage.readonly(args.readonly))
                .with_context(|| format!("Failed to initialize the storage of peer {peer_id}"))
        })
        .collect::<anyhow::Result<Vec<Storage>>>()?;
    let number_of_clients = args.clients.unwrap_or(2);

    let simulation =
        Simulation::<KeyValueDatabase<Storage>>::new(consistency, peer_storages, number_of_clients)
            .context("Failed to initialize the simulation")?;
    Debugger::<KeyValueDatabase<Storage>, CommandSelectionWidget, QuerySelectionWidget>::new(
        simulation,
    )?
    .start()
}
