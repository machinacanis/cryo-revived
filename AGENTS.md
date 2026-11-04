# cryo — QQ Bot framework (Rust)

A developer-friendly QQ Bot framework prototype. Ported from Go
(baseline preserved in commit `8cf9d2e`).

## Hard constraints

- **Follow Tencent's official API design.** Naming and semantics track the
  official docs (distilled in-repo at `docs/api-spec/`, upstream
  <https://bot.q.qq.com/wiki/develop/api-v2/>): event names verbatim
  (`C2C_MESSAGE_CREATE`…), endpoint verbs (`send_message`, `recall_message`,
  `get_wss`…), and the official 被动回复(passive)/主动(active) message
  distinction. No invented abstractions that hide the official model.
- **No matcher-layer helpers** (decided 2026-xx, user instruction): no
  `on_command` / `CommandCtx` / rules / aliases / priority matchers.
  Handlers bind directly to official event types; keep only basic DX.
- **Explicit permissions per bot.** `Config` takes `Intent` as a required
  parameter (`Intent::BASIC` / `WITH_MEMBERS` / `ALL` / bit-or'ed flags).
  `Bot::new` panics on zero intents — no silent defaults, fleet members too.
- **No cross-bot message dedupe** — explicitly rejected; do not reintroduce.

## API surface rules

- Handlers: `Fn(args...) -> impl IntoFuture`, every arg `impl FromEvent`,
  awaited output `()` or `cryo::Result<()>`. Infallible closures may just
  return `()` — no `Ok(())` needed. A `ReplyBuilder` is `IntoFuture`, so
  one-liners need no `async move`: `|c: Ctx| c.say("hi")`.
- The handler error type is `cryo::Error` — a re-export of `anyhow::Error`.
  A single `Result` impl lets `Ok(())` be inferred, so a closure with
  `?` never needs `Ok::<(), cryo::Error>(())`; `?` also converts any
  `std::error::Error` via anyhow — no `From` impl needed anywhere.
- Control-flow errors are marker structs, not enum variants:
  `NoResume` / `FatalClose`, checked via `err.is::<T>()`.
- Multiple handlers per event = chained calls:
  `bot.on_c2c_message(a).on_c2c_message(b)` (executed in order).
- Event data injects by type: `PlainText`, `SenderOpenId`, `GroupOpenId`,
  `MessageId`, `EventId`, `model::User`, decoded event bodies, typed ctxs
  (`C2CMessageCtx`…), `Inject<T>` for `bot.provide` services, `Option<T>`
  for optional data. Custom data → `impl FromEvent`.
- Zero-noise send path: typed ctxs expose `say`/`push` (+`reply`/`active`
  builders) and `text()`; awaiting a `ReplyBuilder` sends it
  (`c.say("hi").await`), `.send()` stays available.
- Credentials: `Bot::from_env(intent)`, `cryo::init_env(intent)`,
  `Fleet::add_env(intent)`. API handles take `impl AsRef<str>`, so
  `api.user(&openid)` / `api.group(&gid)` need no `to_string`/`clone`.

## Layout

`src/model` wire types+builders · `src/api` REST client+endpoints ·
`src/gateway` WS session · `src/handler.rs` Handler/IntoOutcome ·
`src/extract.rs` FromEvent · `src/context.rs` ctxs · `src/reply.rs`
ReplyBuilder · `src/bot.rs` · `src/fleet.rs` · `src/default.rs` ·
`src/tests.rs` · `examples/*.rs`

## Build & test (toolchain lives in WSL)

```bash
wsl -- bash -c "cd /mnt/c/Users/machi/Desktop/cryo-revived && cargo fmt && cargo clippy --all-targets && cargo test"
```

Run an example: `QQBOT_APP_ID=.. QQBOT_APP_SECRET=.. cargo run --example minimal`

## Secrets

- `.env` (QQBOT_APP_ID/SECRET) — gitignored, never commit or paste into source.
