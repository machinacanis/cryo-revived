//! Inbound event bodies — the `d` field of dispatch payloads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// User carried by events and member APIs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub bot: bool,
    #[serde(default)]
    pub union_openid: String,
    #[serde(default)]
    pub union_user_account: String,
    /// c2c scene
    #[serde(default)]
    pub user_openid: String,
    /// group scene
    #[serde(default)]
    pub member_openid: String,
    /// member|admin|owner
    #[serde(default)]
    pub member_role: String,
}

/// Attachment of a received message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub url: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    #[serde(default)]
    pub size: i64,
    /// voice | image/jpeg | image/png | image/gif | video/mp4 | file
    #[serde(default)]
    pub content_type: String,
    /// converted wav for voice
    #[serde(default)]
    pub voice_wav_url: String,
    /// ASR result for voice
    #[serde(default)]
    pub asr_refer_text: String,
}

/// Message indexes used for referencing (and cross-account identity probing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageScene {
    #[serde(default)]
    pub source: String,
    /// key=value pairs: msg_idx / ref_msg_idx / auth_token
    #[serde(default)]
    pub ext: Vec<String>,
}

/// Structured card payload (message_type=3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArkData {
    #[serde(default)]
    pub prompt: String,
    /// tuwen|feed|miniapp|map|contact_card|video_share|music_together|picture
    #[serde(default)]
    pub ark_type: String,
    #[serde(default)]
    pub ark_name: String,
    #[serde(default)]
    pub fields: Value,
}

/// Nested element of a quote message (message_type=103).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgElement {
    #[serde(default)]
    pub msg_idx: String,
    #[serde(default)]
    pub author: Option<User>,
    #[serde(default)]
    pub message_type: i32,
}

/// `d` of `C2C_MESSAGE_CREATE`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2CMessage {
    pub id: String,
    pub author: User,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timestamp: String,
    /// 0=text 3=ark 101=parallel 102=records 103=quote
    #[serde(default)]
    pub message_type: i32,
    #[serde(default)]
    pub message_scene: Option<MessageScene>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub ark_data: Option<ArkData>,
    #[serde(default)]
    pub msg_elements: Vec<MsgElement>,
}

/// `d` of `GROUP_AT_MESSAGE_CREATE` / `GROUP_MESSAGE_CREATE`.
///
/// Derefs to [`C2CMessage`] so `msg.content`, `msg.author` etc. work directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessage {
    #[serde(flatten)]
    pub base: C2CMessage,
    pub group_openid: String,
}

impl std::ops::Deref for GroupMessage {
    type Target = C2CMessage;
    fn deref(&self) -> &C2CMessage {
        &self.base
    }
}

/// `d` of `FRIEND_ADD` / `FRIEND_DEL` / `C2C_MSG_*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendEvent {
    pub openid: String,
    #[serde(default)]
    pub timestamp: String,
}

/// `d` of `GROUP_ADD_ROBOT` / `GROUP_DEL_ROBOT` / `GROUP_MSG_*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupBotEvent {
    pub group_openid: String,
    #[serde(default)]
    pub op_member_openid: String,
    #[serde(default)]
    pub timestamp: String,
}

/// `d` of `GROUP_MEMBER_ADD` / `GROUP_MEMBER_REMOVE` / `GROUP_JOIN_REQUEST`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberEvent {
    pub group_openid: String,
    #[serde(default)]
    pub member_openid: String,
    #[serde(default)]
    pub timestamp: String,
}

/// `d` of `INTERACTION_CREATE`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: String,
    /// 11=msg button 12=quick menu 14=clear session
    #[serde(default, rename = "type")]
    pub interaction_type: i32,
    /// c2c|group|guild
    #[serde(default)]
    pub scene: String,
    #[serde(default)]
    pub chat_type: i64,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub guild_id: String,
    #[serde(default)]
    pub channel_id: String,
    #[serde(default)]
    pub user_openid: String,
    #[serde(default)]
    pub group_openid: String,
    #[serde(default)]
    pub group_member_openid: String,
    /// button: {type, resolved:{button_id, button_data}}
    #[serde(default)]
    pub data: Value,
}

/// `d` of `MESSAGE_AUDIT_PASS` / `MESSAGE_AUDIT_REJECT`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAudited {
    pub audit_id: String,
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub channel_id: String,
    #[serde(default)]
    pub guild_id: String,
    #[serde(default)]
    pub audit_time: String,
    #[serde(default)]
    pub create_time: String,
    #[serde(default)]
    pub seq_in_channel: String,
}
