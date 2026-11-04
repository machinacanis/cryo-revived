//! [`Fleet`] — run multiple bot accounts in one process.

use std::sync::Arc;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::bot::{Bot, Config};
use crate::handler::{self, Handler};
use crate::model::{event, Intent};
use crate::{Error, Result};

type Shared = Box<dyn Fn(&Bot) + Send + Sync>;

/// Runs N bot accounts: each bot keeps its own gateway session, api client
/// and handler set — but shared registrations apply once to every member
/// (including bots added later):
///
/// ```no_run
/// # use cryo::{Config, Fleet, model::Intent, C2CMessageCtx, PlainText};
/// # async fn reply_echo(c: C2CMessageCtx, text: PlainText) -> cryo::Result<()> {
/// #     c.say(format!("echo: {text}")).await
/// # }
/// # async fn f() {
/// let mut fleet = Fleet::new();
/// let b1 = fleet.add_env(Intent::BASIC);
/// fleet.add_env(Intent::BASIC);
///
/// fleet.on_c2c_message(reply_echo);      // applies to all bots
/// b1.on_c2c_message(reply_echo);         // only on b1
///
/// fleet.run().await;
/// # }
/// ```
///
/// Event contexts automatically belong to the receiving account:
/// `c.bot()` / `c.api()` always point at the right bot.
#[derive(Default)]
pub struct Fleet {
    bots: Vec<Bot>,
    shared: Vec<Shared>,
}

impl Fleet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a bot and join the fleet. Shared registrations made before
    /// or after this call apply to it.
    pub fn add(&mut self, cfg: Config) -> Bot {
        self.join(Bot::new(cfg))
    }

    /// Create a bot from `QQBOT_APP_ID` / `QQBOT_APP_SECRET` and join the fleet.
    ///
    /// # Panics
    /// If the env vars are unset, or `intents` is zero.
    pub fn add_env(&mut self, intents: Intent) -> Bot {
        self.join(Bot::from_env(intents))
    }

    /// Adopt an existing bot (shared registrations apply).
    pub fn join(&mut self, b: Bot) -> Bot {
        for f in &self.shared {
            f(&b);
        }
        self.bots.push(b.clone());
        b
    }

    /// Current members.
    pub fn bots(&self) -> &[Bot] {
        &self.bots
    }

    /// Apply `f` to all existing bots and remember it for future add/join.
    fn each(&mut self, f: impl Fn(&Bot) + Send + Sync + 'static) -> &mut Self {
        for b in &self.bots {
            f(b);
        }
        self.shared.push(Box::new(f));
        self
    }

    /// Register a service on every bot (the same `Arc<T>` is shared).
    pub fn provide<T: Send + Sync + 'static>(&mut self, svc: T) -> &mut Self {
        let svc = Arc::new(svc);
        self.each(move |b| {
            b.provide_shared(svc.clone());
        })
    }

    /// Register a handler for an event type on every bot.
    pub fn on<H, Args>(&mut self, event_type: &str, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        let e = handler::erase(h);
        let ev = event_type.to_string();
        self.each(move |b| {
            b.push_erased(&ev, e.clone());
        })
    }

    pub fn on_c2c_message<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::C2C_MESSAGE_CREATE, h)
    }
    pub fn on_group_at_message<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_AT_MESSAGE, h)
    }
    pub fn on_group_message<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_MESSAGE, h)
    }
    pub fn on_friend_add<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::FRIEND_ADD, h)
    }
    pub fn on_friend_del<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::FRIEND_DEL, h)
    }
    pub fn on_group_join<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_ADD_ROBOT, h)
    }
    pub fn on_group_leave<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_DEL_ROBOT, h)
    }
    pub fn on_group_member_add<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_MEMBER_ADD, h)
    }
    pub fn on_group_member_remove<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_MEMBER_REMOVE, h)
    }
    pub fn on_interaction<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        self.on(event::INTERACTION_CREATE, h)
    }
    pub fn on_message_audited<H, Args>(&mut self, h: H) -> &mut Self
    where
        H: Handler<Args>,
    {
        let e = handler::erase(h);
        let e2 = e.clone();
        self.each(move |b| {
            b.push_erased(event::MESSAGE_AUDIT_PASS, e.clone());
            b.push_erased(event::MESSAGE_AUDIT_REJECT, e2.clone());
        })
    }

    /// Start every bot session concurrently until Ctrl+C. The first fatal
    /// error cancels the rest and is returned.
    pub async fn run(&self) -> Result<()> {
        let token = CancellationToken::new();
        let t = token.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            t.cancel();
        });
        self.run_until(token).await
    }

    /// [`Fleet::run`] with a caller-controlled cancellation token.
    pub async fn run_until(&self, cancel: CancellationToken) -> Result<()> {
        let mut set = JoinSet::new();
        for b in &self.bots {
            let (b, t) = (b.clone(), cancel.clone());
            set.spawn(async move { b.run_until(t).await });
        }
        let mut first_err: Option<Error> = None;
        while let Some(r) = set.join_next().await {
            match r {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error!("bot session failed: {e}");
                    if first_err.is_none() {
                        first_err = Some(e);
                        cancel.cancel();
                    }
                }
                Err(je) => {
                    if first_err.is_none() {
                        first_err = Some(Error::msg(format!("task: {je}")));
                        cancel.cancel();
                    }
                }
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}
