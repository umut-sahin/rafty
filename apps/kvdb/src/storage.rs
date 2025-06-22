use {
    rafty::prelude::*,
    rafty_kvdb::*,
    serde::{
        Deserialize,
        Serialize,
    },
    std::{
        fs::{
            File,
            OpenOptions,
        },
        io::{
            BufRead,
            BufReader,
            Read,
            Seek,
            SeekFrom,
            Write,
        },
        path::Path,
    },
};

/// A [File] based [RaftStorage] for [KeyValueDatabase].
pub struct Storage {
    state_file: File,
    log_file: File,
    snapshot_file: File,

    state: State,
    log: Log<KeyValueDatabase<Self>>,
    snapshot: Snapshot<KeyValueDatabase<Self>>,

    readonly: bool,
}

impl Storage {
    /// Creates a new storage.
    pub fn new(directory: impl AsRef<Path>, reset: bool) -> Result<Self, StorageError> {
        let directory = directory.as_ref();
        if !directory.exists() {
            std::fs::create_dir_all(directory)
                .map_err(|error| StorageError::CreatingDataDirectory(error.to_string()))?;
        }

        let state_path = directory.join("state.json");
        let log_path = directory.join("log");
        let snapshot_path = directory.join("snapshot.json");

        let mut state_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .append(false)
            .open(state_path)
            .map_err(|error| StorageError::OpeningStateFile(error.to_string()))?;
        let mut log_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .append(false)
            .open(log_path)
            .map_err(|error| StorageError::OpeningLogFile(error.to_string()))?;
        let mut snapshot_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .append(false)
            .open(snapshot_path)
            .map_err(|error| StorageError::OpeningSnapshotFile(error.to_string()))?;

        if reset {
            Storage::overwrite(&mut state_file, "")
                .map_err(|error| StorageError::ResettingStateFile(error.to_string()))?;
            Storage::overwrite(&mut log_file, "")
                .map_err(|error| StorageError::ResettingLogFile(error.to_string()))?;
            Storage::overwrite(&mut snapshot_file, "")
                .map_err(|error| StorageError::ReadingSnapshotFile(error.to_string()))?;
        }

        log_file
            .seek(SeekFrom::End(0))
            .map_err(|error| StorageError::OpeningLogFile(error.to_string()))?;

        let mut state_string = String::new();
        state_file
            .read_to_string(&mut state_string)
            .map_err(|error| StorageError::ResettingStateFile(error.to_string()))?;

        let mut first_run = false;
        let state = if state_string.is_empty() {
            first_run = true;
            State { current_term: Term(0), voted_for: None }
        } else {
            serde_json::from_str(&state_string)
                .map_err(|error| StorageError::ParsingState(error.to_string()))?
        };

        let mut storage = Storage {
            state_file,
            log_file,
            snapshot_file,
            state,
            log: Log::default(),
            snapshot: Snapshot::default(),
            readonly: false,
        };
        if first_run {
            storage
                .flush_state()
                .map_err(|error| StorageError::InitializingStateFile(Box::new(error)))?;
            Storage::overwrite(&mut storage.log_file, "")
                .map_err(|error| StorageError::InitializingLogFile(error.to_string()))?;
            storage
                .install_snapshot(Snapshot::default())
                .map_err(|error| StorageError::InitializingSnapshotFile(Box::new(error)))?;
        } else {
            let mut log_string = String::new();
            storage
                .log_file
                .seek(SeekFrom::Start(0))
                .map_err(|error| StorageError::ReadingLogFile(error.to_string()))?;
            storage
                .log_file
                .read_to_string(&mut log_string)
                .map_err(|error| StorageError::ReadingLogFile(error.to_string()))?;
            storage.log = log_string
                .split("\n")
                .enumerate()
                .filter(|(_, line)| !line.is_empty())
                .map(|(i, log_entry_string)| {
                    serde_json::from_str::<LogEntry<KeyValueDatabase<Storage>>>(log_entry_string)
                        .map_err(|error| StorageError::ParsingLogEntry(i + 1, error.to_string()))
                })
                .collect::<Result<Vec<_>, StorageError>>()?
                .into();

            let mut snapshot_string = String::new();
            storage
                .snapshot_file
                .seek(SeekFrom::Start(0))
                .map_err(|error| StorageError::ReadingSnapshotFile(error.to_string()))?;
            storage
                .snapshot_file
                .read_to_string(&mut snapshot_string)
                .map_err(|error| StorageError::ReadingSnapshotFile(error.to_string()))?;
            storage.snapshot = serde_json::from_str(&snapshot_string)
                .map_err(|error| StorageError::ParsingSnapshot(error.to_string()))?;
        }

        Ok(storage)
    }

