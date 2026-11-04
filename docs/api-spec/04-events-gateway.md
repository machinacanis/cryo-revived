# 04 事件推送与网关

## 1. Payload 通用结构（webhook/websocket 一致）

```json
{"id": "event_id", "op": 0, "d": {}, "s": 42, "t": "GATEWAY_EVENT_NAME"}
```

| 字段 | 说明 |
|---|---|
| id | 事件 ID（被动回复时用 `event_id` 取此值） |
| op | opcode，见下表 |
| s | 下行序列号，心跳/Resume 时回传最新值 |
| t | 事件类型名（op=0 Dispatch 时有效） |
| d | 事件体，随 t 变化 |

## 2. Opcode

| code | 名称 | 接入方式 | 客户端行为 | 说明 |
|---|---|---|---|---|
| 0 | Dispatch | webhook/websocket | Receive | 服务端推送 |
| 1 | Heartbeat | websocket | Send/Receive | 心跳 |
| 2 | Identify | websocket | Send | 登录鉴权 |
| 6 | Resume | websocket | Send | 恢复连接 |
| 7 | Reconnect | websocket | Receive | 服务端要求重连 |
| 9 | Invalid Session | websocket | Receive | identify/resume 参数错误 |
| 10 | Hello | websocket | Receive | 连接后首条，含 heartbeat_interval(ms) |
| 11 | Heartbeat ACK | websocket | Receive/Reply | 心跳应答 |
| 12 | HTTP Callback ACK | webhook | Reply | 回调模式回包（收到事件） |
| 13 | 回调地址验证 | webhook | Receive | 平台验证回调地址 |

## 3. Intents 订阅位（Identify.d.intents，位或）

| 位 | 名称 | 事件（t 值） | 权限 |
|---|---|---|---|
| 1<<0 | GUILDS | GUILD_CREATE/UPDATE/DELETE、CHANNEL_CREATE/UPDATE/DELETE | 默认 |
| 1<<1 | GUILD_MEMBERS | GUILD_MEMBER_ADD/UPDATE/REMOVE | 默认 |
| 1<<9 | GUILD_MESSAGES | MESSAGE_CREATE、MESSAGE_DELETE | 私域申请 |
| 1<<10 | GUILD_MESSAGE_REACTIONS | MESSAGE_REACTION_ADD/REMOVE | 申请 |
| 1<<12 | DIRECT_MESSAGE | DIRECT_MESSAGE_CREATE/DELETE | 申请 |
| 1<<24 | GROUP_MEMBER_EVENT | GROUP_JOIN_REQUEST、GROUP_MEMBER_ADD、GROUP_MEMBER_REMOVE | 申请 |
| 1<<25 | GROUP_AND_C2C_EVENT | C2C_MESSAGE_CREATE、FRIEND_ADD/DEL、C2C_MSG_REJECT/RECEIVE、GROUP_AT_MESSAGE_CREATE、GROUP_MESSAGE_CREATE（全量）、GROUP_ADD_ROBOT、GROUP_DEL_ROBOT、GROUP_MSG_REJECT/RECEIVE、SUBSCRIBE_MESSAGE_STATUS | 默认 |
| 1<<26 | INTERACTION | INTERACTION_CREATE | 默认 |
| 1<<27 | MESSAGE_AUDIT | MESSAGE_AUDIT_PASS/REJECT（主动消息审核结果，MessageAudited） | 申请 |
| 1<<28 | FORUMS_EVENT | FORUM_THREAD/POST/REPLY_CREATE/DELETE、FORUM_PUBLISH_AUDIT_RESULT | 私域申请 |
| 1<<29 | AUDIO_ACTION | AUDIO_START/FINISH/ON_MIC/OFF_MIC | 申请 |
| 1<<30 | PUBLIC_GUILD_MESSAGES | AT_MESSAGE_CREATE、PUBLIC_MESSAGE_DELETE | 默认 |

计算：`intents = (1<<25) | (1<<26)` 订阅群聊单聊事件 + 互动。传了无权限的位 → WS 报错关连接。

## 4. 事件清单与 d 要点

### 4.1 单聊/群聊（GROUP_AND_C2C_EVENT 1<<25）

| t | 说明 | d 要点 |
|---|---|---|
| C2C_MESSAGE_CREATE | 单聊消息 | id, author(user_openid), content, timestamp, message_type(0/3/101/102/103), message_scene, attachments, ark_data, msg_elements |
| FRIEND_ADD / FRIEND_DEL | 添加/删除机器人 | openid, timestamp |
| C2C_MSG_REJECT / C2C_MSG_RECEIVE | 用户关/开"主动消息" | openid, timestamp |
| GROUP_AT_MESSAGE_CREATE | 群内@机器人 | 同消息结构 + group_openid、author.member_openid、member_role |
| GROUP_MESSAGE_CREATE | 群消息全量模式 | 同上 |
| GROUP_ADD_ROBOT / GROUP_DEL_ROBOT | 机器人进群/被移出 | group_openid, op_member_openid, timestamp |
| GROUP_MSG_REJECT / GROUP_MSG_RECEIVE | 群管理员关/开通知 | group_openid, op_member_openid, timestamp |
| SUBSCRIBE_MESSAGE_STATUS | 订阅消息授权变更 | 授权状态信息 |

