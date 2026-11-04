# 03 消息收发模型（v2 单聊/群聊）

适用于 `/v2/users/{user_openid}/messages` 与 `/v2/groups/{group_openid}/messages`。
频道消息结构不同，见 [02-rest-endpoints.md](02-rest-endpoints.md) §6.4。

## 1. 消息分类

| 类型 | 特征 | 说明 |
|---|---|---|
| 主动消息 | 不带 msg_id/event_id | 受主动频控限制；用户在客户端关闭"允许主动发送"则必失败 |
| 被动消息（回复消息） | 带 `msg_id` | 对事件 `d.id` 所指消息的回复 |
| 被动消息（响应事件） | 带 `event_id` | 对事件最外层 `id` 的回复，与 msg_id 二选一；支持事件：INTERACTION_CREATE、C2C_MSG_RECEIVE、FRIEND_ADD（单聊）/ GROUP_ADD_ROBOT、GROUP_MSG_RECEIVE（群聊） |
| 互动召回消息 | `is_wakeup=true` | 用户主动对话后 30 天内，每周期可发 1 条：当天 / 1-3 天 / 3-7 天 / 7-30 天（仅单聊），与 msg_id/event_id 互斥 |

### 被动消息时效与次数

| 场景 | msg_id 有效期 | 每条消息可回复次数 |
|---|---|---|
| 单聊 | 60 分钟 | 4 |
| 群聊 | 5 分钟 | 5 |
| 频道 | 5 分钟 | - |

## 2. 发送侧 msg_type

| msg_type | 类型 | 生效字段 | 单聊 | 群聊 |
|---|---|---|---|---|
| 0 | 纯文本 | `content` | ✅ | ✅ |
| 2 | Markdown | `markdown` | ✅ | ✅ |
| 6 | 输入中状态 | `input_notify` | ✅ | ❌ |
| 7 | 富媒体 | `media` | ✅ | ✅ |
| 8 | 图文卡片 | `card` | ✅ | ✅ |

约束：填 `markdown` 后 `content`/`ark` 必须为空。

## 3. 接收侧 message_type（事件 d 中）

| message_type | 含义 | 字段 |
|---|---|---|
| 0 | 普通文本 | `content` |
| 3 | 结构化卡片 | `ark_data` |
| 101 | 并行消息 | - |
| 102 | 聊天记录 | - |
| 103 | 引用消息 | `msg_elements`（含被引用内容），`message_scene.ext` 含 `ref_msg_idx` |

图片/视频/语音/文件等附件走 `attachments[]`，由 `content_type` 区分（voice、image/jpeg、image/png、image/gif、video/mp4、file），语音另附 `voice_wav_url`/`asr_refer_text`。

## 4. 发送请求体（完整字段）

| 字段 | 类型 | 说明 |
|---|---|---|
| msg_type | int | 见上表，决定内容字段 |
| content | string | 文本（msg_type=0） |
| markdown | MessageMarkdown | msg_type=2 |
| keyboard | Keyboard | 内嵌键盘：`{id}` 用平台模板，或 `{content:{rows:[{buttons:[Button]}]}}` 自定义，互斥 |
| msg_id | string | 被动回复消息 ID（事件 d.id） |
| event_id | string | 被动回复事件 ID（事件最外层 id） |
| msg_seq | int | 回复序号，默认 1；同一 msg_id+msg_seq 重复发送会失败，递增可多次回复 |
| media | `{file_info: string}` | msg_type=7；file_info 来自 files 上传接口 |
| message_reference | `{message_id: string}` | 引用回复。非机器人消息取 `message_scene.ext` 的 `msg_idx`；机器人消息取发送响应 `ext_info.ref_idx` |
| is_wakeup | bool | 互动召回消息标记（仅单聊） |
| card | MessageCard | msg_type=8：`{type:"tuwen", content:{title(1-50), description(1-200), url(https ≤1024), pic_url(https ≤1024)}}` |
| input_notify | `{input_type:1, input_second}` | msg_type=6，状态持续 ≤60s（仅单聊） |

