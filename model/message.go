package model

import "encoding/json"

// User carried by events and member APIs.
type User struct {
	ID               string `json:"id,omitempty"`
	Username         string `json:"username,omitempty"`
	Avatar           string `json:"avatar,omitempty"`
	Bot              bool   `json:"bot,omitempty"`
	UnionOpenID      string `json:"union_openid,omitempty"`
	UnionUserAccount string `json:"union_user_account,omitempty"`
	UserOpenID       string `json:"user_openid,omitempty"`   // c2c scene
	MemberOpenID     string `json:"member_openid,omitempty"` // group scene
	MemberRole       string `json:"member_role,omitempty"`   // member|admin|owner
}

// Attachment of a received message.
type Attachment struct {
	URL          string `json:"url"`
	Filename     string `json:"filename,omitempty"`
	Width        int    `json:"width,omitempty"`
	Height       int    `json:"height,omitempty"`
	Size         int    `json:"size,omitempty"`
	ContentType  string `json:"content_type,omitempty"`   // voice|image/jpeg|image/png|image/gif|video/mp4|file
	VoiceWavURL  string `json:"voice_wav_url,omitempty"`  // converted wav for voice
	ASRReferText string `json:"asr_refer_text,omitempty"` // ASR result for voice
}

// MessageScene carries message indexes used for dedupe and referencing.
type MessageScene struct {
	Source string   `json:"source,omitempty"`
	Ext    []string `json:"ext,omitempty"` // key=value pairs: msg_idx / ref_msg_idx / auth_token
}

// ARKData of a structured card message (message_type=3).
type ARKData struct {
	Prompt  string                 `json:"prompt,omitempty"`
	ARKType string                 `json:"ark_type,omitempty"` // tuwen|feed|miniapp|map|contact_card|video_share|music_together|picture
	ARKName string                 `json:"ark_name,omitempty"`
	Fields  map[string]interface{} `json:"fields,omitempty"`
}

// MsgElement is a nested element of a quote message (message_type=103).
type MsgElement struct {
	MsgIdx      string `json:"msg_idx,omitempty"`
	Author      *User  `json:"author,omitempty"`
	MessageType int    `json:"message_type,omitempty"`
}

// C2CMessage is the d field of C2C_MESSAGE_CREATE.
type C2CMessage struct {
	ID           string        `json:"id"`
	Author       *User         `json:"author"`
	Content      string        `json:"content"`
	Timestamp    string        `json:"timestamp"`
	MessageType  int           `json:"message_type"` // 0=text 3=ark 101=parallel 102=records 103=quote
	MessageScene *MessageScene `json:"message_scene,omitempty"`
	Attachments  []*Attachment `json:"attachments,omitempty"`
	ARKData      *ARKData      `json:"ark_data,omitempty"`
	MsgElements  []*MsgElement `json:"msg_elements,omitempty"`
}

// GroupMessage is the d field of GROUP_AT_MESSAGE_CREATE / GROUP_MESSAGE_CREATE.
type GroupMessage struct {
	C2CMessage
	GroupOpenID string `json:"group_openid"`
}

// FriendEvent is the d field of FRIEND_ADD / FRIEND_DEL / C2C_MSG_*.
type FriendEvent struct {
	OpenID    string `json:"openid"`
	Timestamp string `json:"timestamp"`
}

// GroupBotEvent is the d field of GROUP_ADD_ROBOT / GROUP_DEL_ROBOT / GROUP_MSG_*.
type GroupBotEvent struct {
	GroupOpenID    string `json:"group_openid"`
	OpMemberOpenID string `json:"op_member_openid,omitempty"`
	Timestamp      string `json:"timestamp"`
}

// GroupMemberEvent is the d field of GROUP_MEMBER_ADD / REMOVE / GROUP_JOIN_REQUEST.
type GroupMemberEvent struct {
	GroupOpenID  string `json:"group_openid"`
	MemberOpenID string `json:"member_openid,omitempty"`
	Timestamp    string `json:"timestamp"`
}

// Interaction is the d field of INTERACTION_CREATE.
type Interaction struct {
	ID                string          `json:"id"`
	Type              int             `json:"type"`            // 11=msg button 12=quick menu 14=clear session
	Scene             string          `json:"scene,omitempty"` // c2c|group|guild
	ChatType          int             `json:"chat_type,omitempty"`
	Timestamp         string          `json:"timestamp,omitempty"`
	GuildID           string          `json:"guild_id,omitempty"`
	ChannelID         string          `json:"channel_id,omitempty"`
	UserOpenID        string          `json:"user_openid,omitempty"`
	GroupOpenID       string          `json:"group_openid,omitempty"`
	GroupMemberOpenID string          `json:"group_member_openid,omitempty"`
	Data              json.RawMessage `json:"data,omitempty"` // button: {type,resolved:{button_id,button_data}}
}

// MessageAudited is the d field of MESSAGE_AUDIT_PASS / REJECT.
type MessageAudited struct {
	AuditID    string `json:"audit_id"`
	MessageID  string `json:"message_id"`
	ChannelID  string `json:"channel_id"`
	GuildID    string `json:"guild_id"`
	AuditTime  string `json:"audit_time"`
	CreateTime string `json:"create_time"`
	SeqInChan  string `json:"seq_in_channel"`
}
