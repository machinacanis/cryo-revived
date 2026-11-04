# 05 公共数据模型

## 1. User（用户）

| 字段 | 类型 | 说明 |
|---|---|---|
| id | string | 用户 id（v2 场景为 openid） |
| username | string | 用户名/昵称 |
| avatar | string | 头像 URL |
| bot | bool | 是否机器人 |
| union_openid | string | 跨应用统一 openid（需申请，仅单独拉取 member 时返回） |
| union_user_account | string | 跨应用统一账号（同上） |
| user_openid | string | 单聊 openid（v2 事件） |
| member_openid | string | 群成员 openid（v2 群事件） |
| member_role | string | member / admin / owner（群内角色） |

## 2. Guild（频道）

| 字段 | 类型 | 说明 |
|---|---|---|
| id | string | 频道 ID |
| name | string | 名称 |
| icon | string | 头像 URL |
| owner_id | string | 创建人 ID |
| owner | bool | 当前用户是否创建人 |
| member_count | int | 成员数 |
| max_members | int | 最大成员数 |
| description | string | 描述 |
| joined_at | string | 加入时间 |

## 3. Channel（子频道）

| 字段 | 类型 | 说明 |
|---|---|---|
| id / guild_id / name | string | 基础字段 |
| type | int | ChannelType |
| sub_type | int | ChannelSubType |
| position | int | 排序（分组类型 ≥2；填已存在值则插入其前） |
| parent_id | string | 所属分组 id |
| owner_id | string | 创建人 |
| private_type | int | PrivateType |
| speak_permission | int | SpeakPermission |
| application_id | string | 应用子频道的应用类型 |
| permissions | string | 用户子频道权限位串 |

### ChannelType

`0` 文字 / `2` 语音 / `4` 分组 / `10005` 直播 / `10006` 应用 / `10007` 论坛（1、3 保留；需容忍未知值）

### ChannelSubType（仅文字子频道）

`0` 闲聊 / `1` 公告 / `2` 攻略 / `3` 开黑（需容忍未知值）

### PrivateType / SpeakPermission

PrivateType：`0` 公开 / `1` 群主管理员可见 / `2` 群主管理员+指定成员
SpeakPermission：`0` 无效 / `1` 所有人 / `2` 群主管理员+指定成员

## 4. Member（频道成员）

| 字段 | 类型 | 说明 |
|---|---|---|
| user | User | 用户信息（仅成员相关接口填充） |
| nick | string | 频道昵称 |
| roles | []string | 身份组 ID（默认身份组见 DefaultRoles） |
| joined_at | ISO8601 | 加入时间 |
| guild_id | string | （MemberWithGuildID 变体）频道 id |

## 5. Message（频道消息）

频道消息事件（AT_MESSAGE_CREATE 等）的 `d`：

| 字段 | 说明 |
|---|---|
| id / channel_id / guild_id | 消息与位置 |
| content | 文本（含 `<@!id>` `<#id>` 内嵌格式） |
| author / member | 发送者与频道成员信息 |
| mentions / mentions_everyone | @对象列表 / 是否@全体 |
| timestamp / edited_timestamp | 时间 |
| attachments | 附件 |
| embeds | Embed 列表 |
| ark | Ark 对象 |
| seq / seq_in_channel | 排序 |
| message_reference | 引用 |

## 6. MessageEmbed / MessageArk（频道）

- Embed 发送结构：`{title, prompt, thumbnail:{url}, fields:[{name}]}` 等（旧式频道 embed，v2 不支持）。
- Ark：`{template_id, kv:[{key, value, obj:[{obj_kv:[]}]}]}`，依赖模板。

## 7. Schedule / Announces / Pins / Forum

- Schedule：`{id, name, description, start_timestamp, end_timestamp, creator(Member), jump_channel_id, remind_type}`。
- 论坛 Thread：`{guild_id, channel_id, author_id, thread_info:{thread_id,title,content,date_time}}`；发表体 `{title, content, format}`。

## 8. 表情表态 EmojiType

`type=1` 系统表情（id=表情 id），`type=2` emoji（id=unicode）。表态对象 `{id, type, name, url, animate, tip, list[]}`。

## 9. 群聊 v2 专用结构

- 群信息：`{group_openid, group_name, group_finger_memo, group_class_text, group_tags[], group_member_num}`。
- 成员：member_openid + 角色/昵称/入群时间（详见 autogen 镜像文档）。
- MessageScene：`{source:"default", ext:["msg_idx=…","ref_msg_idx=…","auth_token=…"]}`。

## 10. 序列化约定（SDK 实现注意）

- 所有 `*_openid`、`guild_id`、`channel_id` 一律按 `string` 处理（部分 id 数值超 int64 安全范围）。
- 时间字段为 RFC3339/ISO8601 字符串，反序列化用 `time.Time` + 自定义 Unmarshal。
- 枚举（ChannelType/message_type/action.type 等）必须容忍未知值，避免 decode 失败。
- 分页统一为游标模式（`cursor` → `next_cursor`）或 `after`/`limit`（频道旧接口）。
