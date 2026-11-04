// 最小实例：收到私聊消息回 "hi"。凭证读环境变量 QQBOT_APP_ID/QQBOT_APP_SECRET。
package main

import (
	"context"

	"cryo"
	"cryo/model"
)

func main() {
	cryo.New(cryo.Config{Intents: model.IntentBasic}).
		OnC2CMessage(func(c *cryo.C2CMessageCtx) error {
			return c.Reply().Text("hi").Send()
		}).
		MustRun(context.Background())
}
