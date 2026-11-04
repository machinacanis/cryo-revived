# 02 REST API 端点总表

Base URL：`https://api.bot.qq.com`
通用鉴权头：`Authorization: QQBot {ACCESS_TOKEN}`
通用响应：成功直接返回业务数据；失败返回 `{"err_code": n, "message": "...", "trace_id": "..."}`。
`{xxx}` 为路径参数。完整字段细节见官方文档：<https://bot.q.qq.com/wiki/develop/api-v2/>。

## 1. 凭证与网关

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| POST | `https://api.bot.qq.com/app/getAppAccessToken` | 获取 access_token（完整 URL，非前缀拼接） | - |
| GET | `/gateway` | 获取通用 WSS 接入点 `{"url": "wss://..."}` | 2 QPM（10 QPM burst） |
| GET | `/gateway/bot` | 获取带分片 WSS 接入点，返回 `url/shards/session_start_limit` | - |

`/gateway/bot` 响应：

```json
{"url": "wss://api.bot.qq.com/websocket", "shards": 1,
 "session_start_limit": {"total": 1000, "remaining": 1000, "reset_after": 86400000, "max_concurrency": 1}}
```

## 2. 机器人与工具

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| GET | `/users/@me` | 获取机器人自身详情（User 对象） | 50 QPS |
| GET | `/users/@me/guilds` | 获取机器人所在频道列表 | 50 QPS |
| POST | `/v2/generate_url_link` | 生成分享链接；请求体 `{"callback_data": "..."}` | 50 QPS |
| PUT | `/interactions/{interaction_id}` | 互动事件响应，见 §7 | 50 QPS |

## 3. 单聊（C2C）消息与文件

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| POST | `/v2/users/{user_openid}/messages` | 发送单聊消息（msg_type 0/2/6/7/8） | 100 QPS |
| DELETE | `/v2/users/{user_openid}/messages/{message_id}` | 撤回单聊消息（发出 2 分钟内） | 10 QPS |
| POST | `/v2/users/{user_openid}/stream_messages` | 流式发送单聊消息（markdown 分片） | 50 QPS |
| POST | `/v2/users/{user_openid}/files` | 单聊富媒体上传 → `file_info` | 50 QPS |
| POST | `/v2/users/{user_id}/upload_prepare` | 分片上传预申请 → `upload_id`+预签名 URL | 10 QPS |
| POST | `/v2/users/{user_id}/upload_part_finish` | 分片上传完成通知 | 10 QPS |

`/v2/users/{user_openid}/files` 请求体：

| 字段 | 类型 | 说明 |
|---|---|---|
| file_type | int | 1=图片(png/jpg) 2=视频(mp4) 3=语音(silk) 4=文件；软限 20/30/20/200MB，硬限 200MB，超软限降级为文件 |
| url | string | http(s) URL 上传（平台下载转存）；分片合并时可为空 |
| srv_send_msg | bool | true=直接发送（占主动消息频次，返回消息 ID）；false=仅返回 file_info |
| upload_id | string | 分片上传任务 ID（走合并路径时传） |
| file_name | string | 文件名（分片/二进制上传必填） |
| file_data | string | 二进制直传数据（≤5MB，超出用分片） |

消息发送请求体详见 [03-messaging.md](03-messaging.md)。

## 4. 群聊消息与文件

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| POST | `/v2/groups/{group_openid}/messages` | 发送群消息（msg_type 0/2/7/8，不支持流式） | 100 QPS |
| DELETE | `/v2/groups/{group_openid}/messages/{message_id}` | 撤回群消息 | 10 QPS |
| POST | `/v2/groups/{group_openid}/files` | 群聊富媒体上传 → `file_info`（与单聊不互通） | 50 QPS |
| POST | `/v2/groups/{group_id}/upload_prepare` | 分片上传预申请 | 10 QPS |
| POST | `/v2/groups/{group_id}/upload_part_finish` | 分片上传完成通知 | 10 QPS |

分片上传流程：`upload_prepare`（传 file_type/file_size/file_name/md5/sha1/md5_10m）→ 按 `block_size`（默认 5MB）分片 PUT 到各 `presigned_url` → 每片完成调 `upload_part_finish` → 带 `upload_id` 调 `files` 合并得 `file_info`。`upload_config` 含 concurrency/retry_timeout/retry_delay。

