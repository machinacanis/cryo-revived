package model

// MsgType selects which content field of an outgoing message takes effect.
type MsgType int

const (
	MsgText        MsgType = 0 // Content
	MsgMarkdown    MsgType = 2 // Markdown
	MsgInputNotify MsgType = 6 // InputNotify, c2c only
	MsgMedia       MsgType = 7 // Media
	MsgCard        MsgType = 8 // Card
)

// MessageToCreate is the request body of c2c/group send-message endpoints.
type MessageToCreate struct {
	Type        MsgType      `json:"msg_type,omitempty"`
	Content     string       `json:"content,omitempty"`
	Markdown    *Markdown    `json:"markdown,omitempty"`
	Keyboard    *Keyboard    `json:"keyboard,omitempty"`
	MsgID       string       `json:"msg_id,omitempty"`
	EventID     string       `json:"event_id,omitempty"`
	MsgSeq      int          `json:"msg_seq,omitempty"`
	Media       *Media       `json:"media,omitempty"`
	Reference   *MessageRef  `json:"message_reference,omitempty"`
	Wakeup      bool         `json:"is_wakeup,omitempty"`
	Card        *MessageCard `json:"card,omitempty"`
	InputNotify *InputNotify `json:"input_notify,omitempty"`
}

// NewMessage starts building an outgoing message.
//
//	api.User(id).Send(ctx, model.Message().Text("hello").ReplyTo(msgID))
func NewMessage() *MessageToCreate { return &MessageToCreate{} }

// Text sets a plain-text body (msg_type 0). Clears Markdown to satisfy
// the "content and markdown are mutually exclusive" rule.
func (m *MessageToCreate) Text(s string) *MessageToCreate {
	m.Type, m.Content, m.Markdown = MsgText, s, nil
	return m
}

// MarkdownText sets a markdown body (msg_type 2).
func (m *MessageToCreate) MarkdownText(s string) *MessageToCreate {
	m.Type, m.Markdown, m.Content = MsgMarkdown, &Markdown{Content: s}, ""
	return m
}

// MD attaches a Markdown object (msg_type 2).
func (m *MessageToCreate) MD(md *Markdown) *MessageToCreate {
	m.Type, m.Markdown, m.Content = MsgMarkdown, md, ""
	return m
}

// WithKeyboard attaches an inline keyboard. Works with any msg_type.
func (m *MessageToCreate) WithKeyboard(k *Keyboard) *MessageToCreate { m.Keyboard = k; return m }

// FileInfo attaches rich media by its uploaded file_info (msg_type 7).
func (m *MessageToCreate) FileInfo(info string) *MessageToCreate {
	m.Type, m.Media = MsgMedia, &Media{FileInfo: info}
	return m
}

// TuwenCard attaches a link card (msg_type 8).
func (m *MessageToCreate) TuwenCard(title, desc, url, picURL string) *MessageToCreate {
	m.Type = MsgCard
	m.Card = &MessageCard{Type: "tuwen", Content: map[string]string{
		"title": title, "description": desc, "url": url, "pic_url": picURL,
	}}
	return m
}

// Typing sends a "typing..." indicator for secs (msg_type 6, c2c only, ≤60s).
func (m *MessageToCreate) Typing(secs int) *MessageToCreate {
	m.Type, m.InputNotify = MsgInputNotify, &InputNotify{InputType: 1, InputSecond: secs}
	return m
}

// ReplyTo marks the message as a passive reply to a user message (msg_id).
func (m *MessageToCreate) ReplyTo(msgID string) *MessageToCreate { m.MsgID = msgID; return m }

// ReplyToEvent marks the message as a passive reply to an event (event_id).
func (m *MessageToCreate) ReplyToEvent(eventID string) *MessageToCreate {
	m.EventID = eventID
	return m
}

// Seq sets msg_seq for dedupe; increment to reply the same message again.
func (m *MessageToCreate) Seq(n int) *MessageToCreate { m.MsgSeq = n; return m }

// Quote attaches a reference to another message (REFIDX id).
func (m *MessageToCreate) Quote(messageID string) *MessageToCreate {
	m.Reference = &MessageRef{MessageID: messageID}
	return m
}

// AsWakeup marks the message as a re-engagement message (c2c only).
func (m *MessageToCreate) AsWakeup() *MessageToCreate { m.Wakeup = true; return m }

// Markdown body.
type Markdown struct {
	Content                  string `json:"content,omitempty"`
	ForceVerifyImageResource bool   `json:"force_verify_image_resource,omitempty"`
}

// Media wraps the file_info returned by a files upload.
type Media struct {
	FileInfo string `json:"file_info"`
}

// MessageRef quotes another message.
type MessageRef struct {
	MessageID string `json:"message_id"`
}

// MessageCard is a msg_type 8 card.
type MessageCard struct {
	Type    string            `json:"type"`
	Content map[string]string `json:"content"`
}

// InputNotify shows the "typing" indicator (msg_type 6).
type InputNotify struct {
	InputType   int `json:"input_type"`   // always 1
	InputSecond int `json:"input_second"` // ≤ 60
}

// PostMessageResult is the response of send-message endpoints.
type PostMessageResult struct {
	ID        string `json:"id"`
	Timestamp string `json:"timestamp"`
	ExtInfo   struct {
		RefIdx string `json:"ref_idx"`
	} `json:"ext_info,omitempty"`
}
