//! # cryo — a friendly QQ Bot framework
//!
//! Typed event contexts, axum-style parameter injection, fluent message
//! builders and multi-bot fleets.
//!
//! ```no_run
//! use cryo::{Bot, C2CMessageCtx, PlainText, model::Intent};
//!
//! #[tokio::main]
//! async fn main() {
//!     Bot::from_env(Intent::BASIC)
//!         .on_c2c_message(|c: C2CMessageCtx, text: PlainText| {
//!             c.say(format!("echo: {text}"))
//!         })
//!         .run()
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! ## Concepts
//!
//! - **handlers** — any `async fn` whose parameters implement [`FromEvent`];
//!   declare what you need (`PlainText`, `SenderOpenId`, `C2CMessageCtx`,
//!   `Inject<YourService>`…) and it is produced from the event at compile time.
//!   Return `()` or `cryo::Result<()>` — `Ok(())` needs no annotation.
//! - **typed ctxs** — [`C2CMessageCtx`] & friends decode `payload.d` and
//!   expose `.say()` / `.reply()` / `.active()` bound message builders;
//!   awaiting a builder sends it (`c.say("hi").await`).
//! - **services** — `bot.provide(svc)` + the [`Inject<T>`] extractor.
//! - **permissions** — every [`Config`] requires an explicit [`Intent`](model::Intent)
//!   declaration; there is no silent default.
//! - **fleets** — [`Fleet`] runs N bot accounts in one process; registrations
//!   made on the fleet apply to all members.

#![allow(clippy::upper_case_acronyms)]

pub mod api;
pub mod model;

mod bot;
mod context;
mod default;
mod error;
mod extract;
mod fleet;
mod gateway;
mod handler;
mod reply;
#[cfg(test)]
mod tests;

pub use bot::{Bot, Config};
pub use context::{
    C2CMessageCtx, EventCtx, FriendCtx, GroupBotCtx, GroupMemberCtx, GroupMessageCtx,
    InteractionCtx, MessageAuditCtx,
};
pub use default::{
    api, init, init_env, on, on_c2c_message, on_friend_add, on_friend_del, on_group_at_message,
    on_group_join, on_group_leave, on_group_member_add, on_group_member_remove, on_group_message,
    on_interaction, on_message_audited, provide, provide_shared, run,
};
pub use error::{BoxError, Error, Result};
pub use extract::{EventId, FromEvent, GroupOpenId, Inject, MessageId, PlainText, SenderOpenId};
pub use fleet::Fleet;
pub use reply::ReplyBuilder;

pub use tokio_util::sync::CancellationToken;
