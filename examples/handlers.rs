//! 多 handler 示例：私聊与群聊分别由独立函数处理。
//! 凭证读环境变量 QQBOT_APP_ID / QQBOT_APP_SECRET。
//!
//! 同一事件挂多个 handler = 链式注册（Rust 版的变参）：
//! `bot.on_c2c_message(a).on_c2c_message(b)`，按注册顺序执行。

use tracing::info;

use cryo::model::Intent;
use cryo::{
    Bot, C2CMessageCtx, Config, FriendCtx, GroupBotCtx, GroupMemberCtx, GroupMessageCtx,
    GroupOpenId, MessageId, PlainText, SenderOpenId,
};

#[tokio::main]
async fn main() -> cryo::Result<()> {
    tracing_subscriber::fmt::init();
    let bot = Bot::new(Config::from_env(Intent::WITH_MEMBERS));

    // 私聊 handlers（同一事件可链式挂多个）
    bot.on_c2c_message(on_private).on_c2c_message(log_incoming);

    // 群聊 handlers
    bot.on_group_at_message(on_group_at);
    bot.on_group_join(on_bot_join_group);
    bot.on_group_leave(on_bot_leave_group);
    bot.on_group_member_add(on_member_join);
    bot.on_friend_add(on_friend_add);
    bot.on_friend_del(on_friend_del);

    bot.run().await
}

/// 与 on_private 同挂 C2C_MESSAGE_CREATE，只记日志不回复。
async fn log_incoming(c: C2CMessageCtx, from: SenderOpenId) -> cryo::Result<()> {
    info!(%from, len = c.message.content.len(), "c2c message");
    Ok(())
}

// --- 私聊 ---

/// 处理所有私聊消息：注入发送者 openid、纯文本与类型化 ctx。
async fn on_private(c: C2CMessageCtx, from: SenderOpenId, text: PlainText) -> cryo::Result<()> {
    match text.as_ref() {
        "/ping" => c.reply().text("pong").send().await,
        "/me" => {
            let u = &c.message.author;
            c.reply()
                .text(format!(
                    "openid: {from}\n昵称: {}\n消息类型: {}",
                    u.username, c.message.message_type
                ))
                .send()
                .await
        }
        "/help" => {
            c.reply()
                .markdown("## 指令\n- /ping\n- /me\n- /echo <文本>\n- /help")
                .send()
                .await
        }
        s if s.starts_with("/echo ") => c.reply().text(s.trim_start_matches("/echo ")).send().await,
        _ => c.reply().text("收到，发 /help 看指令").send().await,
    }
}

/// 用户添加机器人为好友时主动打招呼（主动消息，无 msg_id）。
async fn on_friend_add(c: FriendCtx, who: SenderOpenId) -> cryo::Result<()> {
    info!(%who, "friend added");
    c.api()
        .user(who.to_string())
        .send_text("你好，我是机器人，发 /help 看指令")
        .await?;
    Ok(())
}

/// 仅记录日志，不发消息（已非好友，发了也失败）。
async fn on_friend_del(c: FriendCtx, who: SenderOpenId) -> cryo::Result<()> {
    let _ = c;
    info!(%who, "friend removed");
    Ok(())
}

// --- 群聊 ---

/// 处理群内 @机器人 的消息：注入群 openid、消息 id 和纯文本。
async fn on_group_at(
    c: GroupMessageCtx,
    gid: GroupOpenId,
    mid: MessageId,
    text: PlainText,
) -> cryo::Result<()> {
    match text.as_ref() {
        "/ping" => c.reply().text("pong").send().await,
        "/whoami" => {
            match c
                .api()
                .group(gid.to_string())
                .get_member(&c.message.author.member_openid)
                .await
            {
                Ok(m) => {
                    c.reply()
                        .text(format!(
                            "角色: {}\n入群: {}",
                            m.member_role, m.join_timestamp
                        ))
                        .send()
                        .await
                }
                Err(_) => c.reply().text("查不到你的成员信息").send().await,
            }
        }
        "/info" => match c.api().group(gid.to_string()).get_info().await {
            Ok(info) => {
                c.reply()
                    .text(format!(
                        "群名: {}\n人数: {}",
                        info.group_name, info.group_member_num
                    ))
                    .send()
                    .await
            }
            Err(_) => c.reply().text("群信息获取失败").send().await,
        },
        _ => {
            c.reply()
                .quote(mid.to_string())
                .text("收到 @ 消息")
                .send()
                .await
        }
    }
}

/// 机器人被拉进群时发言。
async fn on_bot_join_group(c: GroupBotCtx) -> cryo::Result<()> {
    c.api()
        .group(c.event_data.group_openid.clone())
        .send_text("感谢邀请，@我 并发送 /help 查看指令")
        .await?;
    Ok(())
}

/// 仅记录日志。
async fn on_bot_leave_group(c: GroupBotCtx) -> cryo::Result<()> {
    info!(
        group = %c.event_data.group_openid,
        operator = %c.event_data.op_member_openid,
        "removed from group"
    );
    Ok(())
}

/// 新成员进群欢迎语（需要 GROUP_MEMBER_EVENT intent 权限）。
async fn on_member_join(c: GroupMemberCtx, gid: GroupOpenId) -> cryo::Result<()> {
    c.api()
        .group(gid.to_string())
        .send_text("欢迎新成员，@我 /help 看指令")
        .await?;
    Ok(())
}
