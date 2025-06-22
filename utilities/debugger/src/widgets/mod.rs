mod control;
mod info;
mod logs;
mod scroll;

pub use {
    control::ControlWidget,
    info::{
        InfoWidget,
        MainTabSelection,
    },
    logs::LogsWidget,
    scroll::ScrollWidget,
};