    /// Sets the readonly status of the storage.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
}

impl Storage {
    fn overwrite(file: &mut File, content: &str) -> std::io::Result<()> {
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        if !content.is_empty() {
            file.write_all(content.as_bytes())?;
        }
        file.flush()?;
        Ok(())
    }

    fn flush_state(&mut self) -> Result<(), StorageError> {
        let state_string = serde_json::to_string_pretty(&self.state)
            .map_err(|error| StorageError::SerializingState(error.to_string()))?;
        Storage::overwrite(&mut self.state_file, &state_string)
            .map_err(|error| StorageError::WritingState(error.to_string()))
    }
}

impl RaftStorage<KeyValueDatabase<Storage>> for Storage {
    type Error = StorageError;

    fn current_term(&self) -> Term {
        self.state.current_term
    }

    fn set_current_term(&mut self, term: Term) -> Result<(), Self::Error> {
        let old_term = self.state.current_term;
        self.state.current_term = term;

        if self.readonly {
            return Ok(());
        }
        self.flush_state().inspect_err(|_| {
            self.state.current_term = old_term;
        })
    }

    fn voted_for(&self) -> Option<PeerId> {
        self.state.voted_for
    }

    fn set_voted_for(&mut self, voted_for: Option<PeerId>) -> Result<(), Self::Error> {
        let old_voted_for = self.state.voted_for;
        self.state.voted_for = voted_for;

        if self.readonly {
            return Ok(());
        }
        self.flush_state().inspect_err(|_| {
            self.state.voted_for = old_voted_for;
        })
    }

    fn set_current_term_and_voted_for(
        &mut self,
        current_term: Term,
        voted_for: Option<PeerId>,
    ) -> Result<(), Self::Error> {
        let old_term = self.state.current_term;
        let old_voted_for = self.state.voted_for;

        self.state.current_term = current_term;
        self.state.voted_for = voted_for;

        if self.readonly {
            return Ok(());
        }
        self.flush_state().inspect_err(|_| {
            self.state.current_term = old_term;
            self.state.voted_for = old_voted_for;
        })
    }

    fn log(&self) -> &Log<KeyValueDatabase<Storage>> {
        &self.log
    }

    fn append_log_entry(
        &mut self,
        entry: LogEntry<KeyValueDatabase<Storage>>,
    ) -> Result<(), Self::Error> {
        if self.readonly {
            self.log.push(entry);
            return Ok(());
        }

        let mut entry_string = serde_json::to_string(&entry)
            .map_err(|error| StorageError::SerializingLogEntry(error.to_string()))?;
        entry_string += "\n";

        self.log_file
            .write_all(entry_string.as_bytes())
            .map_err(|error| StorageError::AppendingLogEntry(error.to_string()))?;
        let result = self
            .log_file
            .flush()
            .map_err(|error| StorageError::AppendingLogEntry(error.to_string()));

        if result.is_ok() {
            self.log.push(entry);
        }
        result
    }

    fn truncate_log(&mut self, down_to: LogIndex) -> Result<(), Self::Error> {
        if self.readonly {
            self.log.retain(|entry| entry.index() < down_to);
            return Ok(());
        }

        let log_file = self
            .log_file
            .try_clone()
            .map_err(|error| StorageError::OpeningLogFile(error.to_string()))?;
        let mut reader = BufReader::new(log_file);

        let mut line = 0;
        let mut new_content = String::new();

        let mut new_log = Log::default();

        let mut buffer = String::new();
        loop {
            let read = reader
                .read_line(&mut buffer)
                .map_err(|error| StorageError::ReadingLogFile(error.to_string()))?;
            if read == 0 {
                break;
            }

            line += 1;

            if buffer.trim().is_empty() {
                new_content += &buffer;
                buffer.clear();
                continue;
            }

            match serde_json::from_str::<LogEntry<KeyValueDatabase<Storage>>>(&buffer) {
                Ok(entry) => {
                    if entry.index() >= down_to {
                        break;
                    }
                    new_log.push(entry);
                    new_content += &buffer;
                    buffer.clear();
                },
                Err(error) => return Err(StorageError::ParsingLogEntry(line, error.to_string())),
            }
        }
        drop(reader);

        let result = Storage::overwrite(&mut self.log_file, &new_content)
            .map_err(|error| StorageError::TruncatingLogFile(error.to_string()));

        if result.is_ok() {
            self.log = new_log;
        }
        result
    }

