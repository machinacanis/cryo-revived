//! Echo demo：回复私聊和群@消息，展示服务注入 + 消息构造器。
//!
//! ```text
//! QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx cargo run --example echo
//! ```

use cryo::model::{Button, Intent, Keyboard};
use cryo::{Bot, C2CMessageCtx, FriendCtx, GroupBotCtx, GroupMessageCtx, Inject, InteractionCtx};

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

    let bot = Bot::from_env(Intent::WITH_MEMBERS);

    // DI：注册服务实例，handler 用 Inject<T> 声明注入。
    bot.provide(Greeter { name: "cryo" });

    // 类型化 handler：c.text() 取纯文本，c.say() 直接回。
    bot.on_c2c_message(|c: C2CMessageCtx| async move {
        match c.text() {
            "/ping" => c.say("pong").await,
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
                    .await
            }
            text => c.say(format!("echo: {text}")).await,
        }
    });

    bot.on_group_at_message(|c: GroupMessageCtx| async move {
        let who = &c.message.author.member_openid[..8.min(c.message.author.member_openid.len())];
        c.say(format!("收到 @{who} 的消息")).await
    });

    // 服务注入：声明 Inject<Greeter> 参数即可拿到；好友事件无 msg_id，say 即主动消息。
    // 单条表达式时连 async move 都不用写——ReplyBuilder 实现了 IntoFuture，注册即发送。
    bot.on_friend_add(|c: FriendCtx, g: Inject<Greeter>| c.say(g.hello()));

    bot.on_group_join(|c: GroupBotCtx| c.say("感谢拉我进群，发送 /ping 试试"));

    bot.on_interaction(|c: InteractionCtx| async move { c.respond(0).await });

    println!("bot running, ctrl+c to stop");
    bot.run().await
}
