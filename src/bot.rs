//! [`Bot`] — the application entry point.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::api::Client;
use crate::context::EventCtx;
use crate::gateway::Session;
use crate::handler::{self, Erased, Handler};
use crate::model::{event, op, Intent, Payload};
use crate::{Error, Result};

/// Configuration of a [`Bot`].
#[derive(Debug, Clone)]
pub struct Config {
    pub app_id: String,
    pub app_secret: String,
    /// 本 bot 订阅的事件权限（如 [`Intent::BASIC`]）。必须显式声明——
    /// [`Config::new`] / [`Config::from_env`] 把它作为必填参数，没有静默默认。
    pub intents: Intent,
    /// `[id, num]`; zero value means `[0, 1]`.
    pub shard: [u32; 2],
}

impl Config {
    /// `intents` is a required parameter: every bot account declares its
    /// usable permissions explicitly.
    pub fn new(app_id: impl Into<String>, app_secret: impl Into<String>, intents: Intent) -> Self {
        Config {
            app_id: app_id.into(),
            app_secret: app_secret.into(),
            intents,
            shard: [0, 1],
        }
    }

    /// Credentials from `QQBOT_APP_ID` / `QQBOT_APP_SECRET`; `intents` still
    /// must be declared explicitly.
    ///
    /// # Panics
    /// If the env vars are unset — credentials are mandatory.
    pub fn from_env(intents: Intent) -> Self {
        let app_id = std::env::var("QQBOT_APP_ID")
            .expect("cryo: set QQBOT_APP_ID or pass credentials to Config::new");
        let app_secret = std::env::var("QQBOT_APP_SECRET")
            .expect("cryo: set QQBOT_APP_SECRET or pass credentials to Config::new");
        Config::new(app_id, app_secret, intents)
    }
}

pub(crate) struct BotInner {
    cfg: Config,
    api: Client,
    handlers: RwLock<HashMap<String, Vec<Erased>>>,
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    session_id: Mutex<Option<String>>,
    reply_seq: Mutex<HashMap<String, u64>>,
}

/// The application entry point. Cheap to clone — clones share everything
/// (handler registry, services, api client, session state).
///
/// ```no_run
/// # use cryo::{Bot, Config, model::Intent, C2CMessageCtx};
/// let bot = Bot::new(Config::new("id", "secret", Intent::BASIC));
/// bot.on_c2c_message(|c: C2CMessageCtx| async move {
///     c.say("hi").await
/// });
/// ```
#[derive(Clone)]
pub struct Bot {
    pub(crate) inner: Arc<BotInner>,
}

impl Bot {
    /// Build a bot.
    ///
    /// # Panics
    /// If `cfg.intents` is zero — permissions must be declared explicitly.
    pub fn new(cfg: Config) -> Bot {
        if cfg.intents == Intent(0) {
            panic!(
                "cryo: Config.intents must be declared explicitly \
                 (e.g. Intent::BASIC) — no silent permission defaults"
            );
        }
        Bot {
            inner: Arc::new(BotInner {
                api: Client::new(cfg.app_id.clone(), cfg.app_secret.clone()),
                cfg,
                handlers: RwLock::new(HashMap::new()),
                services: RwLock::new(HashMap::new()),
                session_id: Mutex::new(None),
                reply_seq: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Build a bot from `QQBOT_APP_ID` / `QQBOT_APP_SECRET`.
    ///
    /// # Panics
    /// If the env vars are unset, or `intents` is zero — permissions must be
    /// declared explicitly.
    pub fn from_env(intents: Intent) -> Bot {
        Bot::new(Config::from_env(intents))
    }

    /// The raw REST client: `bot.api().user(openid).send_message(&msg).await`.
    pub fn api(&self) -> &Client {
        &self.inner.api
    }

    /// This bot account's app id — handy in fleets to tell bots apart.
    pub fn app_id(&self) -> &str {
        &self.inner.cfg.app_id
    }

    /// The current gateway session id (set after READY).
    pub fn session_id(&self) -> Option<String> {
        self.inner.session_id.lock().unwrap().clone()
    }

    /// Register an application service; handlers take it as `Inject<T>`.
    ///
    /// ```no_run
    /// # use cryo::{Bot, Inject, C2CMessageCtx};
    /// # struct Store;
    /// # let bot = Bot::from_env(cryo::model::Intent::BASIC);
    /// bot.provide(Store);
    /// bot.on_c2c_message(|c: C2CMessageCtx, s: Inject<Store>| async move {
    ///     c.say("ok").await
    /// });
    /// ```
    pub fn provide<T: Send + Sync + 'static>(&self, svc: T) -> &Self {
        self.provide_shared(Arc::new(svc))
    }

    /// Register a pre-built `Arc<T>` service (shared with other bots/owners).
    pub fn provide_shared<T: Send + Sync + 'static>(&self, svc: Arc<T>) -> &Self {
        self.inner
            .services
            .write()
            .unwrap()
            .insert(TypeId::of::<T>(), svc);
        self
    }

    /// Fetch a registered service.
    pub fn service<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.inner
            .services
            .read()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|a| a.clone().downcast::<T>().ok())
            .ok_or(Error::NoService(std::any::type_name::<T>()))
    }

