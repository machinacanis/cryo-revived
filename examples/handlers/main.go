// 多 handler 示例：私聊与群聊分别由独立函数处理。
// 凭证读环境变量 QQBOT_APP_ID / QQBOT_APP_SECRET。
package main

import (
	"context"
	"fmt"
	"log/slog"
	"strings"

	"cryo"
	"cryo/model"
)

func main() {
	bot := cryo.New(cryo.Config{Intents: model.IntentWithMembers})

	// 私聊 handlers（支持一次传多个函数）
	bot.OnC2CMessage(onPrivate, logIncoming)

	// 群聊 handlers
	bot.OnGroupAtMessage(onGroupAt)
	bot.OnGroupJoin(onBotJoinGroup)
	bot.OnGroupLeave(onBotLeaveGroup)
	bot.OnGroupMemberAdd(onMemberJoin)
	bot.OnFriendAdd(onFriendAdd)
	bot.OnFriendDel(onFriendDel)

	bot.MustRun(context.Background())
}

// logIncoming 与 onPrivate 同时注册在 C2C_MESSAGE_CREATE 上，按注册顺序执行。
func logIncoming(c *cryo.C2CMessageCtx, from cryo.SenderOpenID, l *slog.Logger) error {
	l.Info("c2c message", "from", from, "len", len(c.Message.Content))
	return nil
}

// --- 私聊 ---

// onPrivate 处理所有私聊消息：注入发送者 openid、纯文本与完整 User 对象。
func onPrivate(c *cryo.C2CMessageCtx, from cryo.SenderOpenID, text cryo.PlainText) error {
	switch {
	case text == "/ping":
		return c.Reply().Text("pong").Send()

	case text == "/me":
		u := c.Message.Author
		return c.Reply().Text(fmt.Sprintf(
			"openid: %s\n昵称: %s\n消息类型: %d", from, u.Username, c.Message.MessageType)).Send()

	case strings.HasPrefix(string(text), "/echo "):
		return c.Reply().Text(strings.TrimPrefix(string(text), "/echo ")).Send()

	case text == "/help":
		return c.Reply().Markdown("## 指令\n- /ping\n- /me\n- /echo <文本>\n- /help").Send()

	default:
		return c.Reply().Text("收到，发 /help 看指令").Send()
	}
}

// onFriendAdd 用户添加机器人为好友时主动打招呼（主动消息，无 msg_id）。
func onFriendAdd(c *cryo.FriendCtx, who cryo.SenderOpenID, l *slog.Logger) error {
	l.Info("friend added", "openid", who)
	_, err := c.API().User(string(who)).SendText(c, "你好，我是机器人，发 /help 看指令")
	return err
}

// onFriendDel 仅记录日志，不发消息（已非好友，发了也失败）。
func onFriendDel(c *cryo.FriendCtx, who cryo.SenderOpenID, l *slog.Logger) error {
	l.Info("friend removed", "openid", who)
	return nil
}

// --- 群聊 ---

// onGroupAt 处理群内 @机器人 的消息：注入群 openid 和消息 id。
func onGroupAt(c *cryo.GroupMessageCtx, gid cryo.GroupOpenID, mid cryo.MessageID, text cryo.PlainText) error {
	switch {
	case text == "/ping":
		return c.Reply().Text("pong").Send()

	case text == "/whoami":
		m, err := c.API().Group(string(gid)).GetMember(c, c.Message.Author.MemberOpenID)
		if err != nil {
			return c.Reply().Text("查不到你的成员信息").Send()
		}
		return c.Reply().Text(fmt.Sprintf("角色: %s\n入群: %s", m.Role, m.JoinTime)).Send()

	case text == "/info":
		info, err := c.API().Group(string(gid)).GetInfo(c)
		if err != nil {
			return c.Reply().Text("群信息获取失败").Send()
		}
		return c.Reply().Text(fmt.Sprintf("群名: %s\n人数: %d", info.GroupName, info.GroupMemberNum)).Send()

	default:
		return c.Reply().Quote(string(mid)).Text("收到 @ 消息").Send()
	}
}

// onBotJoinGroup 机器人被拉进群时发言。
func onBotJoinGroup(c *cryo.GroupBotCtx) error {
	_, err := c.API().Group(c.Event.GroupOpenID).
		SendText(c, "感谢邀请，@我 并发送 /help 查看指令")
	return err
}

// onBotLeaveGroup 仅记录日志。
func onBotLeaveGroup(c *cryo.GroupBotCtx, l *slog.Logger) error {
	l.Info("removed from group", "group", c.Event.GroupOpenID,
		"operator", c.Event.OpMemberOpenID)
	return nil
}

// onMemberJoin 新成员进群欢迎语（需要 GROUP_MEMBER_EVENT intent 权限）。
func onMemberJoin(c *cryo.GroupMemberCtx, gid cryo.GroupOpenID) error {
	_, err := c.API().Group(string(gid)).
		SendMessage(c, model.NewMessage().Text("欢迎新成员，@我 /help 看指令"))
	return err
}
