// Package cryo is a friendly QQ Bot framework: typed events, fluent message
// builders, dependency injection and a self-healing gateway session.
//
//	bot := cryo.New(cryo.Config{AppID: id, AppSecret: secret})
//	bot.OnC2CMessage(func(c *cryo.C2CMessageCtx) error {
//	    return c.Reply().Text("hi").Send()
//	})
//	bot.Run(context.Background())
package cryo

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"reflect"
	"sync"

	"cryo/api"
	"cryo/gateway"
	"cryo/model"
)

// Config of a Bot.
type Config struct {
	AppID     string
	AppSecret string
	// Intents 必须显式声明本 bot 订阅的事件权限（如 model.IntentBasic）。
	// 留 0 会在 New 时 panic —— 不允许静默默认权限。
	Intents model.Intent
	// Shard [id, num]; zero value means [0,1].
	Shard  [2]int
	Logger *slog.Logger
}

// Bot is the application entry point.
type Bot struct {
	cfg    Config
	api    *api.Client
	logger *slog.Logger

	mu        sync.RWMutex
	di        *injector
	providers map[reflect.Type]provider  // event-data param injectors
	handlers  map[string][]reflect.Value // event type -> handler funcs
	invokes   []reflect.Value            // startup functions, deps auto-injected

	sessionID string
}

// New builds a Bot.
// Credentials fall back to env QQBOT_APP_ID / QQBOT_APP_SECRET when empty.
func New(cfg Config) *Bot {
	if cfg.AppID == "" {
		cfg.AppID = os.Getenv("QQBOT_APP_ID")
	}
	if cfg.AppSecret == "" {
		cfg.AppSecret = os.Getenv("QQBOT_APP_SECRET")
	}
	if cfg.Intents == 0 {
		panic("cryo: Config.Intents must be declared explicitly " +
			"(e.g. model.IntentBasic) — no silent permission defaults")
	}
	if cfg.Shard[1] == 0 {
		cfg.Shard = [2]int{0, 1}
	}
	logger := cfg.Logger
	if logger == nil {
		logger = slog.Default()
	}
	b := &Bot{
		cfg:       cfg,
		api:       api.NewClient(cfg.AppID, cfg.AppSecret),
		logger:    logger,
		di:        newInjector(),
		providers: map[reflect.Type]provider{},
		handlers:  map[string][]reflect.Value{},
	}
	b.registerBuiltinParams()
	// built-in injectable services
	b.Provide(b.api)
	b.Provide(logger)
	b.Provide(b)
	return b
}

// API exposes the raw REST client: bot.API().User(openid).Send(ctx, msg).
func (b *Bot) API() *api.Client { return b.api }

// Logger returns the bot logger.
func (b *Bot) Logger() *slog.Logger { return b.logger }

// SessionID of the current gateway session (set after READY).
func (b *Bot) SessionID() string { return b.sessionID }

// AppID of this bot account — handy in fleets to tell which bot got an event.
func (b *Bot) AppID() string { return b.cfg.AppID }

// Provide registers dependencies — either ready-made instances or
// constructors. Constructors are resolved automatically at startup: their
// parameters are themselves injected from the graph, so registration order
// never matters and cycles/missing deps fail fast in Run().
//
//	bot.Provide(NewGreeter)              // func() *Greeter
//	bot.Provide(NewStore)                // func(*Greeter) (*Store, error)
//	bot.Provide(&StaticCfg{Debug: true}) // instance
func (b *Bot) Provide(svcs ...interface{}) *Bot {
	b.mu.Lock()
	defer b.mu.Unlock()
	for _, s := range svcs {
		if err := b.di.provide(s); err != nil {
			panic(err)
		}
	}
	return b
}

// Invoke runs fn at startup with its parameters auto-injected, fx-style.
// Use it for boot wiring that needs services (e.g. starting an HTTP server).
func (b *Bot) Invoke(fn interface{}) *Bot {
	v := reflect.ValueOf(fn)
	if v.Kind() != reflect.Func {
		panic("cryo: Invoke expects a function")
	}
	b.invokes = append(b.invokes, v)
	return b
}

func (b *Bot) resolve(t reflect.Type) (reflect.Value, error) {
	return b.di.resolve(t)
}

// --- event registration ---

