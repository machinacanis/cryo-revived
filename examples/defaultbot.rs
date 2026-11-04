//! 全局默认 Bot 示例：init 显式声明权限 + 就近注册。
//! 凭证读环境变量 QQBOT_APP_ID / QQBOT_APP_SECRET。

use cryo::model::Intent;
use cryo::{C2CMessageCtx, Config, GroupMessageCtx, PlainText};

#[tokio::main]
async fn main() -> cryo::Result<()> {
    tracing_subscriber::fmt::init();

    // 先声明权限，再注册 —— 默认 bot 不允许静默默认 intents。
    cryo::init(Config::from_env(Intent::BASIC));

    // 函数内就近写法：定义和注册挨在一起。
    cryo::on_c2c_message(|c: C2CMessageCtx, text: PlainText| async move {
        c.reply().text(format!("echo: {text}")).send().await
    });

    cryo::on_group_at_message(|c: GroupMessageCtx| async move {
        c.reply().text("收到 @").send().await
    });

    cryo::run().await
}
