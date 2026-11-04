//! 多 bot 账号示例：一个进程跑 N 个机器人，handler 注册一次全群生效。
//! 凭证：QQBOT_APP_ID/QQBOT_APP_SECRET 与 QQBOT_APP_ID2/QQBOT_APP_SECRET2。

use cryo::model::Intent;
use cryo::{Bot, C2CMessageCtx, Config, Fleet, PlainText, Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut fleet = Fleet::new();

    let b1: Bot = fleet.add(Config::from_env(Intent::BASIC));
    fleet.add(Config {
        app_id: std::env::var("QQBOT_APP_ID2").expect("QQBOT_APP_ID2"),
        app_secret: std::env::var("QQBOT_APP_SECRET2").expect("QQBOT_APP_SECRET2"),
        intents: Intent::BASIC,
        shard: [0, 1],
    });

    // 注册到所有 bot：c.bot() 自动指向收到消息的账号。
    fleet.on_c2c_message(reply_echo);
    fleet.on_c2c_message(log_which);

    // 只注册到 b1：管理员指令。
    b1.on_c2c_message(admin_only);

    fleet.run().await
}

async fn log_which(c: C2CMessageCtx, text: PlainText) -> Result<()> {
    println!("[{}] c2c: {}", c.bot().app_id(), text);
    Ok(())
}

async fn reply_echo(c: C2CMessageCtx, text: PlainText) -> Result<()> {
    c.reply().text(format!("echo: {text}")).send().await
}

async fn admin_only(c: C2CMessageCtx, text: PlainText) -> Result<()> {
    if text.as_ref() != "/admin" {
        return Ok(());
    }
    c.reply().text("only bot1 answers this").send().await
}
