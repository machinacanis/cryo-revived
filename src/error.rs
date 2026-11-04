//! Error types shared across the crate.
//!
//! `cryo::Error` is [`anyhow::Error`]: handlers return `cryo::Result<()>`
//! and `?` converts any `std::error::Error`, so application errors never
//! need a `From` declaration. Typed errors that drive control flow are
//! marker structs checked via [`anyhow::Error::is`] / `downcast_ref`.

pub use anyhow::Error;

/// Convenient alias used by all framework APIs.
pub type Result<T> = anyhow::Result<T>;

/// The gateway session cannot be resumed; re-identify is required.
#[derive(Debug, thiserror::Error)]
#[error("session cannot be resumed")]
pub struct NoResume;

/// Close code 4914/4915 — banned or taken offline; do not retry.
#[derive(Debug, thiserror::Error)]
#[error("fatal gateway close {0}: {1}")]
pub struct FatalClose(pub u16, pub String);

/// A gateway-layer failure (`gateway: ` prefix).
pub(crate) fn gateway(msg: impl Into<String>) -> Error {
    Error::msg(format!("gateway: {}", msg.into()))
}
