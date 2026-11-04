package cryo

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"

	"cryo/model"
)

// --- 事件数据注入（NoneBot-style param injection）---
//
// Handlers may declare extra parameters whose values are extracted from the
// current event on demand — declare the marker type, get the data:
//
//	bot.OnC2CMessage(func(c *cryo.C2CMessageCtx, from cryo.SenderOpenID, text cryo.PlainText) error {
//	    // from = 发送者 openid，text = 消息纯文本
//	})
//
// Custom extractors can be registered via bot.Param, e.g.
//	bot.Param(func(c *cryo.Ctx) (Weather, error) { ... })

// Marker types for event-scoped injection.

// PlainText is the message's plain text content (trimmed).
type PlainText string

// SenderOpenID is the sender's openid — user_openid in c2c,
// member_openid in group, openid in friend events.
type SenderOpenID string

// GroupOpenID is the group_openid of group-scoped events.
type GroupOpenID string

// MessageID is the message id (d.id) of message events — usable as msg_id.
type MessageID string

// EventID is the payload-level event id — usable as event_id.
type EventID string

func (t PlainText) String() string    { return string(t) }
func (t SenderOpenID) String() string { return string(t) }
func (t GroupOpenID) String() string  { return string(t) }
func (t MessageID) String() string    { return string(t) }
func (t EventID) String() string      { return string(t) }

// provider extracts a value of one type from the event context.
type provider func(*Ctx) (reflect.Value, error)

// Param registers a custom event-data provider:
//
//	bot.Param(func(c *cryo.Ctx) (*MyThing, error) { ... })
//
// The function's first return type becomes the injectable param type.
func (b *Bot) Param(fn interface{}) *Bot {
	v := reflect.ValueOf(fn)
	t := v.Type()
	if t.Kind() != reflect.Func || t.NumIn() != 1 || t.In(0) != ctxType ||
		(t.NumOut() != 1 && t.NumOut() != 2) {
		panic("cryo: Param expects func(*Ctx) T or func(*Ctx) (T, error)")
	}
	if t.NumOut() == 2 && !t.Out(1).Implements(errType) {
		panic("cryo: Param second return must be error")
	}
	out := t.Out(0)
	b.providers[out] = func(c *Ctx) (reflect.Value, error) {
		outs := v.Call([]reflect.Value{reflect.ValueOf(c)})
		if t.NumOut() == 2 {
			if e := outs[1].Interface(); e != nil {
				return reflect.Value{}, e.(error)
			}
		}
		return outs[0], nil
	}
	return b
}

// eventProbe decodes the common fields across event bodies.
type eventProbe struct {
	ID          string          `json:"id"`
	Content     string          `json:"content"`
	GroupOpenID string          `json:"group_openid"`
	OpenID      string          `json:"openid"`
	Author      json.RawMessage `json:"author"`
}

func (c *Ctx) probe() *eventProbe {
	var p eventProbe
	_ = json.Unmarshal(c.payload.D, &p)
	return &p
}

// senderID returns the best available sender openid.
func (p *eventProbe) senderID() string {
	if len(p.Author) > 0 {
		var a struct {
			UserOpenID   string `json:"user_openid"`
			MemberOpenID string `json:"member_openid"`
			ID           string `json:"id"`
		}
		_ = json.Unmarshal(p.Author, &a)
		for _, id := range []string{a.UserOpenID, a.MemberOpenID, a.ID} {
			if id != "" {
				return id
			}
		}
	}
	return p.OpenID
}

// eventModel maps decodable event bodies to the event names carrying them.
var eventModel = map[reflect.Type][]string{
	reflect.TypeOf(&model.C2CMessage{}):       {model.EventC2CMessageCreate},
	reflect.TypeOf(&model.GroupMessage{}):     {model.EventGroupAtMessage, model.EventGroupMessage},
	reflect.TypeOf(&model.FriendEvent{}):      {model.EventFriendAdd, model.EventFriendDel, model.EventC2CMsgReject, model.EventC2CMsgReceive},
	reflect.TypeOf(&model.GroupBotEvent{}):    {model.EventGroupAddRobot, model.EventGroupDelRobot, model.EventGroupMsgReject, model.EventGroupMsgReceive},
	reflect.TypeOf(&model.GroupMemberEvent{}): {model.EventGroupMemberAdd, model.EventGroupMemberRemove, model.EventGroupJoinRequest},
	reflect.TypeOf(&model.Interaction{}):      {model.EventInteractionCreate},
	reflect.TypeOf(&model.MessageAudited{}):   {model.EventMessageAuditPass, model.EventMessageAuditReject},
}

func (b *Bot) registerBuiltinParams() {
	// decoded event bodies: *model.C2CMessage, *model.GroupMessage, ...
	for t, names := range eventModel {
		t, names := t, names
		b.providers[t] = func(c *Ctx) (reflect.Value, error) {
			for _, n := range names {
				if n == c.payload.T {
					v := reflect.New(t.Elem())
					if err := json.Unmarshal(c.payload.D, v.Interface()); err != nil {
						return reflect.Value{}, err
					}
					return v, nil
				}
			}
			return reflect.Value{}, fmt.Errorf("event %s doesn't carry %s", c.payload.T, t)
		}
	}
	// marker types
	register := func(sample interface{}, f func(*Ctx) (interface{}, error)) {
		t := reflect.TypeOf(sample)
		b.providers[t] = func(c *Ctx) (reflect.Value, error) {
			v, err := f(c)
			if err != nil {
				return reflect.Value{}, err
			}
			return reflect.ValueOf(v).Convert(t), nil
		}
	}
	register(PlainText(""), func(c *Ctx) (interface{}, error) {
		return strings.TrimSpace(c.probe().Content), nil
	})
	register(SenderOpenID(""), func(c *Ctx) (interface{}, error) {
		if id := c.probe().senderID(); id != "" {
			return id, nil
		}
		return nil, fmt.Errorf("event %s has no sender openid", c.payload.T)
	})
	register(GroupOpenID(""), func(c *Ctx) (interface{}, error) {
		if id := c.probe().GroupOpenID; id != "" {
			return id, nil
		}
		return nil, fmt.Errorf("event %s is not group-scoped", c.payload.T)
	})
	register(MessageID(""), func(c *Ctx) (interface{}, error) {
		if id := c.probe().ID; id != "" {
			return id, nil
		}
		return nil, fmt.Errorf("event %s has no message id", c.payload.T)
	})
	register(EventID(""), func(c *Ctx) (interface{}, error) {
		return c.payload.ID, nil
	})
	// *model.User — message sender / event subject
	b.providers[reflect.TypeOf(&model.User{})] = func(c *Ctx) (reflect.Value, error) {
		p := c.probe()
		if len(p.Author) == 0 {
			return reflect.Value{}, fmt.Errorf("event %s has no author", c.payload.T)
		}
		u := new(model.User)
		if err := json.Unmarshal(p.Author, u); err != nil {
			return reflect.Value{}, err
		}
		return reflect.ValueOf(u), nil
	}
}
