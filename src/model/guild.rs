//! Guild (频道) domain types plus channel-message and streaming payloads.

use serde::{Deserialize, Serialize};

/// QQ 频道.
#[derive(Debug, Clone, Deserialize)]
pub struct Guild {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub owner: bool,
    #[serde(default)]
    pub member_count: i64,
    #[serde(default)]
    pub max_members: i64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub joined_at: String,
}

/// 子频道.
#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: String,
    #[serde(default)]
    pub guild_id: String,
    #[serde(default)]
    pub name: String,
    /// 0 text 2 voice 4 category 10005 live 10006 app 10007 forum
    #[serde(default, rename = "type")]
    pub channel_type: i32,
    /// 0 chat 1 announce 2 guide 3 teamup
    #[serde(default)]
    pub sub_type: i32,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub private_type: i32,
    #[serde(default)]
    pub speak_permission: i32,
    #[serde(default)]
    pub application_id: String,
    #[serde(default)]
    pub permissions: String,
}

/// Guild member.
#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    #[serde(default)]
    pub user: Option<super::User>,
    #[serde(default)]
    pub nick: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub joined_at: String,
    /// MemberWithGuildID only
    #[serde(default)]
    pub guild_id: String,
}

/// Outgoing body of `POST /channels/{id}/messages`
/// (legacy guild-channel format, no msg_type).
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChannelMessageToCreate {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ark: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<super::Markdown>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub image: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub msg_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<super::MessageRef>,
}

impl ChannelMessageToCreate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        self.content = s.into();
        self
    }

    pub fn markdown(&mut self, md: super::Markdown) -> &mut Self {
        self.markdown = Some(md);
        self
    }

    pub fn reply_to(&mut self, msg_id: impl Into<String>) -> &mut Self {
        self.msg_id = msg_id.into();
        self
    }
}

/// One piece of a streaming (typewriter) c2c message.
#[derive(Debug, Clone, Default, Serialize)]
pub struct StreamMessage {
    /// append|replace
    #[serde(skip_serializing_if = "String::is_empty")]
    pub input_mode: String,
    /// 1 generating, 10 done
    #[serde(skip_serializing_if = "crate::model::is_zero")]
    pub input_state: i32,
    /// from 0
    pub index: i32,
    /// text|markdown
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content_type: String,
    pub content_raw: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub event_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub msg_id: String,
    /// first chunk leaves empty, then reuse the returned id
    #[serde(skip_serializing_if = "String::is_empty")]
    pub stream_msg_id: String,
    #[serde(skip_serializing_if = "crate::model::is_zero")]
    pub msg_seq: i64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_wakeup: bool,
}

/// Returned per stream chunk.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamMessageResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub stream_msg_id: String,
    #[serde(default)]
    pub timestamp: String,
}
