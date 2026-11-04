// 示例：NoneBot 式事件数据注入 —— handler 声明参数类型，框架按需注入。
//
// 可注入的事件数据（内置）：
//
//	cryo.PlainText     消息纯文本（已 TrimSpace）
//	cryo.SenderOpenID  发送者 openid（单聊 user_openid / 群聊 member_openid / 好友事件 openid）
//	cryo.GroupOpenID   群 openid（仅群域事件）
//	cryo.MessageID     消息 id（可做被动回复 msg_id）
//	cryo.EventIDM      事件 id（可做被动回复 event_id）
//	*model.User        发送者完整 User 对象
//	*model.C2CMessage / *model.GroupMessage / *model.Interaction / ...  解码后的事件体
//
// 自定义事件数据用 bot.Param(func(c *cryo.Ctx) (T, error)) 注册；
// 应用级服务仍走 bot.Provide 构造器注入。
//
//	QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx go run ./examples/userinfo
package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"cryo"
	"cryo/model"
)

func main() {
	bot := cryo.New(cryo.Config{
		AppID:     os.Getenv("QQBOT_APP_ID"),
		AppSecret: os.Getenv("QQBOT_APP_SECRET"),
		Intents:   model.IntentBasic,
	})

	// 自定义事件数据 provider：把"事件上下文里的当前时间"注入为 time.Time。
	bot.Param(func(c *cryo.Ctx) (time.Time, error) { return time.Now(), nil })

	// 私聊：注入 sender openid + 纯文本 + 完整 User 对象 + 时间
	bot.OnC2CMessage(func(c *cryo.C2CMessageCtx,
		from cryo.SenderOpenID,
		text cryo.PlainText,
		u *model.User,
		now time.Time) error {

		if string(text) != "/me" {
			return c.Reply().Text("发 /me 查看你的信息").Send()
		}
		return c.Reply().Text(fmt.Sprintf(
			"openid: %s\n昵称: %s\n角色: %s\n时间: %s",
			from, u.Username, u.MemberRole, now.Format("15:04:05"))).Send()
	})

	// 群@：只要需要的就声明 —— 注入群id、消息id、纯文本
	bot.OnGroupAtMessage(func(c *cryo.GroupMessageCtx,
		gid cryo.GroupOpenID,
		mid cryo.MessageID,
		text cryo.PlainText,
		msg *model.GroupMessage) error {

		if string(text) != "/whoami" {
			return c.Reply().Text("发 /whoami 查看成员信息").Send()
		}
		member, err := c.API().Group(string(gid)).GetMember(c, msg.Author.MemberOpenID)
		if err != nil {
			return c.Reply().Text("查询失败: " + err.Error()).Send()
		}
		return c.Reply().Text(fmt.Sprintf(
			"角色: %s\n昵称: %s\n入群: %s\n(回复 msg_id=%s…)",
			member.Role, member.Nick, member.JoinTime, string(mid)[:12])).Send()
	})

	// 好友添加：只要 openid，就声明 cryo.SenderOpenID —— 无需解码事件体
	bot.OnFriendAdd(func(c *cryo.FriendCtx, who cryo.SenderOpenID) error {
		_, e := c.API().User(string(who)).SendText(c, "你好！发送 /me 查看你的信息")
		return e
	})

	// 互动事件：注入解码后的 Interaction 体
	bot.OnInteraction(func(c *cryo.InteractionCtx, it *model.Interaction) error {
		fmt.Println("interaction type:", it.Type)
		return c.Respond(0)
	})

	ctx, stop := signal.NotifyContext(context.Background(),
		syscall.SIGINT, syscall.SIGTERM)
	defer stop()
	bot.MustRun(ctx)
}
