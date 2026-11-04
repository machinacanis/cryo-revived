//! 最小实例：收到私聊消息回 "hi"。凭证读环境变量 QQBOT_APP_ID/QQBOT_APP_SECRET。
//!
//! ```text
//! QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx cargo run --example minimal
//! ```

use cryo::model::Intent;
use cryo::{Bot, C2CMessageCtx};

#[tokio::main]
async fn main() -> cryo::Result<()> {
    Bot::from_env(Intent::BASIC)
        .on_c2c_message(|c: C2CMessageCtx| c.say("hi"))
        .run()
        .await
}
