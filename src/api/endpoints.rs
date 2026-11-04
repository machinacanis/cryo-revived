//! Identity-bound endpoint handles.
//!
//! Each method on [`Client`] returns a small handle bound to one openid/id,
//! so calls read naturally: `api.user(openid).send_message(&msg).await`.

use serde::{Deserialize, Serialize};

use super::client::Client;
use crate::model;
use crate::Result;

impl Client {
    /// Endpoint handle of one c2c user.
    pub fn user(&self, openid: impl Into<String>) -> UserApi {
        UserApi {
            c: self.clone(),
            base: format!("/v2/users/{}", openid.into()),
        }
    }

    /// Endpoint handle of one QQ group.
    pub fn group(&self, openid: impl Into<String>) -> GroupApi {
        GroupApi {
            c: self.clone(),
            base: format!("/v2/groups/{}", openid.into()),
        }
    }

    /// Endpoint handle of one guild sub-channel.
    pub fn channel(&self, id: impl Into<String>) -> ChannelApi {
        ChannelApi {
            c: self.clone(),
            id: id.into(),
        }
    }

    /// Endpoint handle of one guild.
    pub fn guild(&self, id: impl Into<String>) -> GuildApi {
        GuildApi {
            c: self.clone(),
            id: id.into(),
        }
    }

    // --- bot self ---

    /// The bot's own profile.
    pub async fn me(&self) -> Result<model::User> {
        self.get("/users/@me", &[]).await
    }

    /// Guilds the bot joined.
    pub async fn get_guilds(&self) -> Result<Vec<model::Guild>> {
        self.get("/users/@me/guilds", &[]).await
    }

    /// A share link carrying `callback_data`.
    pub async fn generate_url_link(&self, callback_data: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Out {
            url: String,
        }
        let out: Out = self
            .post(
                "/v2/generate_url_link",
                &serde_json::json!({ "callback_data": callback_data }),
            )
            .await?;
        Ok(out.url)
    }

    // --- gateway ---

    /// The generic websocket entry point.
    pub async fn get_wss(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct Out {
            url: String,
        }
        let out: Out = self.get("/gateway", &[]).await?;
        Ok(out.url)
    }

    /// Recommended shard count, entry point and session-start limits.
    pub async fn get_wss_shard(&self) -> Result<WssShard> {
        self.get("/gateway/bot", &[]).await
    }

    /// Acks an `INTERACTION_CREATE` event (required for type 11/12).
    /// code: 0 ok, 1 failed, 2 frequent, 3 repeated, 4 no perm, 5 admin only.
    pub async fn respond_interaction(&self, interaction_id: &str, code: i32) -> Result<()> {
        self.put(
            &format!("/interactions/{interaction_id}"),
            &serde_json::json!({ "code": code }),
        )
        .await
    }
}

/// Sharded websocket entry + session limits.
#[derive(Debug, Deserialize)]
pub struct WssShard {
    pub url: String,
    #[serde(default)]
    pub shards: i64,
    #[serde(default)]
    pub session_start_limit: SessionStartLimit,
}

#[derive(Debug, Default, Deserialize)]
pub struct SessionStartLimit {
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub remaining: i64,
    #[serde(default)]
    pub reset_after: i64,
    #[serde(default)]
    pub max_concurrency: i64,
}

#[derive(Serialize)]
struct FileReq {
    /// 1 image 2 video 3 voice 4 file
    file_type: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    url: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    srv_send_msg: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    upload_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    file_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    file_data: String,
}

#[derive(Debug, Deserialize)]
pub struct FileInfo {
    pub file_info: String,
    #[serde(default)]
    pub ttl: i64,
    #[serde(default)]
    pub id: String,
}

// --- c2c user endpoints ---

pub struct UserApi {
    c: Client,
    base: String,
}

impl UserApi {
    /// Deliver a message to this user.
    pub async fn send_message(
        &self,
        m: &model::MessageToCreate,
    ) -> Result<model::PostMessageResult> {
        self.c.post(&format!("{}/messages", self.base), m).await
    }

    /// Shortcut for `send_message(text)`.
    pub async fn send_text(&self, s: impl Into<String>) -> Result<model::PostMessageResult> {
        let mut m = model::MessageToCreate::new();
        m.text(s);
        self.send_message(&m).await
    }

    /// Withdraw a message sent within 2 minutes.
    pub async fn recall_message(&self, msg_id: &str) -> Result<()> {
        self.c
            .delete(&format!("{}/messages/{msg_id}", self.base), &[])
            .await
    }

    /// One chunk of a streaming (typewriter) markdown message.
    pub async fn send_stream_message(
        &self,
        s: &model::StreamMessage,
    ) -> Result<model::StreamMessageResult> {
        self.c
            .post(&format!("{}/stream_messages", self.base), s)
            .await
    }

    /// Upload media by URL; returns `file_info` for [`model::MessageToCreate::file_info`].
    pub async fn upload_file_url(&self, file_type: i32, url: &str) -> Result<String> {
        let f: FileInfo = self
            .c
            .post(
                &format!("{}/files", self.base),
                &FileReq {
                    file_type,
                    url: url.into(),
                    srv_send_msg: false,
                    upload_id: String::new(),
                    file_name: String::new(),
                    file_data: String::new(),
                },
            )
            .await?;
        Ok(f.file_info)
    }

    /// Upload raw bytes (≤5MB); returns `file_info`.
    pub async fn upload_file_data(
        &self,
        file_type: i32,
        name: &str,
        data: &[u8],
    ) -> Result<String> {
        let f: FileInfo = self
            .c
            .post(
                &format!("{}/files", self.base),
                &FileReq {
                    file_type,
                    url: String::new(),
                    srv_send_msg: false,
                    upload_id: String::new(),
                    file_name: name.into(),
                    file_data: String::from_utf8_lossy(data).into_owned(),
                },
            )
            .await?;
        Ok(f.file_info)
    }
}

