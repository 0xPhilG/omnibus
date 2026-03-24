use thiserror::Error;

/// Errors returned by the omnibus message bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OmnibusError {
    /// The channel was disconnected; all senders have been dropped.
    #[error("channel disconnected")]
    Disconnected,

    /// An internal lock was poisoned by a panic in another thread.
    #[error("internal lock poisoned")]
    Poisoned,
}

/// Convenience alias for [`Result`](std::result::Result)`<T, `[`OmnibusError`]`>`.
///
/// All fallible functions in this crate return this type.
pub type Result<T> = std::result::Result<T, OmnibusError>;
