// 全局默认 Bot 示例：包级 Configure 声明权限 + 就近注册。
// 凭证读环境变量 QQBOT_APP_ID / QQBOT_APP_SECRET。
package main

import (
	"context"

	"cryo"
	"cryo/model"
)

// 先声明权限，再注册 —— 包级 var 按声明顺序初始化。
var bot = cryo.Configure(cryo.Config{Intents: model.IntentBasic})

func onPrivate(c *cryo.C2CMessageCtx, text cryo.PlainText) error {
	return c.Reply().Text("echo: " + string(text)).Send()
}

var _ = bot.OnC2CMessage(onPrivate)

func main() {
	// 函数内就近写法同样成立
	bot.OnGroupAtMessage(func(c *cryo.GroupMessageCtx) error {
		return c.Reply().Text("收到 @").Send()
	})
	bot.MustRun(context.Background())
}
