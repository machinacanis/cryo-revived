// Package model defines the wire types of the QQ Bot OpenAPI & Gateway protocol.
package model

import "encoding/json"

// OpCode of a gateway payload.
type OpCode int

const (
	OpDispatch       OpCode = 0  // server pushes an event
	OpHeartbeat      OpCode = 1  // client/server heartbeat
	OpIdentify       OpCode = 2  // client auth
	OpResume         OpCode = 6  // client resumes a session
	OpReconnect      OpCode = 7  // server asks client to reconnect
	OpInvalidSession OpCode = 9  // identify/resume rejected
	OpHello          OpCode = 10 // first frame after connect
	OpHeartbeatACK   OpCode = 11 // heartbeat acknowledged
	OpCallbackACK    OpCode = 12 // webhook ack
	OpCallbackVerify OpCode = 13 // webhook address validation
)

// Payload is the envelope used by both websocket frames and webhook callbacks.
type Payload struct {
	ID string          `json:"id,omitempty"` // event id, usable as event_id in replies
	Op OpCode          `json:"op"`
	D  json.RawMessage `json:"d,omitempty"`
	S  int             `json:"s,omitempty"` // sequence number of downstream frames
	T  string          `json:"t,omitempty"` // event type name when Op == OpDispatch
}

// Intent is a bit mask selecting which event families to subscribe.
type Intent uint32

const (
	IntentGuilds                Intent = 1 << 0
	IntentGuildMembers          Intent = 1 << 1
	IntentGuildMessages         Intent = 1 << 9 // private-domain bots only
	IntentGuildMessageReactions Intent = 1 << 10
	IntentDirectMessage         Intent = 1 << 12
	IntentGroupMemberEvent      Intent = 1 << 24
	IntentGroupAndC2C           Intent = 1 << 25
	IntentInteraction           Intent = 1 << 26
	IntentMessageAudit          Intent = 1 << 27
	IntentForums                Intent = 1 << 28 // private-domain bots only
	IntentAudio                 Intent = 1 << 29
	IntentPublicGuildMessages   Intent = 1 << 30
)

// Intent presets — 声明式权限组合，避免逐位拼装。
const (
	// IntentBasic: 单聊+群聊消息事件 + 互动事件（最小可用集合）
	IntentBasic Intent = IntentGroupAndC2C | IntentInteraction
	// IntentWithMembers: 基础集 + 群成员事件
	IntentWithMembers Intent = IntentBasic | IntentGroupMemberEvent
	// IntentAll: 全部已定义 intent（含需平台申请的位）
	IntentAll Intent = IntentGuilds | IntentGuildMembers | IntentGuildMessages |
		IntentGuildMessageReactions | IntentDirectMessage | IntentGroupMemberEvent |
		IntentGroupAndC2C | IntentInteraction | IntentMessageAudit | IntentForums |
		IntentAudio | IntentPublicGuildMessages
)

// Event type names carried by Payload.T.
const (
	EventReady              = "READY"
	EventResumed            = "RESUMED"
	EventC2CMessageCreate   = "C2C_MESSAGE_CREATE"
	EventFriendAdd          = "FRIEND_ADD"
	EventFriendDel          = "FRIEND_DEL"
	EventC2CMsgReject       = "C2C_MSG_REJECT"
	EventC2CMsgReceive      = "C2C_MSG_RECEIVE"
	EventGroupAtMessage     = "GROUP_AT_MESSAGE_CREATE"
	EventGroupMessage       = "GROUP_MESSAGE_CREATE"
	EventGroupAddRobot      = "GROUP_ADD_ROBOT"
	EventGroupDelRobot      = "GROUP_DEL_ROBOT"
	EventGroupMsgReject     = "GROUP_MSG_REJECT"
	EventGroupMsgReceive    = "GROUP_MSG_RECEIVE"
	EventGroupJoinRequest   = "GROUP_JOIN_REQUEST"
	EventGroupMemberAdd     = "GROUP_MEMBER_ADD"
	EventGroupMemberRemove  = "GROUP_MEMBER_REMOVE"
	EventInteractionCreate  = "INTERACTION_CREATE"
	EventGuildCreate        = "GUILD_CREATE"
	EventGuildUpdate        = "GUILD_UPDATE"
	EventGuildDelete        = "GUILD_DELETE"
	EventChannelCreate      = "CHANNEL_CREATE"
	EventChannelUpdate      = "CHANNEL_UPDATE"
	EventChannelDelete      = "CHANNEL_DELETE"
	EventAtMessage          = "AT_MESSAGE_CREATE"
	EventDirectMessage      = "DIRECT_MESSAGE_CREATE"
	EventMessageAuditPass   = "MESSAGE_AUDIT_PASS"
	EventMessageAuditReject = "MESSAGE_AUDIT_REJECT"
)
