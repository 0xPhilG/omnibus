//! A lightweight, in-process publish/subscribe event bus for Rust.
//!
//! # Overview
//!
//! `omnibus` lets multiple parts of an application exchange typed events without
//! direct dependencies between them. Every [`Bus`] is cheaply cloneable — clones
//! share the same subscription registry — making it easy to pass a handle across
//! threads or into async tasks.
//!
//! # Quick start
//!
//! ```rust
//! use omnibus::{Bus, Event, Filter};
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let bus = Bus::<String, String, String>::new();
//!
//! let sub = bus.subscribe(
//!     Filter::Is("sensor".to_string()),
//!     Filter::Is("temperature".to_string()),
//! )?;
//!
//! bus.publish(Event::new(
//!     "sensor".to_string(),
//!     "temperature".to_string(),
//!     "42.5\u{b0}C".to_string(),
//! ))?;
//!
//! if let Some(event) = sub.recv()? {
//!     println!("[{} / {}] {}", event.origin(), event.kind(), event.payload());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Key types
//!
//! | Type | Purpose |
//! |------|---------|
//! | [`Bus`] | Central hub; clone freely to share across threads |
//! | [`Filter`] | Controls which events a subscription receives |
//! | [`Event`] | A single message: origin + kind + payload |
//! | [`BusReceiver`] | Subscription handle; multiple receive styles |
//! | [`OmnibusError`] | Error type returned by all fallible operations |

#![warn(missing_docs)]

pub(crate) mod bus;
pub(crate) mod error;
pub(crate) mod event;
pub(crate) mod filter;

pub use bus::{Bus, BusReceiver, PublishResult};
pub use error::{OmnibusError, Result};
pub use event::Event;
pub use filter::Filter;
