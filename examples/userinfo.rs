//! 示例：axum 式事件数据注入 —— handler 声明参数类型，框架按需提供。
//!
//! 可注入的事件数据（内置）：
//!
//!   cryo::PlainText     消息纯文本（已 trim）
//!   cryo::SenderOpenId  发送者 openid（单聊 user_openid / 群聊 member_openid / 好友事件 openid）
//!   cryo::GroupOpenId   群 openid（仅群域事件；可选时用 Option<GroupOpenId>）
//!   cryo::MessageId     消息 id（可做被动回复 msg_id）
//!   cryo::EventId       事件 id（可做被动回复 event_id）
//!   model::User         发送者完整 User 对象
//!   model::C2CMessage / model::GroupMessage / model::Interaction / ...  解码后的事件体
//!   Inject<T>           bot.provide 注册的应用服务
//!
//! 自定义事件数据 = 给自己的类型 impl FromEvent。
//!
//! ```text
//! QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx cargo run --example userinfo
//! ```

use std::time::SystemTime;

use cryo::model::{GroupMessage, Intent, User};
use cryo::{
    Bot, C2CMessageCtx, Config, EventCtx, FriendCtx, FromEvent, GroupMessageCtx, GroupOpenId,
    InteractionCtx, MessageId, PlainText, Result, SenderOpenId,
};

/// 自定义 extractor：往 handler 注入"事件处理时刻"。
#[allow(dead_code)]
struct Now(SystemTime);

impl FromEvent for Now {
    fn from_event(_ctx: &EventCtx) -> Result<Self> {
        Ok(Now(SystemTime::now()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let bot = Bot::new(Config::from_env(Intent::BASIC));

    // 私聊：注入 sender openid + 纯文本 + 完整 User 对象 + 自定义 Now。
    bot.on_c2c_message(
        |c: C2CMessageCtx, from: SenderOpenId, text: PlainText, u: User, _now: Now| async move {
            if text.as_ref() != "/me" {
                return c.reply().text("发 /me 查看你的信息").send().await;
            }
            c.reply()
                .text(format!(
                    "openid: {from}\n昵称: {}\n角色: {}",
                    u.username, u.member_role
                ))
                .send()
                .await
        },
    );

    // 群@：只声明需要的 —— 群id、消息id、纯文本、解码后的 GroupMessage。
    bot.on_group_at_message(
        |c: GroupMessageCtx,
         gid: GroupOpenId,
         mid: MessageId,
         text: PlainText,
         msg: GroupMessage| async move {
            if text.as_ref() != "/whoami" {
                return c.reply().text("发 /whoami 查看成员信息").send().await;
            }
            match c
                .api()
                .group(gid.to_string())
                .get_member(&msg.author.member_openid)
                .await
            {
                Ok(m) => {
                    c.reply()
                        .text(format!(
                            "角色: {}\n昵称: {}\n入群: {}\n(回复 msg_id={}…)",
                            m.member_role,
                            m.nick,
                            m.join_timestamp,
                            &mid[..12.min(mid.len())]
                        ))
                        .send()
                        .await
                }
                Err(e) => c.reply().text(format!("查询失败: {e}")).send().await,
            }
        },
    );

    // 好友添加：只要 openid，就声明 SenderOpenId —— 无需解码事件体。
    bot.on_friend_add(|c: FriendCtx, who: SenderOpenId| async move {
        c.api()
            .user(who.to_string())
            .send_text("你好！发送 /me 查看你的信息")
            .await?;
        Ok::<(), cryo::Error>(())
    });

    // 互动事件：注入类型化 ctx，ack 后返回。
    bot.on_interaction(|c: InteractionCtx| async move {
        println!("interaction type: {}", c.interaction.interaction_type);
        c.respond(0).await
    });

    bot.run().await
}