## 5. 群管理（/v2/groups）

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| GET | `/v2/groups/{group_openid}/info` | 群信息：group_name/group_finger_memo/group_class_text/group_tags/group_member_num | 30 QPM |
| GET | `/v2/groups/{group_openid}/bot_state` | 机器人在群内状态 | 30 QPM |
| GET | `/v2/groups/{group_openid}/members` | 成员列表；请求体 `{"cursor": ""}` 分页，响应含 `next_cursor` | 60 QPM |
| GET | `/v2/groups/{group_openid}/members/{member_openid}` | 成员详情 | 30 QPM |
| POST | `/v2/groups/{group_openid}/batch_remove_members` | 批量踢人。体：`member_openids[]`（≤20）、`add_to_member_blacklist` | 30 QPM |
| GET | `/v2/groups/{group_openid}/member_blacklist` | 黑名单查询 | 30 QPM |
| POST | `/v2/groups/{group_openid}/member_blacklist` | 黑名单增删。体：`op`（add/del）、`member_openids[]`（≤20） | 60 QPM |
| GET | `/v2/groups/{group_openid}/restrict_chat_setting` | 查询禁言状态 | 30 QPM |
| POST | `/v2/groups/{group_openid}/restrict_chat_setting` | 成员禁言。体：`members[]`={op:add/update/del, member_openid, mute_expire_at(RFC3339)}，≤20/次；只能禁言普通成员 | 60 QPM |
| GET | `/v2/groups/{group_openid}/join_request_list` | 入群申请列表 | 30 QPM |
| POST | `/v2/groups/{group_openid}/approval_join_request/{member_openid}` | 入群审批。体：`op`(approve/decline)、`join_request_id`、`reject_reason`、`add_to_member_blacklist` | 60 QPM |

### 5.1 入群自动审批策略

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| GET | `/v2/groups/join_approval_strategy` | 策略列表；体 `{"cursor","limit"}`（limit ≤50 默认 20） | 60 QPM |
| POST | `/v2/groups/join_approval_strategy` | 创建策略 | 60 QPM |
| PATCH | `/v2/groups/join_approval_strategy/{strategy_id}` | 修改策略 | 60 QPM |
| DELETE | `/v2/groups/join_approval_strategy/{strategy_id}` | 删除策略 | 60 QPM |
| POST | `/v2/groups/join_approval_strategy/{strategy_id}/execute` | 手动执行策略 | 60 QPM |
| POST | `/v2/groups/join_approval_strategy/{strategy_id}/whitelist_users` | 修改策略白名单 | 60 QPM |

创建策略请求体：`group_openids[]` 与 `group_ids[]`（uint64 群号）二选一（≤100）、`is_enable`（on/off 默认 on）、`expire_at`（RFC3339，默认一年）、`remark`（≤255 汉字）。

## 6. 频道域（Guild / Channel）

> 频道为早期接口风格，返回对象模型见 [05-data-models.md](05-data-models.md)。

### 6.1 频道与子频道

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/guilds/{guild_id}` | 获取频道详情（Guild） |
| GET | `/guilds/{guild_id}/channels` | 子频道列表 |
| POST | `/guilds/{guild_id}/channels` | 创建子频道。体：name/type/sub_type/position/parent_id/private_type/private_user_ids/speak_permission/application_id |
| GET | `/channels/{channel_id}` | 子频道详情 |
| PATCH | `/channels/{channel_id}` | 修改子频道（同创建字段） |
| DELETE | `/channels/{channel_id}` | 删除子频道 |

### 6.2 频道成员与身份组

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/guilds/{guild_id}/members` | 成员列表（`?limit=&after=` 分页） |
| GET | `/guilds/{guild_id}/members/{user_id}` | 成员详情（Member） |
| DELETE | `/guilds/{guild_id}/members/{user_id}` | 移除成员（可加黑名单参数） |
| GET | `/guilds/{guild_id}/roles/{role_id}/members` | 身份组成员列表 |
| GET | `/guilds/{guild_id}/roles` | 身份组列表 |
| POST | `/guilds/{guild_id}/roles` | 创建身份组（name/color/hoist） |
| PATCH | `/guilds/{guild_id}/roles/{role_id}` | 修改身份组 |
| DELETE | `/guilds/{guild_id}/roles/{role_id}` | 删除身份组 |
| PUT | `/guilds/{guild_id}/members/{user_id}/roles/{role_id}` | 授予成员身份组 |
| DELETE | `/guilds/{guild_id}/members/{user_id}/roles/{role_id}` | 收回成员身份组 |
| GET | `/channels/{channel_id}/members/{user_id}/permissions` | 成员子频道权限 |
| PUT | `/channels/{channel_id}/members/{user_id}/permissions` | 修改成员子频道权限（add/remove permission 位串） |
| GET | `/channels/{channel_id}/roles/{role_id}/permissions` | 身份组子频道权限 |
| PUT | `/channels/{channel_id}/roles/{role_id}/permissions` | 修改身份组子频道权限 |
| GET | `/channels/{channel_id}/online_nums` | 在线人数（音频/直播子频道） |
| GET | `/guilds/{guild_id}/api_permission` | 查询频道内 API 权限 |
| POST | `/guilds/{guild_id}/api_permission/demand` | 发起接口权限申请 |
| GET | `/guilds/{guild_id}/message/setting` | 消息推送设置 |