响应：`{id, timestamp, ext_info:{ref_idx}}`，`id` 可用于撤回。

## 5. Keyboard 对象树

```
Keyboard
├── id: string                    // 平台模板 ID（与 content 互斥）
└── content: { rows: [ Row ] }    // 自定义布局
    └── Row { buttons: [ Button ] }
        └── Button
            ├── id: string            // 键盘内唯一
            ├── group_id: string      // 同组按钮一个被点后其余变灰（仅 action.type=1）
            ├── render_data: { label(≤10字符), visited_label, style }   // style: 0灰线框 1蓝线框 3白底红字 4蓝底白字
            └── action
                ├── type: int         // 0=跳转(http/小程序) 1=回调(data给后台) 2=指令(填入@bot data)
                ├── permission: { type: 0指定用户|1管理员|2所有人, specify_user_ids[], specify_role_ids[](仅频道) }
                ├── data: string      // type=1/2 必填
                ├── unsupport_tips: string   // 低版本提示
                ├── enter: bool       // 指令按钮直接发送（仅单聊，QQ≥8983）
                ├── reply: bool       // 指令带引用回复
                ├── anchor: int       // =1 唤起选图器（覆盖 enter；仅单聊手机端 8983+）
                └── modal: { content(≤40,无URL), confirm_text(≤4), cancel_text(≤4) }  // 二次确认
```

## 6. MessageMarkdown

| 字段 | 说明 |
|---|---|
| content | Markdown 文本（`template_id`/`custom_template_id` 均已废弃） |
| force_verify_image_resource | true 时图片转存失败则整体报错（默认 false） |

## 7. 富媒体上传

发图/视频/语音/文件必须先上传拿 `file_info`，再以 `msg_type=7` + `media.file_info` 发送。
`file_info` 有时效（ttl），过期需重传。**单聊与群聊上传接口不互通。**

三种方式：

| 方式 | 用法 | 限制 |
|---|---|---|
| 分片上传（推荐） | upload_prepare → PUT 分片到 presigned_url → upload_part_finish → files(upload_id) | 大文件必选 |
| URL 上传 | files 传 `url` | 平台下载可能失败，失败可改本地下载后二进制直传 |
| 二进制直传 | files 传 `file_data`+`file_name` | ≤5MB |

files 响应（srv_send_msg=false）：`{file_info, ttl}`；srv_send_msg=true 时直接发出并返回消息 ID。

## 8. 流式消息（仅单聊）

`POST /v2/users/{user_openid}/stream_messages`，用于打字机式分批下发 markdown：

| 字段 | 说明 |
|---|---|
| input_mode | append（默认，增量追加 Pending）/ replace（content_raw 为全量，须以已下发前缀开头） |
| input_state | 1=生成中，10=生成结束（最后一片） |
| index | 分片序号，从 0 递增 |
| content_type | text / markdown |
| content_raw | 本片段内容 |
| stream_msg_id | 首片由服务端返回，后续分片携带 |
| msg_id / event_id / msg_seq / is_wakeup | 同普通发送 |

## 9. 主动消息频控

| 认证类型 | 单聊 Bot 维度 | 群 Bot 维度 | 单关系维度 | 日上限 |
|---|---|---|---|---|
| 企业/个人认证 | 10 qps | 60 qpm | 20 qpm | 1000 条/(用户\|群) |
| 未认证 | 5 qps 且 30 qpm | 30 qpm | 20 qpm | 1000 条/(用户\|群) |

频道侧：默认每子频道每天主动 20 条、每天限 2 个子频道（频道主可改）、每子频道 ≤5 条/秒；私信每用户每天 2 条、每天共 200 条。

## 10. 去重与撤回

- 同一 `msg_id` 事件可能重复推送，用 `msg_seq` / `message_scene.ext` 的 `msg_idx` 去重。
- 被动回复：同 `msg_id + msg_seq` 重复发送会失败（错误码 40054005），递增 `msg_seq` 复用。
- 撤回：仅能撤回机器人自己发出 ≤2 分钟的消息。
