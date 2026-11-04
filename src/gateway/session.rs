//! Websocket event session: Hello → Identify/Resume → heartbeat → dispatch.

use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::api::Client;
use crate::error::gateway;
use crate::model::{event, op, Intent, Payload, User};
use crate::{Error, FatalClose, NoResume, Result};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WriteHalf = futures_util::stream::SplitSink<Ws, Message>;
type OnEvent = Box<dyn Fn(Payload) + Send + Sync>;
type OnSession = Box<dyn Fn(&str, Option<&User>) + Send + Sync>;

#[derive(Default)]
struct State {
    session_id: String,
    seq: i64,
}

/// One sharded gateway connection with automatic resume/reconnect.
pub struct Session {
    api: Client,
    intents: Intent,
    shard: [u32; 2],
    on_event: OnEvent,
    on_session: OnSession,
    state: Arc<Mutex<State>>,
}

impl Session {
    pub fn new(api: Client, intents: Intent, shard: [u32; 2]) -> Self {
        Session {
            api,
            intents,
            shard,
            on_event: Box::new(|_| {}),
            on_session: Box::new(|_, _| {}),
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    /// Called for every incoming payload (including READY/RESUMED).
    pub fn on_event(mut self, f: impl Fn(Payload) + Send + Sync + 'static) -> Self {
        self.on_event = Box::new(f);
        self
    }

    /// Called after READY with the session id and the bot's user object.
    pub fn on_session(mut self, f: impl Fn(&str, Option<&User>) + Send + Sync + 'static) -> Self {
        self.on_session = Box::new(f);
        self
    }

    /// Connect and serve events until `cancel` fires or a fatal error occurs.
    /// Recoverable disconnects resume automatically.
    pub async fn run(&self, cancel: CancellationToken) -> Result<()> {
        info!("gateway: resolving entry");
        let url = self.api.get_wss().await?;
        loop {
            match self.serve(&url, &cancel).await {
                Ok(()) => return Ok(()),
                Err(_) if cancel.is_cancelled() => return Ok(()),
                Err(e) if e.is::<NoResume>() => {
                    warn!("gateway: session invalidated, re-identify");
                    self.state.lock().unwrap().session_id.clear();
                    self.state.lock().unwrap().seq = 0;
                    continue;
                }
                Err(e) if e.is::<FatalClose>() => return Err(e),
                Err(e) => {
                    warn!("gateway: reconnecting: {e}");
                    tokio::select! {
                        _ = cancel.cancelled() => return Ok(()),
                        _ = tokio::time::sleep(std::time::Duration::from_secs(3)) => continue,
                    }
                }
            }
        }
    }

    /// One connection lifecycle: hello → auth → read loop.
    async fn serve(&self, url: &str, cancel: &CancellationToken) -> Result<()> {
        let (ws, _) = connect_async(url).await?;
        let (write, mut read) = ws.split();
        let write = Arc::new(tokio::sync::Mutex::new(write));

        // Hello
        let hello = read
            .next()
            .await
            .ok_or_else(|| gateway("closed before hello"))??;
        let hello = parse(&hello)?;
        if hello.op != op::HELLO {
            return Err(gateway(format!("expect hello(op10), got op={}", hello.op)));
        }
        let interval = hello.d["heartbeat_interval"].as_u64().unwrap_or(45_000);
        let interval = std::time::Duration::from_millis(interval.max(1_000));

        // Identify or Resume
        let session_id = self.state.lock().unwrap().session_id.clone();
        if session_id.is_empty() {
            self.send_identify(&write).await?;
        } else {
            self.send_resume(&write, &session_id).await?;
        }

        // Heartbeat
        let hb = {
            let write = write.clone();
            let state = self.state.clone();
            let cancel = cancel.child_token();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(interval);
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => return,
                        _ = tick.tick() => {
                            let seq = state.lock().unwrap().seq;
                            let d = if seq == 0 { json!(null) } else { json!(seq) };
                            let p = Payload { op: op::HEARTBEAT, d, ..Default::default() };
                            let mut w = write.lock().await;
                            if w.send(Message::Text(serde_json::to_string(&p).unwrap().into())).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            })
        };

        let result = self.read_loop(&mut read, cancel).await;
        hb.abort();
        result
    }

    async fn send_identify(&self, write: &Write) -> Result<()> {
        let tok = self.api.token().await?;
        let p = Payload {
            op: op::IDENTIFY,
            d: json!({
                "token": format!("QQBot {tok}"),
                "intents": self.intents.0,
                "shard": self.shard,
                "properties": {"$os": std::env::consts::OS, "$browser": "cryo", "$device": "cryo"},
            }),
            ..Default::default()
        };
        send(write, &p).await
    }

    async fn send_resume(&self, write: &Write, session_id: &str) -> Result<()> {
        let tok = self.api.token().await?;
        let seq = self.state.lock().unwrap().seq;
        let p = Payload {
            op: op::RESUME,
            d: json!({
                "token": format!("QQBot {tok}"),
                "session_id": session_id,
                "seq": seq,
            }),
            ..Default::default()
        };
        send(write, &p).await
    }

    async fn read_loop(
        &self,
        read: &mut futures_util::stream::SplitStream<Ws>,
        cancel: &CancellationToken,
    ) -> Result<()> {
        loop {
            let msg = tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                msg = read.next() => msg,
            };
            let msg = match msg {
                Some(m) => m,
                None => return Err(gateway("connection closed")),
            };
            match msg {
                Ok(Message::Text(text)) => match serde_json::from_str::<Payload>(&text) {
                    Ok(p) => self.handle(p),
                    Err(e) => debug!("gateway: undecodable frame: {e}"),
                },
                Ok(Message::Close(f)) => {
                    let (code, text) = f
                        .map(|c| (u16::from(c.code), c.reason.to_string()))
                        .unwrap_or((1000, String::new()));
                    return Err(classify_close(code, &text));
                }
                Ok(_) => {}
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn handle(&self, p: Payload) {
        if p.s > 0 {
            self.state.lock().unwrap().seq = p.s;
        }
        match p.op {
            op::DISPATCH if p.t == event::READY => {
                let sid = p.d["session_id"].as_str().unwrap_or_default().to_string();
                let user: Option<User> = serde_json::from_value(p.d["user"].clone()).ok();
                self.state.lock().unwrap().session_id = sid.clone();
                (self.on_session)(&sid, user.as_ref());
            }
            op::RECONNECT => debug!("gateway: server asked to reconnect"),
            op::INVALID_SESSION => warn!("gateway: invalid session"),
            _ => {}
        }
        (self.on_event)(p);
    }
}

type Write = Arc<tokio::sync::Mutex<WriteHalf>>;

async fn send(write: &Write, p: &Payload) -> Result<()> {
    write
        .lock()
        .await
        .send(Message::Text(serde_json::to_string(p)?.into()))
        .await?;
    Ok(())
}

fn parse(m: &Message) -> Result<Payload> {
    match m {
        Message::Text(t) => Ok(serde_json::from_str(t)?),
        other => Err(gateway(format!("expect text frame, got {other:?}"))),
    }
}

/// Maps gateway close codes to resumable/re-identify/fatal outcomes.
fn classify_close(code: u16, text: &str) -> Error {
    match code {
        // session timeout / invalid seq — resume is still possible
        4008 | 4009 => gateway(format!("close {code} (resumable): {text}")),
        // must re-identify
        4006 | 4007 | 4900..=4913 => NoResume.into(),
        // banned / offline — do not retry
        4914 | 4915 => FatalClose(code, text.to_string()).into(),
        _ => gateway(format!("close {code}: {text}")),
    }
}
