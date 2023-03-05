#![warn(rust_2018_idioms)]
#![allow(dead_code)]

mod channel;

pub use channel::{channel, Receiver, Sender};
