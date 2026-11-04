package cryo

import (
	"context"
	"sync"
)

// Fleet runs multiple bot accounts in one process. Each bot keeps its own
// gateway session, api client and handler set — but shared registrations
// can be applied once on the fleet:
//
//	fleet := cryo.NewFleet()
//	b1 := fleet.Add(cryo.Config{AppID: id1, AppSecret: s1})
//	b2 := fleet.Add(cryo.Config{AppID: id2, AppSecret: s2})
//
//	fleet.OnC2CMessage(echo)      // 注册到所有 bot（含以后 Add 的）
//	b1.OnGroupAtMessage(adminCmd) // 只注册到 b1
//
//	fleet.MustRun(ctx)            // 并发跑所有会话，一个 fatal 全停
//
// 事件上下文自动归属收到消息的账号：c.API() / c.Bot() 始终指向正确的 bot。
type Fleet struct {
	mu     sync.Mutex
	bots   []*Bot
	shared []func(*Bot) // 对后续 Add 的 bot 也生效的共享注册
}

// NewFleet creates an empty fleet.
func NewFleet() *Fleet { return &Fleet{} }

// Add creates a bot and joins the fleet. Shared registrations made before
// or after this call are applied to it.
func (f *Fleet) Add(cfg Config) *Bot {
	f.mu.Lock()
	defer f.mu.Unlock()
	b := New(cfg)
	for _, fn := range f.shared {
		fn(b)
	}
	f.bots = append(f.bots, b)
	return b
}

// Join adopts an existing bot into the fleet (shared registrations apply).
func (f *Fleet) Join(b *Bot) *Bot {
	f.mu.Lock()
	defer f.mu.Unlock()
	for _, fn := range f.shared {
		fn(b)
	}
	f.bots = append(f.bots, b)
	return b
}

// Bots returns the current members.
func (f *Fleet) Bots() []*Bot {
	f.mu.Lock()
	defer f.mu.Unlock()
	return append([]*Bot(nil), f.bots...)
}

// each applies fn to all existing bots and remembers it for future Add/Join.
func (f *Fleet) each(fn func(*Bot)) *Fleet {
	f.mu.Lock()
	defer f.mu.Unlock()
	f.shared = append(f.shared, fn)
	for _, b := range f.bots {
		fn(b)
	}
	return f
}

// Provide registers dependencies on every bot.
func (f *Fleet) Provide(svcs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.Provide(svcs...) })
}

// Param registers an event-data provider on every bot.
func (f *Fleet) Param(fn interface{}) *Fleet {
	return f.each(func(b *Bot) { b.Param(fn) })
}

// Invoke registers a startup function on every bot.
func (f *Fleet) Invoke(fn interface{}) *Fleet {
	return f.each(func(b *Bot) { b.Invoke(fn) })
}

// On registers handlers for an event type on every bot.
func (f *Fleet) On(eventType string, handlers ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.On(eventType, handlers...) })
}

func (f *Fleet) OnC2CMessage(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnC2CMessage(hs...) })
}
func (f *Fleet) OnGroupAtMessage(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnGroupAtMessage(hs...) })
}
func (f *Fleet) OnGroupMessage(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnGroupMessage(hs...) })
}
func (f *Fleet) OnFriendAdd(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnFriendAdd(hs...) })
}
func (f *Fleet) OnFriendDel(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnFriendDel(hs...) })
}
func (f *Fleet) OnGroupJoin(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnGroupJoin(hs...) })
}
func (f *Fleet) OnGroupLeave(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnGroupLeave(hs...) })
}
func (f *Fleet) OnGroupMemberAdd(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnGroupMemberAdd(hs...) })
}
func (f *Fleet) OnGroupMemberRemove(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnGroupMemberRemove(hs...) })
}
func (f *Fleet) OnInteraction(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnInteraction(hs...) })
}
func (f *Fleet) OnMessageAudited(hs ...interface{}) *Fleet {
	return f.each(func(b *Bot) { b.OnMessageAudited(hs...) })
}

// Run starts every bot session concurrently. The first fatal error cancels
// the rest and is returned; ctx cancel stops all cleanly.
func (f *Fleet) Run(ctx context.Context) error {
	f.mu.Lock()
	bots := append([]*Bot(nil), f.bots...)
	f.mu.Unlock()
	if len(bots) == 0 {
		return nil
	}
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	errc := make(chan error, len(bots))
	var wg sync.WaitGroup
	for _, b := range bots {
		b := b
		wg.Add(1)
		go func() {
			defer wg.Done()
			if err := b.Run(ctx); err != nil && err != context.Canceled {
				select {
				case errc <- err:
					cancel() // first fatal error stops the fleet
				default:
				}
			}
		}()
	}
	wg.Wait()
	select {
	case err := <-errc:
		return err
	default:
		return ctx.Err()
	}
}

// MustRun is Run that panics on fatal error.
func (f *Fleet) MustRun(ctx context.Context) {
	if err := f.Run(ctx); err != nil && err != context.Canceled {
		panic(err)
	}
}
