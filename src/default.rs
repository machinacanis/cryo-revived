//! Global default Bot — `http.HandleFunc` style, for prototypes and
//! single-bot apps. Registration and definition sit next to each other:
//!
//! ```no_run
//! use cryo::{Config, model::Intent, C2CMessageCtx};
//!
//! #[tokio::main]
//! async fn main() {
//!     cryo::init(Config::from_env(Intent::BASIC)); // 必须先声明权限
//!     cryo::on_c2c_message(|c: C2CMessageCtx| async move {
//!         c.reply().text("hi").send().await
//!     });
//!     cryo::run().await.unwrap();
//! }
//! ```
//!
//! For multiple instances or custom configs use [`crate::Bot`] /
//! [`crate::Fleet`] directly.

use std::sync::Arc;
use std::sync::OnceLock;

use crate::bot::{Bot, Config};
use crate::handler::Handler;
use crate::model::event;
use crate::Result;

static DEFAULT: OnceLock<Bot> = OnceLock::new();

/// Initialize the default bot. Must be called before any `cryo::on_*` or
/// [`run`]. `Config` carries the explicit intent declaration — the default
/// bot never picks permissions for you.
///
/// # Panics
/// If called twice, or if `cfg.intents` is zero.
pub fn init(cfg: Config) -> Bot {
    let b = Bot::new(cfg);
    if DEFAULT.set(b.clone()).is_err() {
        panic!("cryo: default bot already initialized");
    }
    b
}

fn default() -> &'static Bot {
    DEFAULT.get().expect(
        "cryo: call cryo::init(Config) first — \
         the default bot requires explicit intents",
    )
}

/// Register a handler on the default bot for `event_type` ([`crate::model::event`])
/// or `"*"`.
pub fn on<H, Args>(event_type: &str, h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    default().on(event_type, h);
    default()
}

pub fn on_c2c_message<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::C2C_MESSAGE_CREATE, h)
}

pub fn on_group_at_message<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::GROUP_AT_MESSAGE, h)
}

pub fn on_group_message<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::GROUP_MESSAGE, h)
}

pub fn on_friend_add<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::FRIEND_ADD, h)
}

pub fn on_friend_del<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::FRIEND_DEL, h)
}

pub fn on_group_join<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::GROUP_ADD_ROBOT, h)
}

pub fn on_group_leave<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::GROUP_DEL_ROBOT, h)
}

pub fn on_group_member_add<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::GROUP_MEMBER_ADD, h)
}

pub fn on_group_member_remove<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::GROUP_MEMBER_REMOVE, h)
}

pub fn on_interaction<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    on(event::INTERACTION_CREATE, h)
}

pub fn on_message_audited<H, Args>(h: H) -> &'static Bot
where
    H: Handler<Args>,
{
    default().on_message_audited(h);
    default()
}

/// Register a service on the default bot.
pub fn provide<T: Send + Sync + 'static>(svc: T) -> &'static Bot {
    default().provide(svc);
    default()
}

/// Register a pre-built `Arc<T>` service on the default bot.
pub fn provide_shared<T: Send + Sync + 'static>(svc: Arc<T>) -> &'static Bot {
    default().provide_shared(svc);
    default()
}

/// The default bot's REST client.
pub fn api() -> &'static crate::api::Client {
    default().api()
}

/// Run the default bot until Ctrl+C or a fatal error.
pub async fn run() -> Result<()> {
    default().run().await
}
