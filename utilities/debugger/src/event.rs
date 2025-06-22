use crate::*;

#[derive(Debug)]
pub enum DebuggerEvent {
    UserEvent(Event),
    Redraw,
}
