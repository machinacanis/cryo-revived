package cryo

import (
	"context"
	"sync"

	"cryo/api"
)

// --- 全局默认 Bot（http.HandleFunc 风格）---
//
// 适合快速原型与单 bot 应用：注册函数与 handler 可以就近书写，
// 既支持函数内调用，也支持包级 `var _ = cryo.OnXxx(fn)` 自注册：
//
//	func main() {
//	    cryo.OnC2CMessage(func(c *cryo.C2CMessageCtx) error {
//	        return c.Reply().Text("hi").Send()
//	    })
//	    cryo.MustRun(context.Background())
//	}
//
// 需要多实例或自定义 Config 时仍使用 cryo.New()。

var (
	defMu  sync.Mutex
	defCfg Config
	defBot *Bot
)

// Configure 设置默认 Bot 的配置并返回它。
// 必须在任何注册或 Run 之前调用（默认 Bot 构建后再改配置会 panic）。
// 包级写法同样成立：
//
//	var bot = cryo.Configure(cryo.Config{Intents: model.IntentBasic})
//	var _   = bot.OnC2CMessage(onPrivate)
func Configure(cfg Config) *Bot {
	defMu.Lock()
	defer defMu.Unlock()
	if defBot != nil {
		panic("cryo: Configure must be called before default bot is used")
	}
	defCfg = cfg
	defBot = New(defCfg)
	return defBot
}

// Default 返回全局默认 Bot（惰性构建，凭证默认读环境变量）。
func Default() *Bot {
	defMu.Lock()
	defer defMu.Unlock()
	if defBot == nil {
		defBot = New(defCfg)
	}
	return defBot
}

// --- 包级透传 API ---

// On 注册事件 handler 到默认 Bot。
func On(eventType string, handlers ...interface{}) *Bot { return Default().On(eventType, handlers...) }

func OnC2CMessage(hs ...interface{}) *Bot     { return Default().OnC2CMessage(hs...) }
func OnGroupAtMessage(hs ...interface{}) *Bot { return Default().OnGroupAtMessage(hs...) }
func OnGroupMessage(hs ...interface{}) *Bot   { return Default().OnGroupMessage(hs...) }
func OnFriendAdd(hs ...interface{}) *Bot      { return Default().OnFriendAdd(hs...) }
func OnFriendDel(hs ...interface{}) *Bot      { return Default().OnFriendDel(hs...) }
func OnGroupJoin(hs ...interface{}) *Bot      { return Default().OnGroupJoin(hs...) }
func OnGroupLeave(hs ...interface{}) *Bot     { return Default().OnGroupLeave(hs...) }
func OnGroupMemberAdd(hs ...interface{}) *Bot { return Default().OnGroupMemberAdd(hs...) }
func OnGroupMemberRemove(hs ...interface{}) *Bot {
	return Default().OnGroupMemberRemove(hs...)
}
func OnInteraction(hs ...interface{}) *Bot    { return Default().OnInteraction(hs...) }
func OnMessageAudited(hs ...interface{}) *Bot { return Default().OnMessageAudited(hs...) }

// Provide 注册依赖到默认 Bot。
func Provide(svcs ...interface{}) *Bot { return Default().Provide(svcs...) }

// Param 注册事件数据 provider 到默认 Bot。
func Param(fn interface{}) *Bot { return Default().Param(fn) }

// Invoke 注册启动函数到默认 Bot。
func Invoke(fn interface{}) *Bot { return Default().Invoke(fn) }

// API 返回默认 Bot 的 REST client。
func API() *api.Client { return Default().API() }

// Run 启动默认 Bot。
func Run(ctx context.Context) error { return Default().Run(ctx) }

// MustRun 启动默认 Bot，fatal error 时 panic。
func MustRun(ctx context.Context) { Default().MustRun(ctx) }