消息对象（C2C/Group 共用）字段：`id`、`author`（User：id/username/bot/union_openid/union_user_account/user_openid/member_openid/member_role）、`content`、`timestamp`（RFC3339）、`message_type`、`message_scene`（source、ext[] 含 msg_idx/ref_msg_idx/auth_token）、`attachments[]`（url/filename/width/height/size/content_type/voice_wav_url/asr_refer_text）、`ark_data`（prompt/ark_type[ tuwen|feed|miniapp|map|contact_card|video_share|music_together|picture ]/ark_name/fields）、`msg_elements`（msg_idx/author/message_type，引用消息嵌套）。

### 4.2 群成员（GROUP_MEMBER_EVENT 1<<24）

| t | 说明 |
|---|---|
| GROUP_JOIN_REQUEST | 用户申请加群 |
| GROUP_MEMBER_ADD | 成员加入 |
| GROUP_MEMBER_REMOVE | 成员退出 |

### 4.3 频道（GUILDS 1<<0 等）

| t | 说明 |
|---|---|
| GUILD_CREATE / GUILD_UPDATE / GUILD_DELETE | 频道创建/变更/解散（d=Guild） |
| CHANNEL_CREATE / CHANNEL_UPDATE / CHANNEL_DELETE | 子频道创建/更新/删除（d=Channel） |
| AT_MESSAGE_CREATE | 频道内@机器人（d=Message：id/channel_id/guild_id/content/author/member/mentions/attachments…） |
| MESSAGE_CREATE / MESSAGE_DELETE | 全量频道消息（私域） |
| DIRECT_MESSAGE_CREATE | 频道私信 |
| MESSAGE_REACTION_ADD / REMOVE | 表情表态 |
| MESSAGE_AUDIT_PASS / REJECT | 主动消息审核（d=MessageAudited：audit_id/message_id/channel_id/guild_id/audit_time…） |
| FORUM_* | 论坛主题/帖子/回复事件 |
| AUDIO_* | 音频/上下麦 |

### 4.4 互动（INTERACTION 1<<26）

| t | 说明 |
|---|---|
| INTERACTION_CREATE | 互动事件（按钮点击、菜单等）。d.id 为 interaction_id；type=11/12 需调 `PUT /interactions/{id}` 回应 |

## 5. WebSocket 生命周期

```
GET /gateway(/bot) → 返回 wss 地址（实测为 wss://api.sgroup.qq.com/websocket，**不要硬编码域名，始终用接口返回值**）
  ↓ 连接建立
[10 Hello] {d:{heartbeat_interval:45000}}
  ↓ 发送
[2 Identify] {d:{token:"QQBot {ACCESS_TOKEN}", intents:N, shard:[0,1], properties:{...}}}
  ↓ 成功
[0 READY] {s:1, t:"READY", d:{version, session_id, user:{id,username,bot}, shard}}
  ↓ 周期发送（d=最后收到的 s，首连为 null）
[1 Heartbeat] {"op":1,"d":251} → [11 Heartbeat ACK]
```

### 5.1 断线恢复（Resume）

- 重连后**不要**重发 Identify，发：

```json
{"op": 6, "d": {"token": "QQBot {ACCESS_TOKEN}", "session_id": "保存的", "seq": 最后s}}
```

- 成功 → 补发遗漏事件 → `[0 RESUMED]`。务必在每次处理事件后记录 `s`。

### 5.2 分片（Sharding）

- Identify 传 `shard: [shard_id, num_shards]`，需为每个 shard_id 各建一条连接。
- `GET /gateway/bot` 返回建议 `shards` 与 `session_start_limit`（连接数 ≤ remaining）。
- 分片路由：`shard_id = (guild_id >> 22) % num_shards`，同频道事件固定同连接。

### 5.3 关闭码

| 值 | 含义 | 可 Resume | 可 Identify |
|---|---|---|---|
| 4001 | 无效 opcode | 否 | 否 |
| 4002 | 无效 payload | 否 | 否 |
| 4006 | 无效 session id | 否 | ✅ |
| 4007 | seq 错误 | 否 | ✅ |
| 4008 | 发送过快 | ✅ | ✅ |
| 4009 | 连接过期 | ✅ | ✅ |
| 4010 | 无效 shard | 否 | 否 |
| 4011 | guild 过多需分片 | 否 | 否 |
| 4012 | 无效 version | 否 | 否 |
| 4013 | 无效 intent | 否 | 否 |
| 4014 | intent 无权限 | 否 | 否 |
| 4900~4913 | 内部错误 | 否 | ✅ |
| 4914 | 机器人下架，仅沙箱可连 | 否 | 否 |
| 4915 | 机器人封禁 | 否 | 否 |

处理策略：4009 → Resume；4914/4915 → 停连排查；其余 → 重连 Identify。

## 6. Webhook（HTTP 回调）

- 回调地址在开放平台管理端配置，仅允许端口 **80/443/8080/8443**，需 HTTPS。
- 每个回调请求须做 Ed25519 验签（见 [01-authentication.md](01-authentication.md)）。
- **op=13 地址验证**：收到 `{"op":13,"d":{"plain_token","event_ts"}}`，回 `{"plain_token":<原样>,"signature":hex(ed25519.Sign(event_ts+plain_token))}`。
- **op=0 事件**：验签后处理，HTTP 200 回包即视为 ACK（opcode 12 的语义）。
- 事件体与 WebSocket 完全一致（同 Payload 结构）。
