//! 基本示例：一个 bot，私聊 + 群@ + 好友事件 + 服务注入，全部走通。
//! 同时演示 handler 返回类型的三种写法——都不需要标注错误类型。
//!
//! ```text
//! QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx cargo run --example basic
//! ```

use cryo::model::Intent;
use cryo::{
    Bot, C2CMessageCtx, FriendCtx, GroupMessageCtx, Inject, PlainText, Result, SenderOpenId,
};

// ── 应用服务：provide 一次，handler 里 Inject<T> 拿 ──────────────────────
struct Greeter;

impl Greeter {
    fn hello(&self) -> &'static str {
        "你好，我是 cryo"
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let bot = Bot::from_env(Intent::BASIC);
    bot.provide(Greeter);

    // 形态 1 · 闭包直接返回 builder —— builder 实现了 IntoFuture，注册即发送，连 async 都不用写
    bot.on_c2c_message(|c: C2CMessageCtx, text: PlainText| c.say(format!("echo: {text}")));

    // 形态 2 · 函数体里有 `?` + 末尾 Ok(()) —— 错误类型固定为 cryo::Error，直接写 Ok(())
    bot.on_friend_add(
        |c: FriendCtx, who: SenderOpenId, g: Inject<Greeter>| async move {
            tracing::info!(who = %who, "friend added");
            c.say(g.hello()).await?;
            Ok(())
        },
    );

    // 形态 3 · 命名 async fn，签名声明 -> Result<()> —— 推荐写法，零标注噪音
    bot.on_group_at_message(on_group_at);

    bot.run().await
}

/// 命名 handler：签名里的 `-> Result<()>` 固定了错误类型，
/// 函数体内自由写 `?` 和 `Ok(())`，零标注噪音。
async fn on_group_at(c: GroupMessageCtx, text: PlainText) -> Result<()> {
    match text.as_ref() {
        "/ping" => c.say("pong").await,
        _ => c.say("收到 @ 消息").await,
    }
}
