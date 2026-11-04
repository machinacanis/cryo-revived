//! Echo demo：回复私聊和群@消息，展示服务注入 + 消息构造器。
//!
//! ```text
//! QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx cargo run --example echo
//! ```

use cryo::model::{Button, Intent, Keyboard};
use cryo::{
    Bot, C2CMessageCtx, Config, FriendCtx, GroupBotCtx, GroupMessageCtx, Inject, InteractionCtx,
};

/// 可注入的应用服务。
struct Greeter {
    name: &'static str,
}

impl Greeter {
    fn hello(&self) -> String {
        format!("你好，我是 {}", self.name)
    }
}

#[tokio::main]
async fn main() -> cryo::Result<()> {
    tracing_subscriber::fmt::init();

    let bot = Bot::new(Config::from_env(Intent::WITH_MEMBERS));

    // DI：注册服务实例，handler 用 Inject<T> 声明注入。
    bot.provide(Greeter { name: "cryo" });

    // 类型化 handler：c.message 全字段补全 + 链式回复。
    bot.on_c2c_message(|c: C2CMessageCtx| async move {
        let text = c.message.content.trim();
        match text {
            "/ping" => c.reply().text("pong").send().await,
            "/md" => {
                c.reply()
                    .markdown("## cryo\n**markdown** reply works")
                    .keyboard(
                        Keyboard::builder()
                            .row([Button::new("点我").style(1).callback("btn-1").confirm(
                                "确定？",
                                "好",
                                "算了",
                            )])
                            .build(),
                    )
                    .send()
                    .await
            }
            _ => c.reply().text(format!("echo: {text}")).send().await,
        }
    });

    bot.on_group_at_message(|c: GroupMessageCtx| async move {
        let who = &c.message.author.member_openid[..8.min(c.message.author.member_openid.len())];
        c.reply().text(format!("收到 @{who} 的消息")).send().await
    });

    // 服务注入：声明 Inject<Greeter> 参数即可拿到。
    bot.on_friend_add(|c: FriendCtx, g: Inject<Greeter>| async move {
        c.api()
            .user(c.event_data.openid.clone())
            .send_text(g.hello())
            .await?;
        Ok::<(), cryo::Error>(())
    });

    bot.on_group_join(|c: GroupBotCtx| async move {
        c.api()
            .group(c.event_data.group_openid.clone())
            .send_text("感谢拉我进群，发送 /ping 试试")
            .await?;
        Ok::<(), cryo::Error>(())
    });

    bot.on_interaction(|c: InteractionCtx| async move { c.respond(0).await });

    println!("bot running, ctrl+c to stop");
    bot.run().await
}
