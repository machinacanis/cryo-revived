// Echo demo: replies to c2c and group messages, shows DI + builders.
//
//	QQBOT_APP_ID=xxx QQBOT_APP_SECRET=xxx go run ./examples/echo
package main

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"strings"
	"syscall"

	"cryo"
	"cryo/model"
)

// Greeter is an example injectable service.
type Greeter struct{ Name string }

func (g *Greeter) Hello() string { return "你好，我是 " + g.Name }

// NewGreeter is a DI constructor: its parameters (if any) are auto-injected.
func NewGreeter() *Greeter { return &Greeter{Name: "cryo"} }

func main() {
	bot := cryo.New(cryo.Config{
		AppID:     os.Getenv("QQBOT_APP_ID"),
		AppSecret: os.Getenv("QQBOT_APP_SECRET"),
		// default already covers c2c/group/interaction; add member events:
		Intents: model.IntentGroupAndC2C | model.IntentInteraction |
			model.IntentGroupMemberEvent,
	})

	// DI: register the constructor; dependencies resolve automatically at boot.
	bot.Provide(NewGreeter)

	// Typed handler: full autocomplete on c.Message, fluent reply.
	bot.OnC2CMessage(func(c *cryo.C2CMessageCtx) error {
		text := strings.TrimSpace(c.Message.Content)
		switch {
		case text == "/ping":
			return c.Reply().Text("pong").Send()
		case text == "/md":
			return c.Reply().
				Markdown("## cryo\n**markdown** reply works").
				Keyboard(model.NewKeyboard().Row(
					model.NewButton("点我").Style(1).Callback("btn-1").Confirm("确定？", "好", "算了"),
				).Build()).
				Send()
		default:
			return c.Reply().Text("echo: " + text).Send()
		}
	})

	bot.OnGroupAtMessage(func(c *cryo.GroupMessageCtx) error {
		return c.Reply().Text("收到 @" + c.Message.Author.MemberOpenID[:8] + " 的消息").Send()
	})

	// DI in action: extra params are resolved from Provide().
	bot.On(model.EventFriendAdd, func(c *cryo.FriendCtx, g *Greeter) error {
		_, err := c.API().User(c.Event.OpenID).SendText(c, g.Hello())
		return err
	})

	bot.OnGroupJoin(func(c *cryo.GroupBotCtx) error {
		_, err := c.API().Group(c.Event.GroupOpenID).
			SendText(c, "感谢拉我进群，发送 /ping 试试")
		return err
	})

	bot.OnInteraction(func(c *cryo.InteractionCtx) error {
		return c.Respond(0)
	})

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer stop()
	fmt.Println("bot running, ctrl+c to stop")
	bot.MustRun(ctx)
}
