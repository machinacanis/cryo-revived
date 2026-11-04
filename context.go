package cryo

import (
	"context"
	"encoding/json"
	"fmt"
	"reflect"

	"cryo/api"
	"cryo/model"
)

// Ctx is the handler context: it embeds context.Context, exposes the event
// payload and resolves injected services.
type Ctx struct {
	context.Context
	bot     *Bot
	payload *model.Payload
}

func (b *Bot) newCtx(p *model.Payload) *Ctx {
	return &Ctx{Context: context.Background(), bot: b, payload: p}
}

// EventType returns the event name, e.g. C2C_MESSAGE_CREATE.
func (c *Ctx) EventType() string { return c.payload.T }

// EventID returns the payload-level event id (usable as event_id in replies).
func (c *Ctx) EventID() string { return c.payload.ID }

// Seq returns the gateway sequence number of this payload.
func (c *Ctx) Seq() int { return c.payload.S }

// API returns the REST client for active calls.
func (c *Ctx) API() *api.Client { return c.bot.api }

// Bot returns the bot instance.
func (c *Ctx) Bot() *Bot { return c.bot }

// Decode unmarshals the event body (payload.d) into v.
func (c *Ctx) Decode(v interface{}) error { return json.Unmarshal(c.payload.D, v) }

// Raw returns the raw event body bytes.
func (c *Ctx) Raw() json.RawMessage { return c.payload.D }

// Inject fills ptr with a service registered via bot.Provide:
//
//	var db *sql.DB
//	if err := c.Inject(&db); err != nil { ... }
func (c *Ctx) Inject(ptr interface{}) error {
	rv := reflect.ValueOf(ptr)
	if rv.Kind() != reflect.Ptr || rv.Elem().Kind() != reflect.Ptr {
		return fmt.Errorf("cryo: Inject expects **T, got %T", ptr)
	}
	v, err := c.bot.resolve(rv.Elem().Type())
	if err != nil {
		return err
	}
	rv.Elem().Set(v)
	return nil
}

// MustInject is Inject that panics when the service is missing.
func (c *Ctx) MustInject(ptr interface{}) {
	if err := c.Inject(ptr); err != nil {
		panic(err)
	}
}

// Get fetches an injected service in one call (generic convenience):
//
//	db := cryo.Get[*sql.DB](c)
func Get[T any](c *Ctx) (T, error) {
	var out T
	t := reflect.TypeOf(&out).Elem()
	v, err := c.bot.resolve(t)
	if err != nil {
		return out, err
	}
	return v.Interface().(T), nil
}

// Must is Get that panics when the service is missing.
func Must[T any](c *Ctx) T {
	v, err := Get[T](c)
	if err != nil {
		panic(err)
	}
	return v
}
