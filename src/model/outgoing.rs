//! Outbound message body — `POST /v2/{users|groups}/{openid}/messages`.

use serde::Serialize;

/// Which content field of [`MessageToCreate`] takes effect.
pub mod msg_type {
    /// plain text in `content`
    pub const TEXT: i32 = 0;
    /// `markdown`
    pub const MARKDOWN: i32 = 2;
    /// `input_notify` — c2c only
    pub const INPUT_NOTIFY: i32 = 6;
    /// `media`
    pub const MEDIA: i32 = 7;
    /// `card`
    pub const CARD: i32 = 8;
}

/// Request body of c2c/group send-message endpoints.
///
/// Build it fluently:
///
/// ```no_run
/// # use cryo::model::{Button, Keyboard, MessageToCreate};
/// let mut m = MessageToCreate::new();
/// m.text("hello")
///     .keyboard(Keyboard::builder().row([Button::new("ok").callback("x")]).build());
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct MessageToCreate {
    #[serde(skip_serializing_if = "crate::model::is_zero")]
    pub msg_type: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<Markdown>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<super::keyboard::Keyboard>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub msg_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub event_id: String,
    #[serde(skip_serializing_if = "crate::model::is_zero")]
    pub msg_seq: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Media>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageRef>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_wakeup: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<MessageCard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_notify: Option<InputNotify>,
}

impl MessageToCreate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Plain-text body (msg_type 0). Clears `markdown` — the two are mutually
    /// exclusive per the platform rules.
    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        self.msg_type = msg_type::TEXT;
        self.content = s.into();
        self.markdown = None;
        self
    }

    /// Markdown body (msg_type 2).
    pub fn markdown(&mut self, s: impl Into<String>) -> &mut Self {
        self.msg_type = msg_type::MARKDOWN;
        self.markdown = Some(Markdown {
            content: s.into(),
            ..Default::default()
        });
        self.content.clear();
        self
    }

    /// Attach an explicit [`Markdown`] object (msg_type 2).
    pub fn markdown_obj(&mut self, md: Markdown) -> &mut Self {
        self.msg_type = msg_type::MARKDOWN;
        self.markdown = Some(md);
        self.content.clear();
        self
    }

    /// Inline keyboard — composable with any msg_type.
    pub fn keyboard(&mut self, k: super::keyboard::Keyboard) -> &mut Self {
        self.keyboard = Some(k);
        self
    }

    /// Rich media by its uploaded `file_info` (msg_type 7).
    pub fn file_info(&mut self, info: impl Into<String>) -> &mut Self {
        self.msg_type = msg_type::MEDIA;
        self.media = Some(Media {
            file_info: info.into(),
        });
        self
    }

    /// Tuwen link card (msg_type 8).
    pub fn tuwen_card(
        &mut self,
        title: impl Into<String>,
        desc: impl Into<String>,
        url: impl Into<String>,
        pic_url: impl Into<String>,
    ) -> &mut Self {
        self.msg_type = msg_type::CARD;
        self.card = Some(MessageCard {
            card_type: "tuwen".into(),
            content: [
                ("title", title.into()),
                ("description", desc.into()),
                ("url", url.into()),
                ("pic_url", pic_url.into()),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        });
        self
    }

    /// "typing..." indicator for `secs` (msg_type 6, c2c only, ≤60s).
    pub fn typing(&mut self, secs: i32) -> &mut Self {
        self.msg_type = msg_type::INPUT_NOTIFY;
        self.input_notify = Some(InputNotify {
            input_type: 1,
            input_second: secs,
        });
        self
    }

    /// Passive reply to a user message (`msg_id`).
    pub fn reply_to(&mut self, msg_id: impl Into<String>) -> &mut Self {
        self.msg_id = msg_id.into();
        self
    }

    /// Passive reply to an event (`event_id`).
    pub fn reply_to_event(&mut self, event_id: impl Into<String>) -> &mut Self {
        self.event_id = event_id.into();
        self
    }

    /// `msg_seq` for passive-reply dedupe; increment to re-reply.
    pub fn seq(&mut self, n: i64) -> &mut Self {
        self.msg_seq = n;
        self
    }

    /// Quote another message (REFIDX).
    pub fn quote(&mut self, message_id: impl Into<String>) -> &mut Self {
        self.message_reference = Some(MessageRef {
            message_id: message_id.into(),
        });
        self
    }

    /// Re-engagement message (c2c only).
    pub fn wakeup(&mut self) -> &mut Self {
        self.is_wakeup = true;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Markdown {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub force_verify_image_resource: bool,
}

/// Wraps the `file_info` returned by a files upload.
#[derive(Debug, Clone, Serialize)]
pub struct Media {
    pub file_info: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageRef {
    pub message_id: String,
}

/// msg_type 8 card.
#[derive(Debug, Clone, Serialize)]
pub struct MessageCard {
    #[serde(rename = "type")]
    pub card_type: String,
    pub content: std::collections::HashMap<String, String>,
}

/// "typing" indicator (msg_type 6).
#[derive(Debug, Clone, Serialize)]
pub struct InputNotify {
    /// always 1
    pub input_type: i32,
    /// ≤ 60
    pub input_second: i32,
}

/// Response of send-message endpoints.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PostMessageResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub ext_info: ExtInfo,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ExtInfo {
    #[serde(default)]
    pub ref_idx: String,
}
