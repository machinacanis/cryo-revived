//! Bound reply builders: `c.say("hi").await` / `c.reply().text("hi").send().await`.

use std::future::{Future, IntoFuture};
use std::pin::Pin;

use crate::context::{
    C2CMessageCtx, EventCtx, FriendCtx, GroupBotCtx, GroupMemberCtx, GroupMessageCtx,
};
use crate::model::{Keyboard, MessageToCreate, PostMessageResult};
use crate::Result;

pub(crate) enum Target {
    User(String),
    Group(String),
}

/// Builds and sends a message bound to an event's chat target.
/// Obtain one from the typed contexts: [`C2CMessageCtx::reply`],
/// [`C2CMessageCtx::say`] & friends.
///
/// Sending is just `.await` (or the explicit [`ReplyBuilder::send`]):
///
/// ```no_run
/// # async fn f(c: cryo::C2CMessageCtx) -> cryo::Result<()> {
/// # let k = cryo::model::Keyboard::builder().build();
/// c.say("hi").await?;                        // passive reply
/// c.reply().text("hi").keyboard(k).await?;   // richer messages chain too
/// # Ok(())
/// # }
/// ```
#[must_use = "a reply is sent when awaited or via .send()"]
pub struct ReplyBuilder {
    ctx: EventCtx,
    target: Target,
    msg: MessageToCreate,
}

impl ReplyBuilder {
    pub(crate) fn new(ctx: &EventCtx, target: Target, msg: MessageToCreate) -> Self {
        ReplyBuilder {
            ctx: ctx.clone(),
            target,
            msg,
        }
    }

    /// Plain-text content (msg_type 0).
    pub fn text(mut self, s: impl Into<String>) -> Self {
        self.msg.text(s);
        self
    }

    /// Markdown content (msg_type 2).
    pub fn markdown(mut self, s: impl Into<String>) -> Self {
        self.msg.markdown(s);
        self
    }

    /// Attach an inline keyboard.
    pub fn keyboard(mut self, k: Keyboard) -> Self {
        self.msg.keyboard(k);
        self
    }

    /// Attach uploaded rich media (msg_type 7).
    pub fn file_info(mut self, info: impl Into<String>) -> Self {
        self.msg.file_info(info);
        self
    }

    /// Attach a tuwen link card (msg_type 8).
    pub fn card(
        mut self,
        title: impl Into<String>,
        desc: impl Into<String>,
        url: impl Into<String>,
        pic_url: impl Into<String>,
    ) -> Self {
        self.msg.tuwen_card(title, desc, url, pic_url);
        self
    }

    /// "typing..." indicator (c2c, msg_type 6).
    pub fn typing(mut self, secs: i32) -> Self {
        self.msg.typing(secs);
        self
    }

    /// Attach a message reference.
    pub fn quote(mut self, message_id: impl Into<String>) -> Self {
        self.msg.quote(message_id);
        self
    }

    /// Override `msg_seq` (default: auto-incremented per source message).
    pub fn seq(mut self, n: i64) -> Self {
        self.msg.seq(n);
        self
    }

    /// Deliver the built message, discarding the server response.
    pub async fn send(self) -> Result<()> {
        self.send_result().await.map(|_| ())
    }

    /// Deliver the message and return the response (id/timestamp/ext_info).
    pub async fn send_result(self) -> Result<PostMessageResult> {
        match &self.target {
            Target::User(id) => {
                self.ctx
                    .api()
                    .user(id.clone())
                    .send_message(&self.msg)
                    .await
            }
            Target::Group(id) => {
                self.ctx
                    .api()
                    .group(id.clone())
                    .send_message(&self.msg)
                    .await
            }
        }
    }
}

/// `.await` on a builder sends it: `c.say("hi").await`.
impl IntoFuture for ReplyBuilder {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

impl C2CMessageCtx {
    /// Passive reply to the user (`msg_id` + auto `msg_seq`).
    pub fn reply(&self) -> ReplyBuilder {
        let mut m = MessageToCreate::new();
        m.reply_to(self.message.id.clone())
            .seq(self.bot().next_seq(&self.message.id));
        ReplyBuilder::new(
            &self.event,
            Target::User(self.message.author.user_openid.clone()),
            m,
        )
    }

    /// Active (non-reply) message to this user.
    pub fn active(&self) -> ReplyBuilder {
        ReplyBuilder::new(
            &self.event,
            Target::User(self.message.author.user_openid.clone()),
            MessageToCreate::new(),
        )
    }

    /// Shortcut: passive text reply — `c.say("hi").await`.
    pub fn say(&self, s: impl Into<String>) -> ReplyBuilder {
        self.reply().text(s)
    }

    /// Shortcut: active text message — `c.push("hi").await`.
    pub fn push(&self, s: impl Into<String>) -> ReplyBuilder {
        self.active().text(s)
    }
}

impl GroupMessageCtx {
    /// Passive reply into the group (`msg_id` + auto `msg_seq`).
    pub fn reply(&self) -> ReplyBuilder {
        let mut m = MessageToCreate::new();
        m.reply_to(self.message.id.clone())
            .seq(self.bot().next_seq(&self.message.id));
        ReplyBuilder::new(
            &self.event,
            Target::Group(self.message.group_openid.clone()),
            m,
        )
    }

    /// Active (non-reply) message to this group.
    pub fn active(&self) -> ReplyBuilder {
        ReplyBuilder::new(
            &self.event,
            Target::Group(self.message.group_openid.clone()),
            MessageToCreate::new(),
        )
    }

    /// Shortcut: passive text reply — `c.say("hi").await`.
    pub fn say(&self, s: impl Into<String>) -> ReplyBuilder {
        self.reply().text(s)
    }

    /// Shortcut: active text message — `c.push("hi").await`.
    pub fn push(&self, s: impl Into<String>) -> ReplyBuilder {
        self.active().text(s)
    }
}

impl FriendCtx {
    /// Active message to the user (friend events carry no `msg_id`, so
    /// there is no passive reply).
    pub fn active(&self) -> ReplyBuilder {
        ReplyBuilder::new(
            &self.event,
            Target::User(self.event_data.openid.clone()),
            MessageToCreate::new(),
        )
    }

    /// Shortcut: text message to the user — `c.say("hi").await`.
    pub fn say(&self, s: impl Into<String>) -> ReplyBuilder {
        self.active().text(s)
    }
}

impl GroupBotCtx {
    /// Active message to the group (these events carry no `msg_id`).
    pub fn active(&self) -> ReplyBuilder {
        ReplyBuilder::new(
            &self.event,
            Target::Group(self.event_data.group_openid.clone()),
            MessageToCreate::new(),
        )
    }

    /// Shortcut: text message to the group — `c.say("hi").await`.
    pub fn say(&self, s: impl Into<String>) -> ReplyBuilder {
        self.active().text(s)
    }
}

impl GroupMemberCtx {
    /// Active message to the group (these events carry no `msg_id`).
    pub fn active(&self) -> ReplyBuilder {
        ReplyBuilder::new(
            &self.event,
            Target::Group(self.event_data.group_openid.clone()),
            MessageToCreate::new(),
        )
    }

    /// Shortcut: text message to the group — `c.say("hi").await`.
    pub fn say(&self, s: impl Into<String>) -> ReplyBuilder {
        self.active().text(s)
    }
}
