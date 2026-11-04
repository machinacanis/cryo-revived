package cryo

import (
	"reflect"

	"cryo/model"
)

// --- typed handler contexts ---
//
// Each typed ctx embeds *Ctx (context.Context + DI) and exposes the decoded
// event plus a bound Reply() builder.

// C2CMessageCtx handles C2C_MESSAGE_CREATE.
type C2CMessageCtx struct {
	*Ctx
	Message *model.C2CMessage
}

// GroupMessageCtx handles GROUP_AT_MESSAGE_CREATE / GROUP_MESSAGE_CREATE.
type GroupMessageCtx struct {
	*Ctx
	Message *model.GroupMessage
}

// FriendCtx handles FRIEND_ADD / FRIEND_DEL / C2C_MSG_REJECT / C2C_MSG_RECEIVE.
type FriendCtx struct {
	*Ctx
	Event *model.FriendEvent
}

// GroupBotCtx handles GROUP_ADD_ROBOT / GROUP_DEL_ROBOT / GROUP_MSG_*.
type GroupBotCtx struct {
	*Ctx
	Event *model.GroupBotEvent
}

// GroupMemberCtx handles GROUP_MEMBER_ADD / GROUP_MEMBER_REMOVE / GROUP_JOIN_REQUEST.
type GroupMemberCtx struct {
	*Ctx
	Event *model.GroupMemberEvent
}

// InteractionCtx handles INTERACTION_CREATE.
type InteractionCtx struct {
	*Ctx
	Interaction *model.Interaction
}

// MessageAuditCtx handles MESSAGE_AUDIT_PASS / MESSAGE_AUDIT_REJECT.
type MessageAuditCtx struct {
	*Ctx
	Event *model.MessageAudited
}

// --- typed ctx factory ---

type ctxFactory func(*Ctx, *model.Payload) interface{}

var typedCtxs = map[reflect.Type]ctxFactory{
	reflect.TypeOf(&C2CMessageCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		m := new(model.C2CMessage)
		if c.Decode(m) != nil {
			return nil
		}
		return &C2CMessageCtx{Ctx: c, Message: m}
	},
	reflect.TypeOf(&GroupMessageCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		m := new(model.GroupMessage)
		if c.Decode(m) != nil {
			return nil
		}
		return &GroupMessageCtx{Ctx: c, Message: m}
	},
	reflect.TypeOf(&FriendCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		e := new(model.FriendEvent)
		if c.Decode(e) != nil {
			return nil
		}
		return &FriendCtx{Ctx: c, Event: e}
	},
	reflect.TypeOf(&GroupBotCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		e := new(model.GroupBotEvent)
		if c.Decode(e) != nil {
			return nil
		}
		return &GroupBotCtx{Ctx: c, Event: e}
	},
	reflect.TypeOf(&GroupMemberCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		e := new(model.GroupMemberEvent)
		if c.Decode(e) != nil {
			return nil
		}
		return &GroupMemberCtx{Ctx: c, Event: e}
	},
	reflect.TypeOf(&InteractionCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		e := new(model.Interaction)
		if c.Decode(e) != nil {
			return nil
		}
		return &InteractionCtx{Ctx: c, Interaction: e}
	},
	reflect.TypeOf(&MessageAuditCtx{}): func(c *Ctx, p *model.Payload) interface{} {
		e := new(model.MessageAudited)
		if c.Decode(e) != nil {
			return nil
		}
		return &MessageAuditCtx{Ctx: c, Event: e}
	},
}

func buildTypedCtx(pt reflect.Type, c *Ctx, p *model.Payload) interface{} {
	f, ok := typedCtxs[pt]
	if !ok {
		return nil
	}
	return f(c, p)
}

// --- typed registration shortcuts ---

// OnC2CMessage registers a handler for incoming c2c (private chat) messages.
func (b *Bot) OnC2CMessage(hs ...interface{}) *Bot {
	return b.On(model.EventC2CMessageCreate, hs...)
}

// OnGroupAtMessage handles messages @-mentioning the bot in a group.
func (b *Bot) OnGroupAtMessage(hs ...interface{}) *Bot {
	return b.On(model.EventGroupAtMessage, hs...)
}

// OnGroupMessage handles every group message (full-message mode, needs
// the corresponding permission). Same ctx as OnGroupAtMessage.
func (b *Bot) OnGroupMessage(hs ...interface{}) *Bot {
	return b.On(model.EventGroupMessage, hs...)
}

// OnFriendAdd / OnFriendDel handle users adding/removing the bot.
func (b *Bot) OnFriendAdd(hs ...interface{}) *Bot { return b.On(model.EventFriendAdd, hs...) }
func (b *Bot) OnFriendDel(hs ...interface{}) *Bot { return b.On(model.EventFriendDel, hs...) }

// OnGroupJoin / OnGroupLeave handle the bot being added to / removed from a group.
func (b *Bot) OnGroupJoin(hs ...interface{}) *Bot {
	return b.On(model.EventGroupAddRobot, hs...)
}
func (b *Bot) OnGroupLeave(hs ...interface{}) *Bot {
	return b.On(model.EventGroupDelRobot, hs...)
}

// OnGroupMemberAdd / OnGroupMemberRemove handle member changes (intent 1<<24).
func (b *Bot) OnGroupMemberAdd(hs ...interface{}) *Bot {
	return b.On(model.EventGroupMemberAdd, hs...)
}
func (b *Bot) OnGroupMemberRemove(hs ...interface{}) *Bot {
	return b.On(model.EventGroupMemberRemove, hs...)
}

// OnInteraction handles INTERACTION_CREATE (button taps, menu actions).
// For type 11/12 the bot must respond via c.Respond(code).
func (b *Bot) OnInteraction(hs ...interface{}) *Bot {
	return b.On(model.EventInteractionCreate, hs...)
}

// OnMessageAudited handles active-message audit results (intent 1<<27).
func (b *Bot) OnMessageAudited(hs ...interface{}) *Bot {
	b.On(model.EventMessageAuditPass, hs...)
	return b.On(model.EventMessageAuditReject, hs...)
}
