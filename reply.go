package cryo

import (
	"sync"

	"cryo/model"
)

// ReplyBuilder builds and sends a message bound to an event's chat target.
// Obtain one from the typed contexts: c.Reply().Text("hi").Send()
type ReplyBuilder struct {
	msg  *model.MessageToCreate
	send func(*model.MessageToCreate) (*model.PostMessageResult, error)
}

// Text sets plain-text content (msg_type 0).
func (r *ReplyBuilder) Text(s string) *ReplyBuilder { r.msg.Text(s); return r }

// Markdown sets markdown content (msg_type 2).
func (r *ReplyBuilder) Markdown(s string) *ReplyBuilder { r.msg.MarkdownText(s); return r }

// Keyboard attaches an inline keyboard.
func (r *ReplyBuilder) Keyboard(k *model.Keyboard) *ReplyBuilder {
	r.msg.WithKeyboard(k)
	return r
}

// FileInfo attaches uploaded rich media (msg_type 7).
func (r *ReplyBuilder) FileInfo(info string) *ReplyBuilder {
	r.msg.FileInfo(info)
	return r
}

// Card attaches a tuwen link card (msg_type 8).
func (r *ReplyBuilder) Card(title, desc, url, picURL string) *ReplyBuilder {
	r.msg.TuwenCard(title, desc, url, picURL)
	return r
}

// Typing sends a typing indicator (c2c, msg_type 6).
func (r *ReplyBuilder) Typing(secs int) *ReplyBuilder { r.msg.Typing(secs); return r }

// Quote attaches a message reference.
func (r *ReplyBuilder) Quote(messageID string) *ReplyBuilder {
	r.msg.Quote(messageID)
	return r
}

// Seq overrides msg_seq (default: auto-incremented per source message).
func (r *ReplyBuilder) Seq(n int) *ReplyBuilder { r.msg.Seq(n); return r }

// Send delivers the built message.
func (r *ReplyBuilder) Send() error {
	_, err := r.send(r.msg)
	return err
}

// SendResult delivers the message and returns the server response (id/timestamp/ext_info).
func (r *ReplyBuilder) SendResult() (*model.PostMessageResult, error) { return r.send(r.msg) }

// --- bound Reply() on typed contexts ---

// Reply builds a passive reply to the user (msg_id + auto msg_seq).
func (c *C2CMessageCtx) Reply() *ReplyBuilder {
	m := model.NewMessage().ReplyTo(c.Message.ID).Seq(seqOf(c.Message.ID))
	return &ReplyBuilder{msg: m, send: func(o *model.MessageToCreate) (*model.PostMessageResult, error) {
		return c.API().User(c.Message.Author.UserOpenID).SendMessage(c.Context, o)
	}}
}

// Active builds an active (non-reply) message to this user.
func (c *C2CMessageCtx) Active() *ReplyBuilder {
	return &ReplyBuilder{msg: model.NewMessage(), send: func(o *model.MessageToCreate) (*model.PostMessageResult, error) {
		return c.API().User(c.Message.Author.UserOpenID).SendMessage(c.Context, o)
	}}
}

// Reply builds a passive reply into the group.
func (c *GroupMessageCtx) Reply() *ReplyBuilder {
	m := model.NewMessage().ReplyTo(c.Message.ID).Seq(seqOf(c.Message.ID))
	return &ReplyBuilder{msg: m, send: func(o *model.MessageToCreate) (*model.PostMessageResult, error) {
		return c.API().Group(c.Message.GroupOpenID).SendMessage(c.Context, o)
	}}
}

// Active builds an active (non-reply) message to this group.
func (c *GroupMessageCtx) Active() *ReplyBuilder {
	return &ReplyBuilder{msg: model.NewMessage(), send: func(o *model.MessageToCreate) (*model.PostMessageResult, error) {
		return c.API().Group(c.Message.GroupOpenID).SendMessage(c.Context, o)
	}}
}

// Respond answers an interaction (code 0-5, see Client.RespondInteraction).
func (c *InteractionCtx) Respond(code int) error {
	return c.API().RespondInteraction(c.Context, c.Interaction.ID, code)
}

// seqOf returns an incrementing counter per source msg_id so repeated
// replies to the same message don't collide on msg_id+msg_seq.
func seqOf(msgID string) int {
	seqMu.Lock()
	defer seqMu.Unlock()
	seqN[msgID]++
	return seqN[msgID]
}

var (
	seqMu sync.Mutex
	seqN  = map[string]int{}
)
