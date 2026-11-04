# 01 认证与签名

## 1. access_token（调用 OpenAPI 的凭证）

### 1.1 获取

| 字段 | 值 |
|---|---|
| URL | `POST https://api.bot.qq.com/app/getAppAccessToken`（注意：是完整 URL，不带统一前缀之外的路径规则） |
| Content-Type | `application/json` |

请求体：

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| appId | string | 是 | 开放平台管理端获取 |
| clientSecret | string | 是 | 开放平台管理端获取 |

成功响应：

| 字段 | 类型 | 说明 |
|---|---|---|
| access_token | string | 凭证 |
| expires_in | number | 有效期（秒），目前 ≤ 7200 |

失败响应（HTTP 仍为 200，必须看 `code`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| code | number | 业务错误码 |
| message | string | 仅供人工排查，勿用于判断逻辑 |

业务错误码：

| code | 含义 |
|---|---|
| 100001 | 请求过于频繁 |
| 100007 | AppID 无效或机器人被封禁/删除 |
| 100016 | AppID 或 clientSecret 不正确 |
| 10004 | 机器人不存在 |

### 1.2 有效期与刷新策略

- 默认生命周期 **7200 秒（2 小时）**，过期后需自行刷新。
- 有效期内重复获取返回**相同值**（不会刷新）。
- 剩余有效期 < **60 秒**时再获取会返回**新 token**；旧 token 在这 60 秒重叠窗口内仍可用。
- **SDK 实现要求**：进程内缓存 token + 定时刷新（建议到期前 5 分钟触发），不要在每次请求时才去取。

### 1.3 使用方式

```
Authorization: QQBot {ACCESS_TOKEN}
```

> 安全要求：凭证只能在服务端使用，禁止下发前端。

## 2. Webhook 签名（Ed25519）

仅 Webhook（HTTP 回调）模式需要。Bot Secret 同时用于：平台 → 机器人的请求验签、机器人对 op13 验证请求的签名应答。

### 2.1 密钥派生

用 Bot Secret 重复拼接至 ≥32 字节，取前 32 字节作为 Ed25519 seed，生成密钥对：

```go
seed := botSecret
for len(seed) < ed25519.SeedSize {
    seed = strings.Repeat(seed, 2)
}
publicKey, privateKey, _ := ed25519.GenerateKey(strings.NewReader(seed[:ed25519.SeedSize]))
```

### 2.2 验证平台请求签名

请求头：

| Header | 说明 |
|---|---|
| X-Signature-Ed25519 | 签名的 hex 编码（解码后 64 字节） |
| X-Signature-Timestamp | 签名时间戳 |
| X-Bot-Appid | 机器人 AppID |
| User-Agent | `QQBot-Callback` |

验证流程：

1. hex 解码 `X-Signature-Ed25519` → `sig`（长度必须 64，且 `sig[63]&224 == 0`）。
2. 拼接签名体：`msg = timestamp + rawBody`（时间戳字符串原文 + HTTP Body 原文）。
3. `ed25519.Verify(publicKey, msg, sig)`。

```go
signature := req.Header.Get("X-Signature-Ed25519")
sig, err := hex.DecodeString(signature)
if err != nil || len(sig) != ed25519.SignatureSize || sig[63]&224 != 0 {
    return false
}
timestamp := req.Header.Get("X-Signature-Timestamp")
body, _ := io.ReadAll(req.Body)
var msg bytes.Buffer
msg.WriteString(timestamp)
msg.Write(body)
ok := ed25519.Verify(publicKey, msg.Bytes(), sig)
```

### 2.3 回调地址验证应答（op=13）

平台验证回调地址时下发 `{"op":13, "d":{"plain_token":"...","event_ts":"..."}}`，
机器人需签名 `event_ts + plain_token` 并回包：

```
签名体 = event_ts + plain_token
signature = hex( ed25519.Sign(privateKey, 签名体) )
响应体 = {"plain_token": "<原样返回>", "signature": "<hex>"}
```

详见 [04-events-gateway.md](04-events-gateway.md) 的 Webhook 章节。
