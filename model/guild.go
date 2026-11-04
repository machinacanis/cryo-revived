package model

// Guild (QQ 频道).
type Guild struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Icon        string `json:"icon,omitempty"`
	OwnerID     string `json:"owner_id,omitempty"`
	Owner       bool   `json:"owner,omitempty"`
	MemberCount int    `json:"member_count,omitempty"`
	MaxMembers  int    `json:"max_members,omitempty"`
	Description string `json:"description,omitempty"`
	JoinedAt    string `json:"joined_at,omitempty"`
}

// Channel (子频道).
type Channel struct {
	ID              string `json:"id"`
	GuildID         string `json:"guild_id"`
	Name            string `json:"name"`
	Type            int    `json:"type"`     // 0 text 2 voice 4 category 10005 live 10006 app 10007 forum
	SubType         int    `json:"sub_type"` // 0 chat 1 announce 2 guide 3 teamup
	Position        int    `json:"position"`
	ParentID        string `json:"parent_id,omitempty"`
	OwnerID         string `json:"owner_id,omitempty"`
	PrivateType     int    `json:"private_type,omitempty"`
	SpeakPermission int    `json:"speak_permission,omitempty"`
	ApplicationID   string `json:"application_id,omitempty"`
	Permissions     string `json:"permissions,omitempty"`
}

// Member of a guild.
type Member struct {
	User     *User    `json:"user,omitempty"`
	Nick     string   `json:"nick"`
	Roles    []string `json:"roles"`
	JoinedAt string   `json:"joined_at"`
	GuildID  string   `json:"guild_id,omitempty"` // MemberWithGuildID only
}

// ChannelMessageToCreate is the outgoing body of POST /channels/{id}/messages
// (legacy guild-channel format, no msg_type).
type ChannelMessageToCreate struct {
	Content   string      `json:"content,omitempty"`
	Embed     interface{} `json:"embed,omitempty"`
	ARK       interface{} `json:"ark,omitempty"`
	Markdown  *Markdown   `json:"markdown,omitempty"`
	Image     string      `json:"image,omitempty"`
	MsgID     string      `json:"msg_id,omitempty"`
	EventID   string      `json:"event_id,omitempty"`
	Reference *MessageRef `json:"message_reference,omitempty"`
}

// NewChannelMessage starts building a channel message.
func NewChannelMessage() *ChannelMessageToCreate { return &ChannelMessageToCreate{} }

func (m *ChannelMessageToCreate) Text(s string) *ChannelMessageToCreate { m.Content = s; return m }
func (m *ChannelMessageToCreate) MD(md *Markdown) *ChannelMessageToCreate {
	m.Markdown = md
	return m
}
func (m *ChannelMessageToCreate) ReplyTo(msgID string) *ChannelMessageToCreate {
	m.MsgID = msgID
	return m
}

// StreamMessage is one piece of a streaming (c2c) message.
type StreamMessage struct {
	InputMode   string `json:"input_mode,omitempty"`   // append|replace
	InputState  int    `json:"input_state,omitempty"`  // 1 generating, 10 done
	Index       int    `json:"index"`                  // from 0
	ContentType string `json:"content_type,omitempty"` // text|markdown
	ContentRaw  string `json:"content_raw"`
	EventID     string `json:"event_id,omitempty"`
	MsgID       string `json:"msg_id,omitempty"`
	StreamMsgID string `json:"stream_msg_id,omitempty"` // first chunk leaves empty, then reuse returned id
	MsgSeq      int    `json:"msg_seq,omitempty"`
	Wakeup      bool   `json:"is_wakeup,omitempty"`
}

// StreamMessageResult is returned per stream chunk.
type StreamMessageResult struct {
	ID          string `json:"id"`
	StreamMsgID string `json:"stream_msg_id,omitempty"`
	Timestamp   string `json:"timestamp,omitempty"`
}
