//! Core behavior tests: extractor semantics, erased handler calls,
//! explicit-intent enforcement and fleet replay.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;

use crate::extract::{FromEvent, Inject, PlainText, SenderOpenId};
use crate::model::{event, Intent, Payload};
use crate::{Bot, C2CMessageCtx, Config, EventCtx, Fleet};

fn payload(t: &str, d: serde_json::Value) -> Payload {
    Payload {
        op: crate::model::op::DISPATCH,
        t: t.to_string(),
        d,
        ..Default::default()
    }
}

fn c2c_payload() -> Payload {
    payload(
        event::C2C_MESSAGE_CREATE,
        json!({
            "id": "msg-1",
            "content": "  hello  ",
            "author": {"user_openid": "U123", "username": "machi"},
        }),
    )
}

fn ctx(t: &str, d: serde_json::Value) -> EventCtx {
    EventCtx::new(
        Bot::new(Config::new("id", "secret", Intent::BASIC)),
        payload(t, d),
    )
}

#[test]
#[should_panic(expected = "intents must be declared explicitly")]
fn zero_intents_panics() {
    let _ = Bot::new(Config::new("id", "secret", Intent(0)));
}

#[test]
fn plaintext_and_sender_extract() {
    let c = ctx(
        event::C2C_MESSAGE_CREATE,
        json!({"id": "m1", "content": " hi ", "author": {"user_openid": "U1"}}),
    );
    assert_eq!(&*PlainText::from_event(&c).unwrap(), "hi");
    assert_eq!(&*SenderOpenId::from_event(&c).unwrap(), "U1");
}

#[test]
fn typed_ctx_rejects_wrong_event() {
    let c = ctx(event::FRIEND_ADD, json!({"openid": "U9"}));
    assert!(C2CMessageCtx::from_event(&c).is_err());
    // SenderOpenId still works on friend events (top-level openid).
    assert_eq!(&*SenderOpenId::from_event(&c).unwrap(), "U9");
}

#[test]
fn provide_and_inject() {
    struct Store(&'static str);
    let bot = Bot::new(Config::new("id", "secret", Intent::BASIC));
    bot.provide(Store("hello"));
    let c = EventCtx::new(
        bot.clone(),
        payload(
            event::C2C_MESSAGE_CREATE,
            json!({"author": {"user_openid": "U"}}),
        ),
    );
    let s = Inject::<Store>::from_event(&c).unwrap();
    assert_eq!(s.0 .0, "hello");
    assert!(bot.service::<u64>().is_err());
}

#[tokio::test]
async fn erased_handler_runs_in_order() {
    static HITS: AtomicUsize = AtomicUsize::new(0);
    let bot = Bot::new(Config::new("id", "secret", Intent::BASIC));
    bot.on_c2c_message(|_c: C2CMessageCtx, text: PlainText| async move {
        assert_eq!(&*text, "hello");
        HITS.fetch_add(1, Ordering::SeqCst);
        Ok::<(), crate::Error>(())
    });
    bot.dispatch_for_test(c2c_payload());
    tokio::task::yield_now().await;
    assert_eq!(HITS.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fleet_replays_shared_handlers() {
    static HITS: AtomicUsize = AtomicUsize::new(0);
    let mut fleet = Fleet::new();
    fleet.on_c2c_message(|_c: C2CMessageCtx| async move {
        HITS.fetch_add(1, Ordering::SeqCst);
        Ok::<(), crate::Error>(())
    });
    let b1 = fleet.add(Config::new("id", "secret", Intent::BASIC));
    let b2 = fleet.add(Config::new("id", "secret", Intent::BASIC));
    assert_eq!(fleet.bots().len(), 2);
    b1.dispatch_for_test(c2c_payload());
    b2.dispatch_for_test(c2c_payload());
    for _ in 0..20 {
        if HITS.load(Ordering::SeqCst) >= 2 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(HITS.load(Ordering::SeqCst), 2);
}

#[test]
fn shared_service_reaches_fleet_bots() {
    struct Store(Arc<AtomicUsize>);
    let mut fleet = Fleet::new();
    fleet.provide(Store(Arc::new(AtomicUsize::new(7))));
    let b = fleet.add(Config::new("id", "secret", Intent::BASIC));
    let s = b.service::<Store>().unwrap();
    assert_eq!(s.0.load(Ordering::SeqCst), 7);
}
