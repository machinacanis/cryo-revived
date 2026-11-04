//! 基本示例：一个 bot，私聊 + 群@ + 好友事件 + 服务注入，全部走通。
//! 同时演示 handler 返回类型的三种写法——`Ok(())` 推断差异就在其中。
//!
//! ```text
//! QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx cargo run --example basic
//! ```

use cryo::model::Intent;
use cryo::{
    Bot, C2CMessageCtx, Config, FriendCtx, GroupMessageCtx, Inject, PlainText, Result, SenderOpenId,
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
    let bot = Bot::new(Config::from_env(Intent::BASIC));
    bot.provide(Greeter);

    // 形态 1 · 闭包尾表达式直接返回 send() —— E 被固定为 cryo::Error，无需标注
    bot.on_c2c_message(|c: C2CMessageCtx, text: PlainText| async move {
        c.reply().text(format!("echo: {text}")).send().await
    });

    // 形态 2 · 函数体里有 `?` + 末尾 Ok(()) —— E 推断不出，必须标注
    //
    //   写成 `Ok(())` 会报错：
    //   error[E0282]: type annotations needed
    //     --> cannot infer type of the type parameter `E`
    //
    bot.on_friend_add(
        |c: FriendCtx, who: SenderOpenId, g: Inject<Greeter>| async move {
            c.api().user(who.to_string()).send_text(g.hello()).await?;
            Ok::<(), cryo::Error>(()) // ← 差异点：必须写全
        },
    );

    // 形态 3 · 命名 async fn，签名声明 -> Result<()> —— 推荐写法，无需标注
    bot.on_group_at_message(on_group_at);

    bot.run().await
}

/// 命名 handler：签名里的 `-> Result<()>` 固定了错误类型，
/// 函数体内自由写 `?` 和 `Ok(())`，零标注噪音。
async fn on_group_at(c: GroupMessageCtx, text: PlainText) -> Result<()> {
    match text.as_ref() {
        "/ping" => c.reply().text("pong").send().await,
        _ => c.reply().text("收到 @ 消息").send().await,
    }
}
