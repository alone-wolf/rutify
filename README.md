# rutify

通知接收与推送服务：通过 HTTP 接收 notification，并通过 WebSocket 广播给所有客户端。

## 运行

```bash
cargo run
```

默认监听 `0.0.0.0:3000`，使用本地 sqlite 数据库 `rutify.db`。

### 环境变量

- `RUTIFY_ADDR`：监听地址，默认 `0.0.0.0:3000`
- `RUTIFY_DB_URL`：数据库连接串，默认 `sqlite://rutify.db?mode=rwc`

示例：

```bash
RUTIFY_ADDR=127.0.0.1:4000 \
RUTIFY_DB_URL=sqlite://./data/rutify.db?mode=rwc \
cargo run
```

## HTTP 接口

### POST /notify

请求体：

```json
{
  "notify": "content",
  "title": "default title",
  "device": "default device"
}
```

字段说明：
- `notify`：必填
- `title`：可选，默认 `default title`
- `device`：可选，默认 `default device`

响应：

```json
{ "status": "ok" }
```

示例：

```bash
curl -X POST http://127.0.0.1:3000/notify \
  -H 'Content-Type: application/json' \
  -d '{"notify":"hello","title":"hi","device":"mac"}'
```

## WebSocket

### GET /ws

客户端连接 `/ws` 后，会接收广播消息：

```json
{
  "event": "notify",
  "data": {
    "notify": "...",
    "title": "...",
    "device": "..."
  }
}
```

## 数据库

表 `notifications`：
- `notify`
- `title`
- `device`
- `received_at`

服务启动时自动创建表（如不存在）。
