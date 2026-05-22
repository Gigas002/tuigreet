//! Core greeter: greetd IPC, session discovery, greeter state, keyboard, events.

mod strings;

pub mod event;
pub mod greeter;
pub mod info;
pub mod ipc;
pub mod keyboard;
pub mod model;
pub mod power;

pub use event::Event;
pub use greeter::*;
