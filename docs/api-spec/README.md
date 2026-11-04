# QQ 机器人开放平台 API Spec（api-v2）

> 依据 QQ 机器人官方开发文档整理（抓取于 2026-11-04），
> 作为 cryo QQ Bot 框架（Rust）的开发指导规范。
> 官方文档：<https://bot.q.qq.com/wiki/develop/api-v2/>

## 1. 平台组成

| 组成 | 协议 | 用途 |
|---|---|---|
| OpenAPI（REST） | HTTPS + JSON | 主动调用：发消息、管理群/频道、上传文件、菜单面板等 |
| 事件推送 | WebSocket 或 Webhook | 被动接收：消息事件、成员事件、互动事件等 |
| 回调验签 | Ed25519 | Webhook 模式下验证平台请求、响应回调地址验证 |

## 2. 核心概念

### 2.1 票据

| 名称 | 说明 |
|---|---|
| AppID | 机器人 ID，开放平台管理端获取 |
| AppSecret（clientSecret/Bot Secret） | 密钥：用于换取 access_token、生成/校验 Webhook 签名 |

Token 鉴权方式已废弃，统一使用 `access_token`。

### 2.2 三大聊天场景

| 场景 | 标识体系 | 发消息接口前缀 | 主要事件 |
|---|---|---|---|
| QQ 单聊（C2C） | `user_openid` | `/v2/users/{user_openid}/` | `C2C_MESSAGE_CREATE`、`FRIEND_ADD/DEL` |
| QQ 群聊 | `group_openid`、`member_openid` | `/v2/groups/{group_openid}/` | `GROUP_AT_MESSAGE_CREATE`、`GROUP_MESSAGE_CREATE`、`GROUP_ADD/DEL_ROBOT` |
| QQ 频道 | `guild_id`、`channel_id`、`user_id` | `/guilds/`、`/channels/` | `AT_MESSAGE_CREATE`、`GUILD_*`、`CHANNEL_*` |

> **注意**：不同 AppID 下同一用户/群的 openid 不同；单聊与群聊的文件上传接口不互通。
> 跨 AppID 打通身份需申请 `union_openid` 机制。

### 2.3 统一约定

- Base URL：`https://api.bot.qq.com`（WebSocket 地址由 `GET /gateway` 动态返回，实测为 `wss://api.sgroup.qq.com/websocket`，勿硬编码）
- 鉴权头：`Authorization: QQBot {ACCESS_TOKEN}`
- 请求体：`Content-Type: application/json; charset=utf-8`
- 错误判定：以响应 JSON 的 `err_code`/`code` 为准（HTTP 200 也可能是业务失败），`trace_id` + `X-Tps-trace-ID` 用于排查
- 时间格式：RFC3339（东八区）

## 3. 文档导航

| 文件 | 内容 |
|---|---|
| [01-authentication.md](01-authentication.md) | access_token 获取/刷新、Webhook Ed25519 签名与验证 |
| [02-rest-endpoints.md](02-rest-endpoints.md) | 全部 REST 端点清单（按域分组、含限频与请求体） |
| [03-messaging.md](03-messaging.md) | 消息收发模型：msg_type、Keyboard、Markdown、富媒体上传、流式消息、频控/时效/去重 |
| [04-events-gateway.md](04-events-gateway.md) | 事件推送：Payload 结构、Opcode、Intents、事件清单、WebSocket 生命周期、Webhook 回调 |
| [05-data-models.md](05-data-models.md) | User / Guild / Channel / Member / Message 等公共数据模型与枚举 |
| [06-error-codes.md](06-error-codes.md) | HTTP 状态码、公共错误码、消息错误码、WS 关闭码 |

## 4. cryo 实现现状（Rust）

```
src/api/       REST client：access_token 缓存与刷新、err_code/trace_id 解析、端点句柄
src/gateway/   WebSocket：Hello→Identify→Heartbeat→Resume 状态机、分片、重连退避
src/model/     消息/用户/频道等 wire 类型与出站消息 builder，字段与 JSON 一一对应
src/context.rs EventCtx + 各事件类型的 typed ctx（解码 payload.d）
src/extract.rs axum 式 FromEvent 提取器，handler 参数注入
src/handler.rs handler 类型擦除与注册
```

未实现：Webhook 回调（Ed25519 验签）、频道域大部分接口；其余端点见
`src/api/endpoints.rs`。
