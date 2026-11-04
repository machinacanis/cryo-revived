//! Gateway wire protocol: payload envelope, opcodes, intents, event names.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Gateway payload envelope used by websocket frames (and webhook callbacks).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Payload {
    /// Event id, usable as `event_id` in passive replies.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// Opcode, see [`op`].
    pub op: i32,
    /// Event data — opaque until decoded into a concrete event body.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub d: Value,
    /// Sequence number of downstream frames (heartbeat/resume bookkeeping).
    #[serde(default, skip_serializing_if = "crate::model::is_zero")]
    pub s: i64,
    /// Event type name when `op == op::DISPATCH`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub t: String,
}

/// Gateway opcodes.
pub mod op {
    pub const DISPATCH: i32 = 0;
    pub const HEARTBEAT: i32 = 1;
    pub const IDENTIFY: i32 = 2;
    pub const RESUME: i32 = 6;
    pub const RECONNECT: i32 = 7;
    pub const INVALID_SESSION: i32 = 9;
    pub const HELLO: i32 = 10;
    pub const HEARTBEAT_ACK: i32 = 11;
    pub const CALLBACK_ACK: i32 = 12;
    pub const CALLBACK_VERIFY: i32 = 13;
}

/// Event type names carried by [`Payload::t`].
pub mod event {
    pub const READY: &str = "READY";
    pub const RESUMED: &str = "RESUMED";
    pub const C2C_MESSAGE_CREATE: &str = "C2C_MESSAGE_CREATE";
    pub const FRIEND_ADD: &str = "FRIEND_ADD";
    pub const FRIEND_DEL: &str = "FRIEND_DEL";
    pub const C2C_MSG_REJECT: &str = "C2C_MSG_REJECT";
    pub const C2C_MSG_RECEIVE: &str = "C2C_MSG_RECEIVE";
    pub const GROUP_AT_MESSAGE: &str = "GROUP_AT_MESSAGE_CREATE";
    pub const GROUP_MESSAGE: &str = "GROUP_MESSAGE_CREATE";
    pub const GROUP_ADD_ROBOT: &str = "GROUP_ADD_ROBOT";
    pub const GROUP_DEL_ROBOT: &str = "GROUP_DEL_ROBOT";
    pub const GROUP_MSG_REJECT: &str = "GROUP_MSG_REJECT";
    pub const GROUP_MSG_RECEIVE: &str = "GROUP_MSG_RECEIVE";
    pub const GROUP_JOIN_REQUEST: &str = "GROUP_JOIN_REQUEST";
    pub const GROUP_MEMBER_ADD: &str = "GROUP_MEMBER_ADD";
    pub const GROUP_MEMBER_REMOVE: &str = "GROUP_MEMBER_REMOVE";
    pub const INTERACTION_CREATE: &str = "INTERACTION_CREATE";
    pub const GUILD_CREATE: &str = "GUILD_CREATE";
    pub const GUILD_UPDATE: &str = "GUILD_UPDATE";
    pub const GUILD_DELETE: &str = "GUILD_DELETE";
    pub const CHANNEL_CREATE: &str = "CHANNEL_CREATE";
    pub const CHANNEL_UPDATE: &str = "CHANNEL_UPDATE";
    pub const CHANNEL_DELETE: &str = "CHANNEL_DELETE";
    pub const AT_MESSAGE: &str = "AT_MESSAGE_CREATE";
    pub const DIRECT_MESSAGE: &str = "DIRECT_MESSAGE_CREATE";
    pub const MESSAGE_AUDIT_PASS: &str = "MESSAGE_AUDIT_PASS";
    pub const MESSAGE_AUDIT_REJECT: &str = "MESSAGE_AUDIT_REJECT";
}

/// Bit mask selecting which event families the bot subscribes to.
///
/// Every [`crate::Config`] must declare one explicitly — there is no
/// silent default. Use the presets [`Intent::BASIC`], [`Intent::WITH_MEMBERS`]
/// or [`Intent::ALL`], or compose bits with `|`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Intent(pub u64);

impl Intent {
    pub const GUILDS: Self = Self(1 << 0);
    pub const GUILD_MEMBERS: Self = Self(1 << 1);
    /// private-domain bots only
    pub const GUILD_MESSAGES: Self = Self(1 << 9);
    pub const GUILD_MESSAGE_REACTIONS: Self = Self(1 << 10);
    pub const DIRECT_MESSAGE: Self = Self(1 << 12);
    pub const GROUP_MEMBER_EVENT: Self = Self(1 << 24);
    pub const GROUP_AND_C2C: Self = Self(1 << 25);
    pub const INTERACTION: Self = Self(1 << 26);
    pub const MESSAGE_AUDIT: Self = Self(1 << 27);
    /// private-domain bots only
    pub const FORUMS: Self = Self(1 << 28);
    pub const AUDIO: Self = Self(1 << 29);
    pub const PUBLIC_GUILD_MESSAGES: Self = Self(1 << 30);

    /// 单聊+群聊消息事件 + 互动事件（最小可用集合）.
    pub const BASIC: Self = Self(Self::GROUP_AND_C2C.0 | Self::INTERACTION.0);
    /// [`Intent::BASIC`] + 群成员事件.
    pub const WITH_MEMBERS: Self = Self(Self::BASIC.0 | Self::GROUP_MEMBER_EVENT.0);
    /// 全部已定义 intent（含需平台申请的位）.
    pub const ALL: Self = Self(
        Self::GUILDS.0
            | Self::GUILD_MEMBERS.0
            | Self::GUILD_MESSAGES.0
            | Self::GUILD_MESSAGE_REACTIONS.0
            | Self::DIRECT_MESSAGE.0
            | Self::GROUP_MEMBER_EVENT.0
            | Self::GROUP_AND_C2C.0
            | Self::INTERACTION.0
            | Self::MESSAGE_AUDIT.0
            | Self::FORUMS.0
            | Self::AUDIO.0
            | Self::PUBLIC_GUILD_MESSAGES.0,
    );

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Intent {
    type Output = Intent;
    fn bitor(self, rhs: Intent) -> Intent {
        Intent(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Intent {
    fn bitor_assign(&mut self, rhs: Intent) {
        self.0 |= rhs.0;
    }
}