    // --- event registration ---

    /// Register a handler for an event type name ([`crate::model::event`])
    /// or `"*"` for all events. Any fn/closure whose parameters implement
    /// [`crate::FromEvent`] qualifies — registration order = execution order.
    ///
    /// ```no_run
    /// # use cryo::{Bot, EventCtx, model::{event, C2CMessage}};
    /// # let bot = Bot::from_env(cryo::model::Intent::BASIC);
    /// bot.on(event::C2C_MESSAGE_CREATE, |c: EventCtx, m: C2CMessage| async move {
    ///     Ok(())
    /// });
    /// ```
    pub fn on<H, Args>(&self, event_type: &str, handler: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.push_erased(event_type, handler::erase(handler))
    }

    pub(crate) fn push_erased(&self, event_type: &str, h: Erased) -> &Self {
        self.inner
            .handlers
            .write()
            .unwrap()
            .entry(event_type.to_string())
            .or_default()
            .push(h);
        self
    }

    /// Incoming c2c (private chat) messages.
    pub fn on_c2c_message<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::C2C_MESSAGE_CREATE, h)
    }

    /// Messages @-mentioning the bot in a group.
    pub fn on_group_at_message<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_AT_MESSAGE, h)
    }

    /// Every group message (full-message mode, needs the permission).
    pub fn on_group_message<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_MESSAGE, h)
    }

    /// A user added the bot as friend.
    pub fn on_friend_add<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::FRIEND_ADD, h)
    }

    /// A user removed the bot.
    pub fn on_friend_del<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::FRIEND_DEL, h)
    }

    /// The bot was added to a group.
    pub fn on_group_join<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_ADD_ROBOT, h)
    }

    /// The bot was removed from a group.
    pub fn on_group_leave<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_DEL_ROBOT, h)
    }

    /// A member joined a group (needs `GROUP_MEMBER_EVENT` intent).
    pub fn on_group_member_add<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_MEMBER_ADD, h)
    }

    /// A member left a group.
    pub fn on_group_member_remove<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::GROUP_MEMBER_REMOVE, h)
    }

    /// `INTERACTION_CREATE` — button taps, menu actions.
    /// For type 11/12 the bot must answer via `c.respond(code)`.
    pub fn on_interaction<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        self.on(event::INTERACTION_CREATE, h)
    }

    /// Active-message audit results (intent `MESSAGE_AUDIT`).
    pub fn on_message_audited<H, Args>(&self, h: H) -> &Self
    where
        H: Handler<Args>,
    {
        let e = handler::erase(h);
        self.push_erased(event::MESSAGE_AUDIT_PASS, e.clone());
        self.push_erased(event::MESSAGE_AUDIT_REJECT, e)
    }

    // --- lifecycle ---

    /// Run the gateway session until Ctrl+C or a fatal error.
    pub async fn run(&self) -> Result<()> {
        let token = CancellationToken::new();
        let t = token.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            t.cancel();
        });
        self.run_until(token).await
    }

    /// Run the gateway session until `cancel` fires or a fatal error.
    pub async fn run_until(&self, cancel: CancellationToken) -> Result<()> {
        let bot = self.clone();
        let bot2 = self.clone();
        let session = Session::new(
            self.inner.api.clone(),
            self.inner.cfg.intents,
            self.inner.cfg.shard,
        )
        .on_event(move |p| bot.dispatch(p))
        .on_session(move |sid, user| {
            bot2.inner
                .session_id
                .lock()
                .unwrap()
                .replace(sid.to_string());
            info!(
                session = sid,
                bot = user.map(|u| u.username.as_str()).unwrap_or("?"),
                "gateway ready"
            );
        });
        session.run(cancel).await
    }

    /// Decode and route one gateway payload.
    fn dispatch(&self, p: Payload) {
        self.dispatch_inner(p)
    }

    /// Test-visible entry for [`Bot::dispatch`].
    #[cfg(test)]
    pub(crate) fn dispatch_for_test(&self, p: Payload) {
        self.dispatch_inner(p)
    }

    fn dispatch_inner(&self, p: Payload) {
        if p.op != op::DISPATCH || p.t == event::READY || p.t == event::RESUMED {
            return;
        }
        let handlers: Vec<Erased> = {
            let map = self.inner.handlers.read().unwrap();
            map.get(&p.t)
                .into_iter()
                .chain(map.get("*"))
                .flatten()
                .cloned()
                .collect()
        };
        if handlers.is_empty() {
            return;
        }
        let ctx = EventCtx::new(self.clone(), p);
        let event_type = ctx.event_type().to_string();
        tokio::spawn(async move {
            for h in handlers {
                if let Err(e) = h(&ctx).await {
                    error!(event = %event_type, "handler error: {e}");
                }
            }
        });
    }

    /// Per-source-message reply counter so repeated replies to the same
    /// message don't collide on `msg_id + msg_seq`.
    pub(crate) fn next_seq(&self, msg_id: &str) -> i64 {
        let mut map = self.inner.reply_seq.lock().unwrap();
        let n = map.entry(msg_id.to_string()).or_insert(0);
        *n += 1;
        *n as i64
    }
}
