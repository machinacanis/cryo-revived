//! 多 handler 示例：私聊与群聊分别由独立函数处理。
//! 凭证读环境变量 QQBOT_APP_ID / QQBOT_APP_SECRET。
//!
//! 同一事件挂多个 handler = 链式注册（Rust 版的变参）：
//! `bot.on_c2c_message(a).on_c2c_message(b)`，按注册顺序执行。

use tracing::info;

use cryo::model::Intent;
use cryo::{
    Bot, C2CMessageCtx, FriendCtx, GroupBotCtx, GroupMemberCtx, GroupMessageCtx, GroupOpenId,
    MessageId, PlainText, SenderOpenId,
};

#[tokio::main]
async fn main() -> cryo::Result<()> {
    tracing_subscriber::fmt::init();
    let bot = Bot::from_env(Intent::WITH_MEMBERS);

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
        "/ping" => c.say("pong").await,
        "/me" => {
            let u = &c.message.author;
            c.say(format!(
                "openid: {from}\n昵称: {}\n消息类型: {}",
                u.username, c.message.message_type
            ))
            .await
        }
        "/help" => {
            c.reply()
                .markdown("## 指令\n- /ping\n- /me\n- /echo <文本>\n- /help")
                .await
        }
        s if s.starts_with("/echo ") => c.say(s.trim_start_matches("/echo ")).await,
        _ => c.say("收到，发 /help 看指令").await,
    }
}

/// 用户添加机器人为好友时主动打招呼（好友事件无 msg_id，say 即主动消息）。
async fn on_friend_add(c: FriendCtx, who: SenderOpenId) -> cryo::Result<()> {
    info!(%who, "friend added");
    c.say("你好，我是机器人，发 /help 看指令").await
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
        "/ping" => c.say("pong").await,
        "/whoami" => {
            match c
                .api()
                .group(&gid)
                .get_member(&c.message.author.member_openid)
                .await
            {
                Ok(m) => {
                    c.say(format!(
                        "角色: {}\n入群: {}",
                        m.member_role, m.join_timestamp
                    ))
                    .await
                }
                Err(_) => c.say("查不到你的成员信息").await,
            }
        }
        "/info" => match c.api().group(&gid).get_info().await {
            Ok(info) => {
                c.say(format!(
                    "群名: {}\n人数: {}",
                    info.group_name, info.group_member_num
                ))
                .await
            }
            Err(_) => c.say("群信息获取失败").await,
        },
        _ => c.say("收到 @ 消息").quote(mid).await,
    }
}

/// 机器人被拉进群时发言。
async fn on_bot_join_group(c: GroupBotCtx) -> cryo::Result<()> {
    c.say("感谢邀请，@我 并发送 /help 查看指令").await
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
    info!(%gid, member = %c.event_data.member_openid, "member joined");
    c.say("欢迎新成员，@我 /help 看指令").await
}