// On registers a generic handler for an event type name (model.Event*).
// The handler must be func(...) error; the first parameter is a *Ctx (or a
// typed ctx like *C2CMessageCtx), further parameters are injected services:
//
//	bot.On(model.EventC2CMessageCreate, func(c *cryo.Ctx, db *sql.DB) error { ... })
func (b *Bot) On(eventType string, handlers ...interface{}) *Bot {
	for _, h := range handlers {
		v := reflect.ValueOf(h)
		t := v.Type()
		if t.Kind() != reflect.Func {
			panic("cryo: handler must be a function")
		}
		if t.NumIn() == 0 || !isCtxType(t.In(0)) {
			panic(fmt.Sprintf("cryo: handler %s first param must be a *Ctx-like type (e.g. *cryo.Ctx, *cryo.C2CMessageCtx)", t))
		}
		b.mu.Lock()
		b.handlers[eventType] = append(b.handlers[eventType], v)
		b.mu.Unlock()
	}
	return b
}

// Run starts the gateway session and blocks until ctx is cancelled or a
// fatal error occurs. Use MustRun for fire-and-forget.
func (b *Bot) Run(ctx context.Context) error {
	// fail fast: build the whole dependency graph before serving
	if err := b.di.warm(); err != nil {
		return err
	}
	for _, iv := range b.invokes {
		if err := b.call(iv); err != nil {
			return fmt.Errorf("cryo: Invoke: %w", err)
		}
	}
	s := &gateway.Session{
		API:     b.api,
		Intents: b.cfg.Intents,
		Shard:   b.cfg.Shard,
		Logger:  b.logger,
		OnEvent: b.dispatch,
		OnSession: func(id string, u *model.User) {
			b.sessionID = id
			name := ""
			if u != nil {
				name = u.Username
			}
			b.logger.Info("gateway ready", "session", id, "bot", name)
		},
	}
	return s.Run(ctx)
}

// MustRun is Run that panics on fatal error (handy for main()).
func (b *Bot) MustRun(ctx context.Context) {
	if err := b.Run(ctx); err != nil && err != context.Canceled {
		panic(err)
	}
}

// dispatch decodes and routes one gateway payload.
func (b *Bot) dispatch(p *model.Payload) {
	if p.Op != model.OpDispatch || p.T == model.EventReady || p.T == model.EventResumed {
		return
	}
	b.mu.RLock()
	handlers := append([]reflect.Value(nil), b.handlers[p.T]...)
	wildcard := append([]reflect.Value(nil), b.handlers["*"]...)
	b.mu.RUnlock()
	for _, h := range append(handlers, wildcard...) {
		if err := b.invoke(h, p); err != nil {
			b.logger.Error("handler error", "event", p.T, "err", err)
		}
	}
}

// invoke calls one handler, injecting the context and declared services.
func (b *Bot) invoke(h reflect.Value, p *model.Payload) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("panic: %v", r)
		}
	}()
	t := h.Type()
	if t.NumIn() == 0 {
		return fmt.Errorf("handler needs at least a context parameter")
	}
	ctx := b.newCtx(p)

	args := make([]reflect.Value, 0, t.NumIn())
	for i := 0; i < t.NumIn(); i++ {
		pt := t.In(i)
		if i == 0 && isCtxType(pt) {
			var c interface{} = ctx
			if pt != ctxType {
				c = buildTypedCtx(pt, ctx, p)
				if c == nil {
					return fmt.Errorf("cannot build %s for event %s", pt, p.T)
				}
			}
			args = append(args, reflect.ValueOf(c))
			continue
		}
		// event-data provider first, then app services
		if pv, ok := b.providers[pt]; ok {
			v, err := pv(ctx)
			if err != nil {
				return fmt.Errorf("param %d (%s): %w", i, pt, err)
			}
			args = append(args, v)
			continue
		}
		v, err := b.resolve(pt)
		if err != nil {
			return fmt.Errorf("param %d: %w", i, err)
		}
		args = append(args, v)
	}
	outs := h.Call(args)
	if len(outs) > 0 {
		if e, ok := outs[len(outs)-1].Interface().(error); ok && e != nil {
			return e
		}
	}
	return nil
}

// call invokes a startup function resolving every parameter from the graph.
func (b *Bot) call(fn reflect.Value) error {
	t := fn.Type()
	args := make([]reflect.Value, t.NumIn())
	for i := range args {
		v, err := b.resolve(t.In(i))
		if err != nil {
			return fmt.Errorf("param %d: %w", i, err)
		}
		args[i] = v
	}
	outs := fn.Call(args)
	if len(outs) > 0 {
		if e, ok := outs[len(outs)-1].Interface().(error); ok && e != nil {
			return e
		}
	}
	return nil
}

var ctxType = reflect.TypeOf(&Ctx{})

func isCtxType(t reflect.Type) bool {
	return t.Kind() == reflect.Ptr && t.Implements(reflect.TypeOf((*interface{ EventType() string })(nil)).Elem())
}
