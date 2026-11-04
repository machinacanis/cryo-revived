//! Gateway websocket session (internal — driven by `Bot::run`).

mod session;

pub(crate) use session::Session;
