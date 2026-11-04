//! Error types shared across the crate.

/// `Box<dyn Error + Send + Sync>` — the error type of erased handlers, so a
/// handler may return `Result<(), E>` for any `E: Into<BoxError>`.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Convenient alias used by all framework APIs.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Api(#[from] crate::api::ApiError),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("websocket: {0}")]
    Ws(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// An extractor could not produce its value from this event.
    #[error("{0}")]
    Extract(String),
    /// `bot.provide` was never called for this type.
    #[error("no service registered for {0}; call bot.provide(...)")]
    NoService(&'static str),
    #[error("gateway: {0}")]
    Gateway(String),
    /// Close code 4914/4915 — banned or taken offline; do not retry.
    #[error("fatal gateway close {0}: {1}")]
    FatalClose(u16, String),
    /// The gateway session cannot be resumed; re-identify is required.
    #[error("session cannot be resumed")]
    NoResume,
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::Ws(Box::new(e))
    }
}