    fn snapshot(&self) -> &Snapshot<KeyValueDatabase<Storage>> {
        &self.snapshot
    }

    fn install_snapshot(
        &mut self,
        snapshot: Snapshot<KeyValueDatabase<Storage>>,
    ) -> Result<(), Self::Error> {
        if self.readonly {
            self.snapshot = snapshot;
            return Ok(());
        }

        let snapshot_string = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| StorageError::SerializingSnapshot(error.to_string()))?;

        let result = Storage::overwrite(&mut self.snapshot_file, &snapshot_string)
            .map_err(|error| StorageError::WritingSnapshot(error.to_string()));

        if result.is_ok() {
            self.snapshot = snapshot;
        }
        result
    }
}

/// Errors that can happen during persistent [Storage] updates.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    derive_more::Display,
    derive_more::Error
)]
pub enum StorageError {
    #[display("Unable to create the data directory: {_0}")]
    CreatingDataDirectory(#[error(not(source))] String),

    #[display("Unable to open the persistent state file: {_0}")]
    OpeningStateFile(#[error(not(source))] String),
    #[display("Unable to initialize the persistent state file: {_0}")]
    InitializingStateFile(Box<StorageError>),
    #[display("Unable to read the persistent state file: {_0}")]
    ReadingStateFile(#[error(not(source))] String),
    #[display("Unable to parse the persistent state file: {_0}")]
    ParsingState(#[error(not(source))] String),
    #[display("Unable to serialize the new state: {_0}")]
    SerializingState(#[error(not(source))] String),
    #[display("Unable to write the new state to the state file persistently: {_0}")]
    WritingState(#[error(not(source))] String),
    #[display("Unable to reset the persistent state file: {_0}")]
    ResettingStateFile(#[error(not(source))] String),

    #[display("Unable to open the persistent log file: {_0}")]
    OpeningLogFile(#[error(not(source))] String),
    #[display("Unable to initialize the persistent log file: {_0}")]
    InitializingLogFile(#[error(not(source))] String),
    #[display("Unable to read the persistent log file: {_0}")]
    ReadingLogFile(#[error(not(source))] String),
    #[display("Unable to parse the log entry at line {_0} in the persistent log file: {_1}")]
    ParsingLogEntry(usize, #[error(not(source))] String),
    #[display("Unable to serialize the new log entry: {_0}")]
    SerializingLogEntry(#[error(not(source))] String),
    #[display("Unable to append the new log entry to the log file persistently: {_0}")]
    AppendingLogEntry(#[error(not(source))] String),
    #[display("Unable to reset the persistent log file: {_0}")]
    ResettingLogFile(#[error(not(source))] String),
    #[display("Unable to truncate the persistent log file: {_0}")]
    TruncatingLogFile(#[error(not(source))] String),

    #[display("Unable to open the persistent snapshot file: {_0}")]
    OpeningSnapshotFile(#[error(not(source))] String),
    #[display("Unable to initialize the persistent snapshot file: {_0}")]
    InitializingSnapshotFile(Box<StorageError>),
    #[display("Unable to read the persistent snapshot file: {_0}")]
    ReadingSnapshotFile(#[error(not(source))] String),
    #[display("Unable to parse the persistent snapshot file: {_0}")]
    ParsingSnapshot(#[error(not(source))] String),
    #[display("Unable to serialize the new snapshot: {_0}")]
    SerializingSnapshot(#[error(not(source))] String),
    #[display("Unable to write the new snapshot to the snapshot file persistently: {_0}")]
    WritingSnapshot(#[error(not(source))] String),
    #[display("Unable to reset the persistent snapshot file: {_0}")]
    ResettingSnapshotFile(#[error(not(source))] String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct State {
    current_term: Term,
    voted_for: Option<PeerId>,
}
