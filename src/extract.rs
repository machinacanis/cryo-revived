//! Handler parameter injection — axum-style extractors.
//!
//! Anything implementing [`FromEvent`] can be declared as a handler
//! parameter and is produced from the current event on demand:
//!
//! ```no_run
//! # use cryo::{Bot, C2CMessageCtx, PlainText, SenderOpenId};
//! # let bot = Bot::from_env(cryo::model::Intent::BASIC);
//! bot.on_c2c_message(|c: C2CMessageCtx, from: SenderOpenId, text: PlainText| async move {
//!     // from = 发送者 openid, text = 消息纯文本
//!     c.say(format!("{from}: {text}")).await
//! });
//! ```
//!
//! Custom event data = implement `FromEvent` on your own type.
//! Application services = [`Inject<T>`] over [`crate::Bot::provide`].

use std::ops::Deref;
use std::sync::Arc;

use crate::api::Client;
use crate::context::EventCtx;
use crate::model::{event, User};
use crate::{Bot, Error, Result};

/// Types that can be extracted from an incoming event — every handler
/// parameter must implement this.
pub trait FromEvent: Sized + Send + 'static {
    fn from_event(ctx: &EventCtx) -> Result<Self>;
}

impl FromEvent for EventCtx {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        Ok(ctx.clone())
    }
}

impl FromEvent for Client {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        Ok(ctx.api().clone())
    }
}

impl FromEvent for Bot {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        Ok(ctx.bot().clone())
    }
}

/// `Option<T>` extracts `None` instead of failing — e.g. `Option<GroupOpenId>`
/// is `Some` only in group-scoped events.
impl<T: FromEvent> FromEvent for Option<T> {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        Ok(T::from_event(ctx).ok())
    }
}

macro_rules! marker {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub String);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<$name> for String {
            fn from(v: $name) -> String {
                v.0
            }
        }
    };
}

marker!(PlainText, "The message's plain text content (trimmed).");
marker!(
    SenderOpenId,
    "The sender's openid — user_openid in c2c, member_openid in group, openid in friend events."
);
marker!(GroupOpenId, "The group_openid of group-scoped events.");
marker!(
    MessageId,
    "The message id (`d.id`) of message events — usable as reply `msg_id`."
);
marker!(
    EventId,
    "The payload-level event id — usable as reply `event_id`."
);

impl FromEvent for PlainText {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        Ok(PlainText(
            ctx.probe()?.content.unwrap_or_default().trim().to_string(),
        ))
    }
}

impl FromEvent for SenderOpenId {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        ctx.probe()?.sender_id().map(SenderOpenId).ok_or_else(|| {
            Error::Extract(format!("event {} has no sender openid", ctx.event_type()))
        })
    }
}

impl FromEvent for GroupOpenId {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        ctx.probe()?.group_openid.map(GroupOpenId).ok_or_else(|| {
            Error::Extract(format!("event {} is not group-scoped", ctx.event_type()))
        })
    }
}

impl FromEvent for MessageId {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        ctx.probe()?
            .id
            .map(MessageId)
            .ok_or_else(|| Error::Extract(format!("event {} has no message id", ctx.event_type())))
    }
}

impl FromEvent for EventId {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        Ok(EventId(ctx.event_id().to_string()))
    }
}

/// Decoded event bodies usable as handler parameters — the event must match.
macro_rules! event_body {
    ($ty:ty, [$($ev:pat_param),+ $(,)?]) => {
        impl FromEvent for $ty {
            fn from_event(ctx: &EventCtx) -> Result<Self> {
                match ctx.event_type() {
                    $($ev)|+ => ctx.decode(),
                    t => Err(Error::Extract(format!(
                        "event {t} doesn't carry {}",
                        stringify!($ty)
                    ))),
                }
            }
        }
    };
}

event_body!(crate::model::C2CMessage, [event::C2C_MESSAGE_CREATE]);
event_body!(
    crate::model::GroupMessage,
    [event::GROUP_AT_MESSAGE, event::GROUP_MESSAGE]
);
event_body!(
    crate::model::FriendEvent,
    [
        event::FRIEND_ADD,
        event::FRIEND_DEL,
        event::C2C_MSG_REJECT,
        event::C2C_MSG_RECEIVE
    ]
);
event_body!(
    crate::model::GroupBotEvent,
    [
        event::GROUP_ADD_ROBOT,
        event::GROUP_DEL_ROBOT,
        event::GROUP_MSG_REJECT,
        event::GROUP_MSG_RECEIVE
    ]
);
event_body!(
    crate::model::GroupMemberEvent,
    [
        event::GROUP_MEMBER_ADD,
        event::GROUP_MEMBER_REMOVE,
        event::GROUP_JOIN_REQUEST
    ]
);
event_body!(crate::model::Interaction, [event::INTERACTION_CREATE]);
event_body!(
    crate::model::MessageAudited,
    [event::MESSAGE_AUDIT_PASS, event::MESSAGE_AUDIT_REJECT]
);

/// The message sender / event subject (decodes `d.author`).
impl FromEvent for User {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        match ctx.probe()?.author {
            Some(v) => Ok(serde_json::from_value(v)?),
            None => Err(Error::Extract(format!(
                "event {} has no author",
                ctx.event_type()
            ))),
        }
    }
}

/// Injects an application service registered via [`Bot::provide`]:
///
/// ```no_run
/// # use cryo::{Bot, C2CMessageCtx, Inject};
/// struct Store;
/// # let bot = Bot::from_env(cryo::model::Intent::BASIC);
/// bot.provide(Store);
/// bot.on_c2c_message(|c: C2CMessageCtx, store: Inject<Store>| async move {
///     let _store: &Store = &store;
///     c.say("ok").await
/// });
/// ```
pub struct Inject<T: Send + Sync + 'static>(pub Arc<T>);

impl<T: Send + Sync + 'static> Clone for Inject<T> {
    fn clone(&self) -> Self {
        Inject(self.0.clone())
    }
}

impl<T: Send + Sync + 'static> Deref for Inject<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Send + Sync + 'static> FromEvent for Inject<T> {
    fn from_event(ctx: &EventCtx) -> Result<Self> {
        ctx.service::<T>().map(Inject)
    }
}

// Typed contexts (C2CMessageCtx etc.) implement FromEvent in context.rs,
// where the decode target lives next to the type.