// --- group endpoints ---

pub struct GroupApi {
    c: Client,
    base: String,
}

impl GroupApi {
    pub async fn send_message(
        &self,
        m: &model::MessageToCreate,
    ) -> Result<model::PostMessageResult> {
        self.c.post(&format!("{}/messages", self.base), m).await
    }

    pub async fn send_text(&self, s: impl Into<String>) -> Result<model::PostMessageResult> {
        let mut m = model::MessageToCreate::new();
        m.text(s);
        self.send_message(&m).await
    }

    pub async fn recall_message(&self, msg_id: &str) -> Result<()> {
        self.c
            .delete(&format!("{}/messages/{msg_id}", self.base), &[])
            .await
    }

    /// Upload media by URL (not shared with c2c uploads).
    pub async fn upload_file_url(&self, file_type: i32, url: &str) -> Result<String> {
        let f: FileInfo = self
            .c
            .post(
                &format!("{}/files", self.base),
                &FileReq {
                    file_type,
                    url: url.into(),
                    srv_send_msg: false,
                    upload_id: String::new(),
                    file_name: String::new(),
                    file_data: String::new(),
                },
            )
            .await?;
        Ok(f.file_info)
    }

    /// Group profile.
    pub async fn get_info(&self) -> Result<GroupInfo> {
        self.c.get(&format!("{}/info", self.base), &[]).await
    }

    /// One page of members; empty `cursor` starts from the head.
    pub async fn get_members(&self, cursor: &str) -> Result<MemberPage> {
        self.c
            .get(&format!("{}/members", self.base), &[("cursor", cursor)])
            .await
    }

    /// One member's profile.
    pub async fn get_member(&self, member_openid: &str) -> Result<GroupMember> {
        self.c
            .get(&format!("{}/members/{member_openid}", self.base), &[])
            .await
    }

    /// Remove up to 20 members; `blacklist` also bans them.
    pub async fn remove_members(&self, blacklist: bool, member_openids: &[String]) -> Result<()> {
        self.c
            .post::<serde::de::IgnoredAny>(
                &format!("{}/batch_remove_members", self.base),
                &serde_json::json!({
                    "member_openids": member_openids,
                    "add_to_member_blacklist": blacklist,
                }),
            )
            .await?;
        Ok(())
    }

    /// Member mute states; `op`: "add"|"update"|"del".
    pub async fn mute_members(
        &self,
        op: &str,
        expire_at: &str,
        member_openids: &[String],
    ) -> Result<()> {
        let members: Vec<_> = member_openids
            .iter()
            .map(|id| {
                serde_json::json!({
                    "op": op,
                    "member_openid": id,
                    "mute_expire_at": expire_at,
                })
            })
            .collect();
        self.c
            .post::<serde::de::IgnoredAny>(
                &format!("{}/restrict_chat_setting", self.base),
                &serde_json::json!({ "members": members }),
            )
            .await?;
        Ok(())
    }
}

// --- channel / guild endpoints ---

pub struct ChannelApi {
    c: Client,
    id: String,
}

impl ChannelApi {
    /// Post to a guild channel (legacy payload). Returns the message id.
    pub async fn send_message(&self, m: &model::ChannelMessageToCreate) -> Result<String> {
        #[derive(Deserialize)]
        struct Out {
            #[serde(default)]
            id: String,
        }
        let out: Out = self
            .c
            .post(&format!("/channels/{}/messages", self.id), m)
            .await?;
        Ok(out.id)
    }

    pub async fn recall_message(&self, msg_id: &str) -> Result<()> {
        self.c
            .delete(
                &format!("/channels/{}/messages/{msg_id}", self.id),
                &[("hidetip", "false")],
            )
            .await
    }

    pub async fn get_info(&self) -> Result<model::Channel> {
        self.c.get(&format!("/channels/{}", self.id), &[]).await
    }
}

pub struct GuildApi {
    c: Client,
    id: String,
}

impl GuildApi {
    pub async fn get_info(&self) -> Result<model::Guild> {
        self.c.get(&format!("/guilds/{}", self.id), &[]).await
    }

    pub async fn get_channels(&self) -> Result<Vec<model::Channel>> {
        self.c
            .get(&format!("/guilds/{}/channels", self.id), &[])
            .await
    }

    pub async fn get_members(&self, limit: i32, after: &str) -> Result<Vec<model::Member>> {
        let limit = limit.to_string();
        self.c
            .get(
                &format!("/guilds/{}/members", self.id),
                &[("limit", limit.as_str()), ("after", after)],
            )
            .await
    }
}

// --- shared response types ---

#[derive(Debug, Deserialize)]
pub struct GroupInfo {
    pub group_openid: String,
    #[serde(default)]
    pub group_name: String,
    #[serde(default)]
    pub group_finger_memo: String,
    #[serde(default)]
    pub group_class_text: String,
    #[serde(default)]
    pub group_tags: Vec<String>,
    #[serde(default)]
    pub group_member_num: i64,
}

#[derive(Debug, Deserialize)]
pub struct GroupMember {
    pub member_openid: String,
    #[serde(default)]
    pub nick: String,
    #[serde(default)]
    pub join_timestamp: String,
    #[serde(default)]
    pub member_role: String,
}

#[derive(Debug, Deserialize)]
pub struct MemberPage {
    #[serde(default)]
    pub members: Vec<GroupMember>,
    #[serde(default)]
    pub next_cursor: String,
}
