//! retty-io is a collection of Metal IO (mio) based utilities, like channel, timer, UDP socket with ECN, etc.

#![warn(rust_2018_idioms)]
#![allow(dead_code)]
#![warn(missing_docs)]

pub mod broadcast;

pub use mio_extras::{channel, timer};
