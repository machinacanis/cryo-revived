// 多 bot 账号示例：一个进程跑 N 个机器人，handler 注册一次全群生效。
// 凭证：QQBOT_APP_ID/QQBOT_APP_SECRET 与 QQBOT_APP_ID2/QQBOT_APP_SECRET2
package main

import (
	"context"
	"fmt"
	"os"

	"cryo"
	"cryo/model"
)

func main() {
	fleet := cryo.NewFleet()

	b1 := fleet.Add(cryo.Config{
		AppID:     os.Getenv("QQBOT_APP_ID"),
		AppSecret: os.Getenv("QQBOT_APP_SECRET"),
		Intents:   model.IntentBasic,
	})
	fleet.Add(cryo.Config{
		AppID:     os.Getenv("QQBOT_APP_ID2"),
		AppSecret: os.Getenv("QQBOT_APP_SECRET2"),
		Intents:   model.IntentBasic,
	})

	// 注册到所有 bot：c.Bot() 自动指向收到消息的账号
	fleet.OnC2CMessage(replyEcho, logWhich)

	// 只注册到 b1：管理员指令
	b1.OnC2CMessage(adminOnly)

	fleet.MustRun(context.Background())
}

func logWhich(c *cryo.C2CMessageCtx, text cryo.PlainText) error {
	fmt.Printf("[%s] c2c: %s\n", c.Bot().AppID(), text)
	return nil
}

func replyEcho(c *cryo.C2CMessageCtx, text cryo.PlainText) error {
	return c.Reply().Text("echo: " + string(text)).Send()
}

func adminOnly(c *cryo.C2CMessageCtx, text cryo.PlainText) error {
	if string(text) != "/admin" {
		return nil
	}
	return c.Reply().Text("only bot1 answers this").Send()
}