### 6.3 频道内容

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/channels/{channel_id}/announces` | 发子频道公告（message_id） |
| DELETE | `/channels/{channel_id}/announces/{message_id}` | 删子频道公告 |
| POST | `/guilds/{guild_id}/announces` | 发频道公告 |
| DELETE | `/guilds/{guild_id}/announces/{message_id}` | 删频道公告 |
| GET | `/channels/{channel_id}/pins` | 精华消息列表 |
| PUT | `/channels/{channel_id}/pins/{message_id}` | 加精华 |
| DELETE | `/channels/{channel_id}/pins/{message_id}` | 取消精华 |
| GET | `/channels/{channel_id}/schedules` | 日程列表 |
| POST | `/channels/{channel_id}/schedules` | 创建日程（Schedule：name/start_timestamp/end_timestamp/creator/remind_timestamp…） |
| GET/PATCH/DELETE | `/channels/{channel_id}/schedules/{schedule_id}` | 查/改/删日程 |
| GET | `/channels/{channel_id}/threads` | 论坛主题列表 |
| PUT | `/channels/{channel_id}/threads` | 发表主题（Thread：title/content/format） |
| GET | `/channels/{channel_id}/threads/{thread_id}` | 主题详情 |
| DELETE | `/channels/{channel_id}/threads/{thread_id}` | 删除主题 |
| POST | `/channels/{channel_id}/audio` | 音频播放控制（audio_url/text/status） |
| PUT | `/channels/{channel_id}/mic` | 上麦 |
| DELETE | `/channels/{channel_id}/mic` | 下麦 |
| PATCH | `/guilds/{guild_id}/mute` | 全员/多成员禁言（mute_seconds/mute_end_timestamp、user_ids） |
| PATCH | `/guilds/{guild_id}/members/{user_id}/mute` | 单成员禁言 |

### 6.4 频道消息

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/channels/{channel_id}/messages` | 发子频道消息。体字段：content/embed/ark/markdown/image/file_image（multipart）、msg_id/event_id、message_reference；**与 v2 接口不同：无 msg_type**，至少一个内容字段 |
| DELETE | `/channels/{channel_id}/messages/{message_id}?hidetip=false` | 撤回子频道消息 |
| POST | `/users/@me/dms` | 创建频道私信会话 → `guild_id`+`channel_id` |
| POST | `/dms/{guild_id}/messages` | 发频道私信 |
| DELETE | `/dms/{guild_id}/messages/{message_id}?hidetip=false` | 撤回频道私信 |
| PUT | `/channels/{channel_id}/messages/{message_id}/reactions/{type}/{id}` | 表情表态（type: 1=emoji 2=表情） |
| DELETE | `/channels/{channel_id}/messages/{message_id}/reactions/{type}/{id}` | 取消表态（`?type=&cookie=&ids=`） |
| GET | `/channels/{channel_id}/messages/{message_id}/reactions/{type}/{id}?cookie=&limit=` | 表态列表 |

频道 content 支持内嵌格式：`<@!user_id>` 提及、`<#channel_id>` 子频道、`@everyone`（需权限）等。

## 7. 互动响应

| 方法 | 路径 | 说明 |
|---|---|---|
| PUT | `/interactions/{interaction_id}` | 回应 INTERACTION_CREATE。仅 type=11（消息按钮）、type=12（快捷菜单）必须回应，否则客户端一直 loading；同一 interaction_id 只能回应一次 |

请求体：`{"code": 0}` — 0=成功 1=操作失败 2=操作频繁 3=重复操作 4=没有权限 5=仅管理员操作。
`interaction_id` 取事件 `d.id`（不带 `INTERACTION_CREATE:` 前缀）。

## 8. 菜单与指令面板（/v2）

| 方法 | 路径 | 说明 | 限频 |
|---|---|---|---|
| GET | `/v2/menu` | 查询全局自定义菜单（仅单聊场景，全局生效） | 30 QPM |
| PUT | `/v2/menu` | 覆盖式修改全局菜单 | 5 QPM |
| GET | `/v2/panels` | 指令面板列表 | 30 QPM |
| POST | `/v2/panels` | 创建指令面板（每机器人 ≤20 个） | 10 QPM |
| GET | `/v2/panels/{panel_id}` | 面板详情 | 30 QPM |
| PUT | `/v2/panels/{panel_id}` | 修改面板 | 10 QPM |
| DELETE | `/v2/panels/{panel_id}` | 删除面板 | 10 QPM |
| PUT | `/v2/panels/{panel_id}/target` | 增删关联对象：`op`(add/del)、`user_openids[]`/`group_openids[]`（≤20） | 60 QPM |

### Menu 结构

`menu.items[]`（≤10）每项：`name`（≤10 字符，中文算 2）、`type`（switch/send_message/link/menu）、
`send_message`、`link`（必须 https）、`sub_menu_items[]`（≤5，仅 type=menu）、`switch`={switch_id, default}。
SubMenuItem：`name`（≤14 字符）、`type`（仅 send_message/link）、`send_message`、`link`。

### Panel 结构

创建体：`scope`（必填：c2c/group/channel/dm；channel/dm 仅 target_type=all）、`target_type`（all/specific，specific 仅 c2c/group）、`user_openids[]`/`group_openids[]`（≤20）、`panel`（必填）。
`panel`：`items[]`（≤20）每项 `name`（≤14 字符）/`desc`（≤30 字符）/`type`（command|link）/`only_admin`/`link`；`remark`（≤255）；`version`。
