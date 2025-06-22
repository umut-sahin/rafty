#![cfg_attr(doctest, doc = "````no_test")]
#![doc = include_str!("../README.md")]

mod debugger;
mod event;
mod widgets;

#[doc(inline)]
pub use debugger::{
    CommandWidget,
    Debugger,
    QueryWidget,
};

pub(crate) use {
    crate::{
        debugger::{
            DebuggerState,
            NextDebuggerState,
        },
        widgets::{
            ControlWidget,
            InfoWidget,
            LogsWidget,
            MainTabSelection,
            ScrollWidget,
        },
    },
    anyhow::Context,
    crossterm::{
        event::{
            DisableMouseCapture,
            EnableMouseCapture,
            Event,
            KeyCode as Key,
            KeyModifiers,
            MouseEventKind,
        },
        terminal::{
            EnterAlternateScreen,
            LeaveAlternateScreen,
        },
    },
    event::DebuggerEvent,
    log::{
        Level,
        LevelFilter,
    },
    rafty::prelude::*,
    rafty_simulator::{
        Action as SimulationAction,
        *,
    },
    ratatui::{
        layout::{
            Flex,
            Offset,
        },
        prelude::*,
        symbols::scrollbar::Set as ScrollbarSet,
        widgets::{
            Block,
            BorderType,
            Borders,
            List,
            ListState,
            Padding,
            Paragraph,
            Scrollbar,
            ScrollbarOrientation,
            ScrollbarState,
            Tabs,
        },
    },
    std::{
        io,
        str::Chars,
        sync::mpsc,
        thread,
        time::Duration,
    },
    tui_logger::{
        ExtLogRecord,
        LogFormatter,
        TuiLoggerWidget as LoggerWidget,
        TuiWidgetEvent as LoggerEvent,
        TuiWidgetState as LoggerState,
    },
};
