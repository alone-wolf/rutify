# Rutify

Rutify 是一个基于 Rust workspace 的通知系统，包含服务端、SDK、CLI 和桌面应用。

## 当前可控范围

- 稳定核心：`rutify-server`、`rutify-core`、`rutify-sdk`、`rutify-client`、`rutify-cli`
- 实验中：`rutify-application`、`rutify-panel`（UI 仍在完善）

## Workspace 结构

```text
packages/
├── rutify-core          # 共享类型
├── rutify-sdk           # HTTP/WS 客户端 SDK
├── rutify-client        # SDK 上层共享客户端逻辑
├── rutify-server        # HTTP + WebSocket + SQLite 服务端
├── rutify-cli           # 命令行客户端
├── rutify-application   # 桌面应用（Slint）
└── rutify-panel         # 管理面板（Slint）
```

## 快速开始

### 1) 启动服务端

```bash
cargo run --package rutify-server
```

带 UI 启动：

```bash
cargo run --package rutify-server -- --ui
```

默认监听 `0.0.0.0:3000`，默认数据库 `sqlite://rutify.db?mode=rwc`。

### 2) 使用 CLI

```bash
cargo run --package rutify-cli -- --help
cargo run --package rutify-cli -- stats
cargo run --package rutify-cli -- notifies
```

## 环境变量

- `RUTIFY_ADDR`：服务监听地址，默认 `0.0.0.0:3000`
- `RUTIFY_DB_URL`：数据库地址，默认 `sqlite://rutify.db?mode=rwc`
- `RUTIFY_JWT_SECRET`：JWT 密钥（生产环境必须设置，至少 32 字符）

## 主要接口

- `GET /`：服务探活
- `GET /ws`：WebSocket（兼容入口）
- `POST /notify`：发送通知
- `GET /notify/ws?token=<notify_token>`：WebSocket 通知流
- `GET /api/notifies`：读取通知列表（真实数据库数据）
- `GET /api/stats`：读取统计（真实数据库数据）
- `POST /auth/register`：注册用户
- `POST /auth/login`：用户登录
- `GET/POST/DELETE /auth/tokens`：Token 管理（需要用户 JWT）

## 维护说明

- 根目录只保留入口文档；历史阶段性文档已清理。
- 服务端删除了未落地的空壳模块，当前目录结构与实际运行路径一致。
