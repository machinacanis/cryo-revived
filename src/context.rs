//! Handler context: the per-event [`EventCtx`] plus typed contexts like
//! [`C2CMessageCtx`] that decode `payload.d` up front.

use std::ops::Deref;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::api::Client;
use crate::model::{event, Payload};
use crate::{Bot, Error, Result};

/// Shared fields probed across event bodies for the extractor markers.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Probe {
    pub id: Option<String>,
    pub content: Option<String>,
    pub group_openid: Option<String>,
    pub openid: Option<String>,
    pub author: Option<Value>,
}

impl Probe {
    /// Best available sender openid: author.user_openid → member_openid →
    /// author.id → top-level openid (friend events).
    pub(crate) fn sender_id(&self) -> Option<String> {
        if let Some(a) = &self.author {
            for k in ["user_openid", "member_openid", "id"] {
                if let Some(v) = a[k].as_str().filter(|s| !s.is_empty()) {
                    return Some(v.to_string());
                }
            }
        }
        self.openid.clone()
    }
}

pub(crate) struct Inner {
    bot: Bot,
    payload: Payload,
}

/// The base handler context: bot/api access plus raw event payload.
/// All typed contexts dereference to it.
#[derive(Clone)]
pub struct EventCtx {
    pub(crate) inner: Arc<Inner>,
}

impl EventCtx {
    pub(crate) fn new(bot: Bot, payload: Payload) -> Self {
        EventCtx {
            inner: Arc::new(Inner { bot, payload }),
        }
    }

    /// The event name, e.g. `C2C_MESSAGE_CREATE`.
    pub fn event_type(&self) -> &str {
        &self.inner.payload.t
    }

    /// Payload-level event id — usable as `event_id` in passive replies.
    pub fn event_id(&self) -> &str {
        &self.inner.payload.id
    }

    /// Gateway sequence number of this payload.
    pub fn seq(&self) -> i64 {
        self.inner.payload.s
    }

    /// The bot instance that received this event.
    pub fn bot(&self) -> &Bot {
        &self.inner.bot
    }

    /// The REST client of the bot that received this event.
    pub fn api(&self) -> &Client {
        self.inner.bot.api()
    }

    /// Decode `payload.d` into a concrete event body.
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_value(self.inner.payload.d.clone())?)
    }

    /// The raw event body.
    pub fn raw(&self) -> &Value {
        &self.inner.payload.d
    }

    /// Fetch a service registered via [`Bot::provide`].
    ///
    /// ```no_run
    /// # async fn f(c: cryo::EventCtx) -> cryo::Result<()> {
    /// struct Store;
    /// let store = c.service::<Store>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn service<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.inner.bot.service::<T>()
    }

    pub(crate) fn probe(&self) -> Result<Probe> {
        Ok(serde_json::from_value(self.inner.payload.d.clone())?)
    }
}

macro_rules! typed_ctx {
    ($name:ident, $field:ident : $ty:ty, [$($ev:pat_param),+ $(,)?], $doc:literal) => {
        #[doc = $doc]
        ///
        /// Obtained by declaring it as a handler parameter — see [`crate::FromEvent`].
        #[derive(Clone)]
        pub struct $name {
            pub event: EventCtx,
            pub $field: $ty,
        }

        impl Deref for $name {
            type Target = EventCtx;
            fn deref(&self) -> &EventCtx {
                &self.event
            }
        }

        impl crate::extract::FromEvent for $name {
            fn from_event(ctx: &EventCtx) -> Result<Self> {
                match ctx.event_type() {
                    $($ev)|+ => Ok($name {
                        event: ctx.clone(),
                        $field: ctx.decode()?,
                    }),
                    t => Err(Error::msg(format!(
                        "event {t} cannot build {}",
                        stringify!($name)
                    ))),
                }
            }
        }
    };
}

typed_ctx!(
    C2CMessageCtx, message: crate::model::C2CMessage, [event::C2C_MESSAGE_CREATE],
    "Context of `C2C_MESSAGE_CREATE` (private-chat message)."
);
typed_ctx!(
    GroupMessageCtx, message: crate::model::GroupMessage,
    [event::GROUP_AT_MESSAGE, event::GROUP_MESSAGE],
    "Context of `GROUP_AT_MESSAGE_CREATE` / `GROUP_MESSAGE_CREATE`."
);
typed_ctx!(
    FriendCtx, event_data: crate::model::FriendEvent,
    [
        event::FRIEND_ADD,
        event::FRIEND_DEL,
        event::C2C_MSG_REJECT,
        event::C2C_MSG_RECEIVE
    ],
    "Context of `FRIEND_ADD` / `FRIEND_DEL` / `C2C_MSG_*`."
);
typed_ctx!(
    GroupBotCtx, event_data: crate::model::GroupBotEvent,
    [
        event::GROUP_ADD_ROBOT,
        event::GROUP_DEL_ROBOT,
        event::GROUP_MSG_REJECT,
        event::GROUP_MSG_RECEIVE
    ],
    "Context of `GROUP_ADD_ROBOT` / `GROUP_DEL_ROBOT` / `GROUP_MSG_*`."
);
typed_ctx!(
    GroupMemberCtx, event_data: crate::model::GroupMemberEvent,
    [
        event::GROUP_MEMBER_ADD,
        event::GROUP_MEMBER_REMOVE,
        event::GROUP_JOIN_REQUEST
    ],
    "Context of group member add/remove/join-request events."
);
typed_ctx!(
    InteractionCtx, interaction: crate::model::Interaction, [event::INTERACTION_CREATE],
    "Context of `INTERACTION_CREATE` (button taps, menu actions)."
);
typed_ctx!(
    MessageAuditCtx, event_data: crate::model::MessageAudited,
    [event::MESSAGE_AUDIT_PASS, event::MESSAGE_AUDIT_REJECT],
    "Context of `MESSAGE_AUDIT_PASS` / `MESSAGE_AUDIT_REJECT`."
);

impl C2CMessageCtx {
    /// The message's plain text (trimmed) — same as the [`crate::PlainText`]
    /// extractor, for handlers that only take the ctx.
    pub fn text(&self) -> &str {
        self.message.content.trim()
    }
}

impl GroupMessageCtx {
    /// The message's plain text (trimmed) — same as the [`crate::PlainText`]
    /// extractor, for handlers that only take the ctx.
    pub fn text(&self) -> &str {
        self.message.content.trim()
    }
}

impl InteractionCtx {
    /// Answer an interaction (code 0-5, see `Client::respond_interaction`).
    pub async fn respond(&self, code: i32) -> Result<()> {
        self.api()
            .respond_interaction(&self.interaction.id, code)
            .await
    }
}
