//! Inline keyboards — template reference or custom button layout.

use serde::Serialize;

/// An inline keyboard: either a platform template (`id`) or a custom layout.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Keyboard {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<KeyboardContent>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct KeyboardContent {
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Button {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// same group greys out others after one click (action type 1)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_data: Option<RenderData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderData {
    /// ≤ 10 chars
    pub label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub visited_label: String,
    /// 0 grey 1 blue 3 white-red 4 blue-white
    pub style: i32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Action {
    /// 0=jump 1=callback 2=command
    #[serde(rename = "type")]
    pub action_type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Permission>,
    /// required for type 1/2
    #[serde(skip_serializing_if = "String::is_empty")]
    pub data: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unsupport_tips: String,
    /// command button auto-send (c2c, qq>=8983)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub enter: bool,
    /// command with quote
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reply: bool,
    /// 1 = open image picker (overrides enter)
    #[serde(skip_serializing_if = "crate::model::is_zero")]
    pub anchor: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modal: Option<Modal>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Permission {
    /// 0=specified users 1=admin 2=everyone
    #[serde(rename = "type")]
    pub perm_type: i32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub specify_user_ids: Vec<String>,
    /// guild only
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub specify_role_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Modal {
    /// ≤40 chars, no URL
    pub content: String,
    /// ≤4 chars
    #[serde(skip_serializing_if = "String::is_empty")]
    pub confirm_text: String,
    /// ≤4 chars
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cancel_text: String,
}

impl Keyboard {
    /// Reference a platform keyboard template by id.
    pub fn by_id(id: impl Into<String>) -> Self {
        Keyboard {
            id: id.into(),
            content: None,
        }
    }

    /// Start a custom keyboard layout.
    pub fn builder() -> KeyboardBuilder {
        KeyboardBuilder { rows: Vec::new() }
    }
}

pub struct KeyboardBuilder {
    rows: Vec<Row>,
}

impl KeyboardBuilder {
    /// Append a row of buttons.
    pub fn row(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
        self.rows.push(Row {
            buttons: buttons.into_iter().collect(),
        });
        self
    }

    pub fn build(self) -> Keyboard {
        Keyboard {
            id: String::new(),
            content: Some(KeyboardContent { rows: self.rows }),
        }
    }
}

impl Button {
    /// A button with the given visible label.
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            render_data: Some(RenderData {
                label: label.into(),
                visited_label: String::new(),
                style: 0,
            }),
            ..Default::default()
        }
    }

    /// Unique button id inside one keyboard.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// 0 grey 1 blue 3 white-red 4 blue-white
    pub fn style(mut self, s: i32) -> Self {
        self.render_data_mut().style = s;
        self
    }

    pub fn visited_label(mut self, s: impl Into<String>) -> Self {
        self.render_data_mut().visited_label = s.into();
        self
    }

    /// Same-group buttons grey out the others after one click (action type 1).
    pub fn group(mut self, id: impl Into<String>) -> Self {
        self.group_id = id.into();
        self
    }

    /// Open a URL or miniapp (action type 0).
    pub fn jump(self, data: impl Into<String>) -> Self {
        self.act(0, data)
    }

    /// Post button data back to the bot backend (action type 1).
    pub fn callback(self, data: impl Into<String>) -> Self {
        self.act(1, data)
    }

    /// Insert "@bot data" into the input box (action type 2).
    pub fn command(self, data: impl Into<String>) -> Self {
        self.act(2, data)
    }

    /// Who may click: 0=specified users 1=admin 2=everyone.
    pub fn perm(mut self, perm_type: i32, user_ids: impl IntoIterator<Item = String>) -> Self {
        self.action_mut().permission = Some(Permission {
            perm_type,
            specify_user_ids: user_ids.into_iter().collect(),
            specify_role_ids: Vec::new(),
        });
        self
    }

    /// Command button auto-send (c2c, qq>=8983).
    pub fn enter(mut self, v: bool) -> Self {
        self.action_mut().enter = v;
        self
    }

    /// Command with quote.
    pub fn quote_reply(mut self, v: bool) -> Self {
        self.action_mut().reply = v;
        self
    }

    /// Modal second-confirmation prompt.
    pub fn confirm(
        mut self,
        content: impl Into<String>,
        confirm: impl Into<String>,
        cancel: impl Into<String>,
    ) -> Self {
        self.action_mut().modal = Some(Modal {
            content: content.into(),
            confirm_text: confirm.into(),
            cancel_text: cancel.into(),
        });
        self
    }

    fn act(mut self, action_type: i32, data: impl Into<String>) -> Self {
        let a = self.action_mut();
        a.action_type = action_type;
        a.data = data.into();
        self
    }

    fn render_data_mut(&mut self) -> &mut RenderData {
        self.render_data.get_or_insert_with(|| RenderData {
            label: String::new(),
            visited_label: String::new(),
            style: 0,
        })
    }

    fn action_mut(&mut self) -> &mut Action {
        self.action.get_or_insert_with(Action::default)
    }
}
